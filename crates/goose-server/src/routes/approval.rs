use axum::{
    extract::State,
    http,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use futures::Stream;
use goose::agents::approval::{ApprovalAction, ApprovalType};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

/// A request for user approval
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub request_id: String,
    pub session_id: String,
    #[serde(flatten)]
    pub approval_type: ApprovalType,
}

/// A response to an approval request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResponse {
    pub request_id: String,
    pub action: ApprovalAction,
}

pub struct SseResponse {
    rx: ReceiverStream<String>,
}

impl SseResponse {
    fn new(rx: ReceiverStream<String>) -> Self {
        Self { rx }
    }
}

impl Stream for SseResponse {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx)
            .poll_next(cx)
            .map(|opt| opt.map(|s| Ok(Bytes::from(s))))
    }
}

impl IntoResponse for SseResponse {
    fn into_response(self) -> axum::response::Response {
        let stream = self;
        let body = axum::body::Body::from_stream(stream);

        http::Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap()
    }
}

/// State for managing approval requests and responses
#[derive(Clone)]
pub struct ApprovalState {
    /// Channel for broadcasting approval requests to connected UI clients
    broadcast_tx: Arc<RwLock<Vec<mpsc::Sender<String>>>>,
    /// Pending approval requests awaiting user response
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalAction>>>>,
}

impl ApprovalState {
    pub fn new() -> Self {
        Self {
            broadcast_tx: Arc::new(RwLock::new(Vec::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Submit a response to an approval request
    pub async fn submit_response(
        &self,
        request_id: String,
        action: ApprovalAction,
    ) -> Result<(), String> {
        let mut pending = self.pending_requests.write().await;
        if let Some(tx) = pending.remove(&request_id) {
            tx.send(action)
                .map_err(|_| "Failed to send response".to_string())?;
            Ok(())
        } else {
            Err("Request not found or already responded".to_string())
        }
    }

    /// Add a new SSE client connection
    async fn add_client(&self, tx: mpsc::Sender<String>) {
        let mut senders = self.broadcast_tx.write().await;
        senders.push(tx);
    }
}

impl Default for ApprovalState {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement the ApprovalHandler trait from goose crate
#[async_trait::async_trait]
impl goose::agents::approval::ApprovalHandler for ApprovalState {
    async fn request_approval(
        &self,
        session_id: String,
        approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // store: request id -> channel so the channel can be used to continue the task when requests come back in
        let mut pending = self.pending_requests.write().await;
        pending.insert(request_id.clone(), tx);

        let request = ApprovalRequest {
            request_id: request_id.clone(),
            session_id,
            approval_type,
        };

        // Send the approval request to the user
        let message = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let sse_message = format!("data: {}\n\n", message);
        let senders = self.broadcast_tx.read().await;
        for sender in senders.iter() {
            let _ = sender.send(sse_message.clone()).await;
        }

        // timeout and expire state in 5 minutes if user has not responded
        match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(action)) => Ok(action),
            Ok(Err(_)) => Err("Response channel closed".to_string()),
            Err(_) => {
                // Timeout - clean up pending request
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err("Approval request timed out".to_string())
            }
        }
    }
}

#[utoipa::path(
    get,
    path = "/approval",
    responses(
        (status = 200, description = "SSE stream of approval requests", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized - invalid secret key")
    )
)]
pub async fn approval_stream(State(state): State<Arc<ApprovalState>>) -> SseResponse {
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    state.add_client(tx).await;

    SseResponse::new(stream)
}

#[utoipa::path(
    post,
    path = "/approval",
    request_body = ApprovalResponse,
    responses(
        (status = 200, description = "Approval response submitted successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - invalid secret key")
    )
)]
pub async fn submit_approval_response(
    State(state): State<Arc<ApprovalState>>,
    Json(response): Json<ApprovalResponse>,
) -> Result<impl IntoResponse, (http::StatusCode, String)> {
    state
        .submit_response(response.request_id, response.action)
        .await
        .map_err(|e| (http::StatusCode::BAD_REQUEST, e))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub fn routes(state: Arc<ApprovalState>) -> Router {
    Router::new()
        .route("/approval", get(approval_stream))
        .route("/approval", post(submit_approval_response))
        .with_state(state)
}
