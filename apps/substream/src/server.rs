use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use axum::{
    Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use futures_util::SinkExt;
use substream_protocol::{ClientMessage, MAX_FRAME_BYTES, ServerMessage, VERSION, decode_audio};
use tokio::{
    net::TcpListener,
    sync::{Semaphore, mpsc, watch},
    time::{Instant, timeout},
};

use crate::{
    auth::Token,
    config::RecognizerConfig,
    display::DisplayHub,
    worker::{self, QueuedAudio},
};

#[derive(Clone)]
struct AppState {
    token: Token,
    allowed_origins: Vec<String>,
    recognizer: RecognizerConfig,
    connections: Arc<Semaphore>,
    inference: Arc<Semaphore>,
    display_connections: Arc<Semaphore>,
    display: DisplayHub,
    shutdown: watch::Receiver<bool>,
}

pub struct ServerConfig {
    pub address: SocketAddr,
    pub token: Token,
    pub allowed_origins: Vec<String>,
    pub recognizer: RecognizerConfig,
}

pub async fn serve(config: ServerConfig) -> Result<()> {
    ensure!(
        config.address.ip().is_loopback(),
        "daemon must bind to a loopback address"
    );
    let listener = TcpListener::bind(config.address)
        .await
        .context("bind daemon")?;
    tracing::info!(address = %listener.local_addr()?, "Substream is listening");
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let router = router(config, shutdown_rx);
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_tx.send(true);
        })
        .await
        .context("serve daemon")
}

/// Exposed for transport integration tests and embedding in another host.
pub fn router(config: ServerConfig, shutdown: watch::Receiver<bool>) -> Router {
    let state = AppState {
        token: config.token,
        allowed_origins: config.allowed_origins,
        recognizer: config.recognizer,
        connections: Arc::new(Semaphore::new(4)),
        inference: Arc::new(Semaphore::new(1)),
        display_connections: Arc::new(Semaphore::new(8)),
        display: DisplayHub::default(),
        shutdown,
    };
    Router::new()
        .route("/health", get(|| async { "substream/1\n" }))
        .route("/v1/stream", get(upgrade))
        .route("/v1/display", get(upgrade_display))
        .with_state(state)
}

async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    upgrade_socket(state, headers, ws, false)
}

async fn upgrade_display(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    upgrade_socket(state, headers, ws, true)
}

