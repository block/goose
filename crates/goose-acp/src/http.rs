use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Response, Sse},
    routing::{delete, get, post},
    Router,
};
use http_body_util::BodyExt;
use serde_json::Value;
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
use tracing::{error, info};

use crate::server_factory::AcpServer;

// ACP header constants
const HEADER_SESSION_ID: &str = "Acp-Session-Id";
const EVENT_STREAM_MIME_TYPE: &str = "text/event-stream";
const JSON_MIME_TYPE: &str = "application/json";

struct HttpSession {
    to_agent_tx: mpsc::Sender<String>,
    from_agent_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    handle: tokio::task::JoinHandle<()>,
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
                handle,
            },
        );

        info!(session_id = %session_id, "Session created");
        Ok(session_id)
    }

    async fn has_session(&self, session_id: &str) -> bool {
        self.sessions.read().await.contains_key(session_id)
    }

    async fn remove_session(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().await.remove(session_id) {
            session.handle.abort();
            info!(session_id = %session_id, "Session removed");
        }
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
                if let Err(e) = self.tx.try_send(line.clone()) {
                    match e {
                        mpsc::error::TrySendError::Full(_) => {
                            let truncated: String = line.chars().take(100).collect();
                            error!(
                                "Channel full, dropping message (backpressure): {}",
                                truncated
                            );
                        }
                        mpsc::error::TrySendError::Closed(_) => {
                            return Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::BrokenPipe,
                                "Channel closed",
                            )));
                        }
                    }
                }
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

fn accepts_mime_type(request: &Request<Body>, mime_type: &str) -> bool {
    request
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| accept.contains(mime_type))
}

fn accepts_json_and_sse(request: &Request<Body>) -> bool {
    request
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|accept| {
            accept.contains(JSON_MIME_TYPE) && accept.contains(EVENT_STREAM_MIME_TYPE)
        })
}

fn content_type_is_json(request: &Request<Body>) -> bool {
    request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with(JSON_MIME_TYPE))
}

