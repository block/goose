use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header, Method, StatusCode},
    response::Sse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info};

use crate::server_factory::AcpServer;

struct HttpSession {
    to_agent_tx: mpsc::Sender<String>,
    from_agent_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    _handle: tokio::task::JoinHandle<()>,
}

pub struct HttpState {
    server: Arc<AcpServer>,
    sessions: RwLock<HashMap<String, HttpSession>>,
}

impl HttpState {
    pub fn new(server: Arc<AcpServer>) -> Self {
        Self {
            server,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    async fn create_session(&self) -> Result<String, StatusCode> {
        let (to_agent_tx, to_agent_rx) = mpsc::channel::<String>(256);
        let (from_agent_tx, from_agent_rx) = mpsc::channel::<String>(256);

        let agent = self.server.create_agent().await.map_err(|e| {
            error!("Failed to create agent: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Create the ACP session upfront and use its ID as the HTTP session ID
        let session_id = agent.create_session().await.map_err(|e| {
            error!("Failed to create ACP session: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let handle = tokio::spawn(async move {
            let read_stream = ReceiverToAsyncRead::new(to_agent_rx);
            let write_stream = SenderToAsyncWrite::new(from_agent_tx);

            if let Err(e) =
                crate::server::serve(agent, read_stream.compat(), write_stream.compat_write()).await
            {
                error!("ACP session error: {}", e);
            }
        });

        self.sessions.write().await.insert(
            session_id.clone(),
            HttpSession {
                to_agent_tx,
                from_agent_rx: Arc::new(Mutex::new(from_agent_rx)),
                _handle: handle,
            },
        );

        info!(session_id = %session_id, "Session created");
        Ok(session_id)
    }

    async fn send_message(&self, session_id: &str, message: String) -> Result<(), StatusCode> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id).ok_or(StatusCode::NOT_FOUND)?;
        session
            .to_agent_tx
            .send(message)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }

    async fn get_receiver(
        &self,
        session_id: &str,
    ) -> Result<Arc<Mutex<mpsc::Receiver<String>>>, StatusCode> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(session_id).ok_or(StatusCode::NOT_FOUND)?;
        Ok(session.from_agent_rx.clone())
    }
}

struct ReceiverToAsyncRead {
    rx: mpsc::Receiver<String>,
    buffer: Vec<u8>,
    pos: usize,
}

impl ReceiverToAsyncRead {
    fn new(rx: mpsc::Receiver<String>) -> Self {
        Self {
            rx,
            buffer: Vec::new(),
            pos: 0,
        }
    }
}

impl tokio::io::AsyncRead for ReceiverToAsyncRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pos < self.buffer.len() {
            let remaining = &self.buffer[self.pos..];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            self.pos += to_copy;
            if self.pos >= self.buffer.len() {
                self.buffer.clear();
                self.pos = 0;
            }
            return Poll::Ready(Ok(()));
        }

        match Pin::new(&mut self.rx).poll_recv(cx) {
            Poll::Ready(Some(msg)) => {
                let bytes = format!("{}\n", msg).into_bytes();
                let to_copy = bytes.len().min(buf.remaining());
                buf.put_slice(&bytes[..to_copy]);
                if to_copy < bytes.len() {
                    self.buffer = bytes[to_copy..].to_vec();
                    self.pos = 0;
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct SenderToAsyncWrite {
    tx: mpsc::Sender<String>,
    buffer: Vec<u8>,
}

impl SenderToAsyncWrite {
    fn new(tx: mpsc::Sender<String>) -> Self {
        Self {
            tx,
            buffer: Vec::new(),
        }
    }
}

impl tokio::io::AsyncWrite for SenderToAsyncWrite {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.buffer.extend_from_slice(buf);

        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(&self.buffer[..pos]).to_string();
            self.buffer.drain(..=pos);

            if !line.is_empty() {
                let tx = self.tx.clone();
                tokio::spawn(async move {
                    let _ = tx.send(line).await;
                });
            }
        }

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[derive(Serialize)]
struct CreateSessionResponse {
    session_id: String,
}

async fn create_session(
    State(state): State<Arc<HttpState>>,
) -> Result<Json<CreateSessionResponse>, StatusCode> {
    let session_id = state.create_session().await?;
    Ok(Json(CreateSessionResponse { session_id }))
}

async fn send_message(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
    body: String,
) -> Result<StatusCode, StatusCode> {
    debug!(session_id = %session_id, "Received message");
    state.send_message(&session_id, body).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn stream_events(
    State(state): State<Arc<HttpState>>,
    Path(session_id): Path<String>,
) -> Result<
    Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, Infallible>>>,
    StatusCode,
> {
    let receiver = state.get_receiver(&session_id).await?;

    let stream = async_stream::stream! {
        let mut rx = receiver.lock().await;
        while let Some(msg) = rx.recv().await {
            yield Ok(axum::response::sse::Event::default().data(msg));
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    ))
}

async fn health() -> &'static str {
    "ok"
}

pub fn create_router(state: Arc<HttpState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    Router::new()
        .route("/health", get(health))
        .route("/acp/session", post(create_session))
        .route("/acp/session/{session_id}/message", post(send_message))
        .route("/acp/session/{session_id}/stream", get(stream_events))
        .layer(cors)
        .with_state(state)
}

pub async fn serve(state: Arc<HttpState>, addr: std::net::SocketAddr) -> Result<()> {
    let router = create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("ACP HTTP server listening on {}", addr);
    axum::serve(listener, router).await?;
    Ok(())
}