fn upgrade_socket(
    state: AppState,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
    display: bool,
) -> Response {
    if let Some(origin) = headers.get("origin") {
        let allowed = origin
            .to_str()
            .ok()
            .is_some_and(|origin| state.allowed_origins.iter().any(|s| s == origin));
        if !allowed {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    let connections = if display {
        &state.display_connections
    } else {
        &state.connections
    };
    let Ok(permit) = Arc::clone(connections).try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.max_message_size(MAX_FRAME_BYTES)
        .max_frame_size(MAX_FRAME_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            if display {
                display_session(socket, state).await;
            } else {
                session(socket, state).await;
            }
        })
}

async fn send(socket: &mut WebSocket, message: &impl serde::Serialize) -> Result<()> {
    let text = serde_json::to_string(message)?;
    timeout(
        Duration::from_secs(3),
        socket.send(Message::Text(text.into())),
    )
    .await??;
    Ok(())
}

async fn fail(socket: &mut WebSocket, code: &str, message: &str) {
    let _ = send(
        socket,
        &ServerMessage::Error {
            code: code.into(),
            message: message.into(),
        },
    )
    .await;
    let _ = timeout(Duration::from_secs(1), socket.close()).await;
}

async fn authenticate(socket: &mut WebSocket, token: &Token) -> bool {
    let authenticated = match timeout(Duration::from_secs(5), socket.recv()).await {
        Ok(Some(Ok(Message::Text(text)))) => matches!(
            serde_json::from_str::<ClientMessage>(&text),
            Ok(ClientMessage::Authenticate { version: VERSION, token: supplied }) if token.matches(&supplied)
        ),
        _ => false,
    };
    if !authenticated {
        fail(
            socket,
            "unauthorized",
            "valid v1 authentication is required as the first message",
        )
        .await;
    }
    authenticated
}

async fn display_session(mut socket: WebSocket, mut state: AppState) {
    if !authenticate(&mut socket, &state.token).await {
        return;
    }
    let mut updates = state.display.subscribe();
    loop {
        let snapshot = updates.borrow_and_update().snapshot();
        if send(&mut socket, &snapshot).await.is_err() {
            break;
        }
        loop {
            tokio::select! {
                _ = state.shutdown.changed() => {
                    let _ = timeout(Duration::from_secs(1), socket.close()).await;
                    return;
                }
                changed = updates.changed() => {
                    if changed.is_err() { return; }
                    break;
                }
                incoming = socket.recv() => match incoming {
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                    Some(Ok(Message::Text(_) | Message::Binary(_))) => {
                        fail(&mut socket, "read_only", "display connections only receive caption state").await;
                        return;
                    }
                    _ => return,
                }
            }
        }
    }
}

async fn session(mut socket: WebSocket, mut state: AppState) {
    if !authenticate(&mut socket, &state.token).await {
        return;
    }
    let Ok(inference) = Arc::clone(&state.inference).try_acquire_owned() else {
        fail(&mut socket, "busy", "one live session is already active").await;
        return;
    };
    let (input_tx, input_rx) = mpsc::channel(worker::AUDIO_QUEUE_CAPACITY);
    let display = state.display.start();
    let (output_tx, mut output_rx) = mpsc::channel(worker::EVENT_QUEUE_CAPACITY);
    let finish = Arc::new(AtomicBool::new(false));
    let worker_finish = Arc::clone(&finish);
    tokio::task::spawn_blocking(move || {
        worker::run(
            state.recognizer,
            input_rx,
            output_tx,
            worker_finish,
            inference,
        )
    });
    let mut input = Some(input_tx);
    let mut ready = false;
    let mut idle_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        tokio::select! {
            _ = state.shutdown.changed() => break,
            _ = tokio::time::sleep_until(idle_deadline) => {
                fail(&mut socket, "timeout", "session inactive or finalization timed out").await;
                break;
            }
            event = output_rx.recv() => {
                let Some(event) = event else {
                    fail(&mut socket, "worker_stopped", "inference worker stopped before completion").await;
                    break;
                };
                let complete = matches!(event, ServerMessage::Finished { .. } | ServerMessage::Error { .. });
                if matches!(event, ServerMessage::Ready { .. }) { ready = true; }
                display.publish(&event);
                if send(&mut socket, &event).await.is_err() || complete { break; }
            }
            incoming = socket.recv(), if input.is_some() => {
                let Some(Ok(message)) = incoming else { break; };
                if !ready && matches!(message, Message::Binary(_) | Message::Text(_)) {
                    fail(&mut socket, "not_ready", "wait for ready before sending audio or finish").await;
                    break;
                }
                idle_deadline = Instant::now() + Duration::from_secs(30);
                match message {
                    Message::Binary(bytes) => {
                        let chunk = match decode_audio(&bytes) {
                            Ok(chunk) => chunk,
                            Err(error) => {
                                fail(&mut socket, "bad_audio", &error.to_string()).await;
                                break;
                            }
                        };
                        let audio = QueuedAudio { chunk, received: std::time::Instant::now() };
                        if input.as_ref().expect("input enabled").try_send(audio).is_err() {
                            fail(&mut socket, "overloaded", "audio queue is full or closed; restart the session").await;
                            break;
                        }
                    }
                    Message::Text(text) => {
                        if matches!(serde_json::from_str::<ClientMessage>(&text), Ok(ClientMessage::Finish)) {
                            finish.store(true, Ordering::Release);
                            input.take();
                        } else {
                            fail(&mut socket, "bad_control", "only finish is valid after authentication").await;
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    Message::Ping(_) | Message::Pong(_) => {}
                }
            }
        }
    }
    // Dropping both ends unblocks a worker waiting for input or a slow consumer.
    drop(input);
    drop(output_rx);
    let _ = timeout(Duration::from_secs(1), socket.close()).await;
}