fn get_session_id(request: &Request<Body>) -> Option<String> {
    request
        .headers()
        .get(HEADER_SESSION_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn is_jsonrpc_request(value: &Value) -> bool {
    value.get("method").is_some() && value.get("id").is_some()
}

fn is_jsonrpc_notification(value: &Value) -> bool {
    value.get("method").is_some() && value.get("id").is_none()
}

fn is_jsonrpc_response(value: &Value) -> bool {
    value.get("id").is_some() && (value.get("result").is_some() || value.get("error").is_some())
}

fn is_initialize_request(value: &Value) -> bool {
    value.get("method").is_some_and(|m| m == "initialize") && value.get("id").is_some()
}

fn create_sse_stream(
    receiver: Arc<Mutex<mpsc::Receiver<String>>>,
    cleanup: Option<(Arc<HttpState>, String)>,
) -> Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut rx = receiver.lock().await;
        while let Some(msg) = rx.recv().await {
            yield Ok::<_, Infallible>(axum::response::sse::Event::default().data(msg));
        }
        if let Some((state, session_id)) = cleanup {
            state.remove_session(&session_id).await;
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text(""),
    )
}

async fn handle_initialize(state: Arc<HttpState>, json_message: &Value) -> Response {
    let new_session_id = match state.create_session().await {
        Ok(id) => id,
        Err(status) => return status.into_response(),
    };

    let message_str = serde_json::to_string(json_message).unwrap();
    if let Err(status) = state.send_message(&new_session_id, message_str).await {
        state.remove_session(&new_session_id).await;
        return status.into_response();
    }

    let receiver = match state.get_receiver(&new_session_id).await {
        Ok(r) => r,
        Err(status) => {
            state.remove_session(&new_session_id).await;
            return status.into_response();
        }
    };

    let sse = create_sse_stream(receiver, Some((state.clone(), new_session_id.clone())));
    let mut response = sse.into_response();
    response
        .headers_mut()
        .insert(HEADER_SESSION_ID, new_session_id.parse().unwrap());
    response
}

async fn handle_request(
    state: Arc<HttpState>,
    session_id: String,
    json_message: &Value,
) -> Response {
    if !state.has_session(&session_id).await {
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    }

    let message_str = serde_json::to_string(json_message).unwrap();
    if let Err(status) = state.send_message(&session_id, message_str).await {
        return status.into_response();
    }

    let receiver = match state.get_receiver(&session_id).await {
        Ok(r) => r,
        Err(status) => return status.into_response(),
    };

    create_sse_stream(receiver, None).into_response()
}

async fn handle_notification_or_response(
    state: Arc<HttpState>,
    session_id: String,
    json_message: &Value,
) -> Response {
    if !state.has_session(&session_id).await {
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    }

    let message_str = serde_json::to_string(json_message).unwrap();
    if let Err(status) = state.send_message(&session_id, message_str).await {
        return status.into_response();
    }

    StatusCode::ACCEPTED.into_response()
}

async fn handle_post(State(state): State<Arc<HttpState>>, request: Request<Body>) -> Response {
    if !accepts_json_and_sse(&request) {
        return (
            StatusCode::NOT_ACCEPTABLE,
            "Not Acceptable: Client must accept both application/json and text/event-stream",
        )
            .into_response();
    }

    if !content_type_is_json(&request) {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Unsupported Media Type: Content-Type must be application/json",
        )
            .into_response();
    }

    let session_id = get_session_id(&request);

    let body_bytes = match request.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    let json_message: Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to parse JSON: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)).into_response();
        }
    };

    if json_message.is_array() {
        return (
            StatusCode::NOT_IMPLEMENTED,
            "Batch requests are not supported",
        )
            .into_response();
    }

    if is_initialize_request(&json_message) {
        handle_initialize(state, &json_message).await
    } else if is_jsonrpc_request(&json_message) {
        let Some(id) = session_id else {
            return (
                StatusCode::BAD_REQUEST,
                "Bad Request: Acp-Session-Id header required",
            )
                .into_response();
        };
        handle_request(state, id, &json_message).await
    } else if is_jsonrpc_notification(&json_message) || is_jsonrpc_response(&json_message) {
        let Some(id) = session_id else {
            return (
                StatusCode::BAD_REQUEST,
                "Bad Request: Acp-Session-Id header required",
            )
                .into_response();
        };
        handle_notification_or_response(state, id, &json_message).await
    } else {
        (StatusCode::BAD_REQUEST, "Invalid JSON-RPC message").into_response()
    }
}

async fn handle_get(State(state): State<Arc<HttpState>>, request: Request<Body>) -> Response {
    if !accepts_mime_type(&request, EVENT_STREAM_MIME_TYPE) {
        return (
            StatusCode::NOT_ACCEPTABLE,
            "Not Acceptable: Client must accept text/event-stream",
        )
            .into_response();
    }

    let session_id = match get_session_id(&request) {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Bad Request: Acp-Session-Id header required",
            )
                .into_response();
        }
    };

    if !state.has_session(&session_id).await {
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    }

    let receiver = match state.get_receiver(&session_id).await {
        Ok(r) => r,
        Err(status) => return status.into_response(),
    };

    let stream = async_stream::stream! {
        let mut rx = receiver.lock().await;
        while let Some(msg) = rx.recv().await {
            yield Ok::<_, Infallible>(axum::response::sse::Event::default().data(msg));
        }
    };

    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text(""),
        )
        .into_response()
}

async fn handle_delete(State(state): State<Arc<HttpState>>, request: Request<Body>) -> Response {
    let session_id = match get_session_id(&request) {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                "Bad Request: Acp-Session-Id header required",
            )
                .into_response();
        }
    };

    if !state.has_session(&session_id).await {
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    }

    state.remove_session(&session_id).await;
    StatusCode::ACCEPTED.into_response()
}

async fn health() -> &'static str {
    "ok"
}

pub fn create_router(state: Arc<HttpState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            HEADER_SESSION_ID.parse().unwrap(),
        ]);

    Router::new()
        .route("/health", get(health))
        .route("/acp", post(handle_post))
        .route("/acp", get(handle_get))
        .route("/acp", delete(handle_delete))
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
