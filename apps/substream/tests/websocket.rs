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
async fn authenticated_audio_flushes_one_short_final_before_finished() {
    let server = TestServer::start().await;
    let mut socket = server.connect(&server.token).await;
    assert!(
        matches!(event(&mut socket).await, ServerMessage::Ready { backend, .. } if backend.synthetic)
    );
    let chunk = AudioChunk::new(0, 0, vec![0.0; 320]).unwrap();
    socket
        .send(Message::Binary(encode_audio(&chunk).into()))
        .await
        .unwrap();
    socket
        .send(Message::Text(r#"{"type":"finish"}"#.into()))
        .await
        .unwrap();
    match event(&mut socket).await {
        ServerMessage::Caption { caption } => {
            assert!(caption.is_final);
            assert_eq!((caption.start_ms, caption.end_ms), (0, 20));
        }
        other => panic!("expected final caption, got {other:?}"),
    }
    assert!(matches!(
        event(&mut socket).await,
        ServerMessage::Finished {
            samples_processed: 320,
            ..
        }
    ));
}

#[tokio::test]
async fn rejects_unknown_origins_and_bad_tokens_before_loading_a_model() {
    let server = TestServer::start().await;
    let mut request = server.url.clone().into_client_request().unwrap();
    request
        .headers_mut()
        .insert("origin", "https://untrusted.example".parse().unwrap());
    assert!(connect_async(request).await.is_err());
    let mut socket = server.connect("wrong").await;
    assert!(
        matches!(event(&mut socket).await, ServerMessage::Error { code, .. } if code == "unauthorized")
    );
}

#[tokio::test]
async fn limits_concurrent_inference_and_reports_timeline_gaps() {
    let server = TestServer::start().await;
    let mut first = server.connect(&server.token).await;
    assert!(matches!(
        event(&mut first).await,
        ServerMessage::Ready { .. }
    ));
    let mut second = server.connect(&server.token).await;
    assert!(
        matches!(event(&mut second).await, ServerMessage::Error { code, .. } if code == "busy")
    );
    let gap = AudioChunk::new(1, 320, vec![0.0; 320]).unwrap();
    first
        .send(Message::Binary(encode_audio(&gap).into()))
        .await
        .unwrap();
    assert!(
        matches!(event(&mut first).await, ServerMessage::Error { code, .. } if code == "worker_failed")
    );
}

#[tokio::test]
async fn rejects_malformed_frames_and_cancels_an_idle_session_on_shutdown() {
    let server = TestServer::start().await;
    let mut socket = server.connect(&server.token).await;
    event(&mut socket).await;
    socket
        .send(Message::Binary(vec![0_u8; 25].into()))
        .await
        .unwrap();
    assert!(
        matches!(event(&mut socket).await, ServerMessage::Error { code, .. } if code == "bad_audio")
    );

    let other_server = TestServer::start().await;
    let mut socket = other_server.connect(&other_server.token).await;
    event(&mut socket).await;
    other_server.shutdown.send(true).unwrap();
    let message = timeout(Duration::from_secs(3), socket.next())
        .await
        .unwrap();
    assert!(matches!(message, Some(Ok(Message::Close(_))) | None));
}
