use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use substream::{
    auth::{Token, create_token},
    config::RecognizerConfig,
    server::{ServerConfig, router},
};
use substream_core::audio::AudioChunk;
use substream_protocol::{ServerMessage, encode_audio};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::watch,
    task::JoinHandle,
    time::timeout,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct TestServer {
    url: String,
    token: String,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl TestServer {
    async fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token");
        create_token(&path).unwrap();
        let token = std::fs::read_to_string(&path).unwrap().trim().to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, receiver) = watch::channel(false);
        let app = router(
            ServerConfig {
                address,
                token: Token::read(&path).unwrap(),
                allowed_origins: vec!["chrome-extension://test".into()],
                recognizer: RecognizerConfig::Demo,
            },
            receiver,
        );
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            url: format!("ws://{address}/v1/stream"),
            token,
            shutdown,
            task,
        }
    }

    async fn connect(&self, token: &str) -> Socket {
        let (mut socket, _) = connect_async(&self.url).await.unwrap();
        socket
            .send(Message::Text(
                serde_json::json!({"type":"authenticate", "version":1, "token":token})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        socket
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        self.task.abort();
    }
}

async fn event(socket: &mut Socket) -> ServerMessage {
    let message = timeout(Duration::from_secs(3), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    serde_json::from_str(message.to_text().unwrap()).unwrap()
}

#[tokio::test]
async fn stopping_a_stream_delivers_final_captions_and_accounts_for_all_audio() {
    let server = TestServer::start().await;
    let mut socket = server.connect(&server.token).await;
    assert!(
        matches!(event(&mut socket).await, ServerMessage::Ready { backend, .. } if backend.synthetic)
    );
    let chunk = AudioChunk::new(0, 0, vec![0.0; 1000]).unwrap();
    socket
        .send(Message::Binary(encode_audio(&chunk).into()))
        .await
        .unwrap();
    socket
        .send(Message::Text(r#"{"type":"finish"}"#.into()))
        .await
        .unwrap();
    let mut finals = Vec::new();
    loop {
        match event(&mut socket).await {
            ServerMessage::Caption { caption } => {
                if caption.is_final {
                    finals.push(caption.segment());
                }
            }
            ServerMessage::Finished {
                samples_processed, ..
            } => {
                assert_eq!(samples_processed, 1000);
                break;
            }
            other => panic!("unexpected stream event: {other:?}"),
        }
    }
    assert!(
        !finals.is_empty(),
        "the last utterance must not be lost on stop"
    );
    assert!(finals.iter().all(|segment| !segment.text.is_empty()
        && segment.start_ms < segment.end_ms
        && segment.end_ms <= 63));
}

#[tokio::test]
async fn rejects_untrusted_origins_and_invalid_tokens() {
    let server = TestServer::start().await;
    let mut request = server.url.clone().into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", "https://untrusted.example".parse().unwrap());
    match connect_async(request).await {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status().as_u16(), 403)
        }
        other => panic!("expected an origin rejection, got {other:?}"),
    }
    let mut socket = server.connect("wrong").await;
    assert!(
        matches!(event(&mut socket).await, ServerMessage::Error { code, .. } if code == "unauthorized")
    );
}

#[tokio::test]
async fn malformed_or_missing_audio_is_reported_instead_of_silently_accepted() {
    let gap = encode_audio(&AudioChunk::new(2, 640, vec![0.0; 320]).unwrap());
    for invalid in [vec![0_u8; 25], gap] {
        let server = TestServer::start().await;
        let mut socket = server.connect(&server.token).await;
        assert!(matches!(
            event(&mut socket).await,
            ServerMessage::Ready { .. }
        ));
        let first = AudioChunk::new(0, 0, vec![0.0; 320]).unwrap();
        socket
            .send(Message::Binary(encode_audio(&first).into()))
            .await
            .unwrap();
        socket.send(Message::Binary(invalid.into())).await.unwrap();
        loop {
            match event(&mut socket).await {
                ServerMessage::Error { .. } => break,
                ServerMessage::Caption { .. } => {}
                other => panic!("expected an audio error, got {other:?}"),
            }
        }
    }
}

#[tokio::test]
async fn stopping_the_server_closes_active_connections() {
    let server = TestServer::start().await;
    let mut socket = server.connect(&server.token).await;
    event(&mut socket).await;
    server.shutdown.send(true).unwrap();
    let message = timeout(Duration::from_secs(3), socket.next())
        .await
        .unwrap();
    assert!(matches!(message, Some(Ok(Message::Close(_))) | None));
}
