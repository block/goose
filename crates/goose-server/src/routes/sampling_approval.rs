use axum::{
    extract::State,
    http,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use futures::Stream;
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SamplingRequest {
    pub request_id: String,
    pub extension_name: String,
    pub messages: Vec<SamplingMessageContent>,
    pub system_prompt: Option<String>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessageContent {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SamplingResponse {
    pub request_id: String,
    pub approved: bool,
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

#[derive(Clone)]
pub struct SamplingApprovalState {
    /// channel for sending sampling approval requests to UI and pending state for them
    broadcast_tx: Arc<RwLock<Vec<mpsc::Sender<String>>>>,
    pending_requests: Arc<RwLock<HashMap<String, oneshot::Sender<bool>>>>,
}

impl SamplingApprovalState {
    pub fn new() -> Self {
        Self {
            broadcast_tx: Arc::new(RwLock::new(Vec::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn request_approval(
        &self,
        extension_name: String,
        messages: Vec<SamplingMessageContent>,
        system_prompt: Option<String>,
        max_tokens: u32,
    ) -> Result<bool, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // Store the response channel
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Create the request
        let request = SamplingRequest {
            request_id: request_id.clone(),
            extension_name,
            messages,
            system_prompt,
            max_tokens,
        };

        let message = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let sse_message = format!("data: {}\n\n", message);

        {
            let senders = self.broadcast_tx.read().await;
            for sender in senders.iter() {
                let _ = sender.send(sse_message.clone()).await;
            }
        }

        match tokio::time::timeout(std::time::Duration::from_secs(300), rx).await {
            Ok(Ok(approved)) => Ok(approved),
            Ok(Err(_)) => Err("Response channel closed".to_string()),
            Err(_) => {
                // represents timeouts. reap and return error
                let mut pending = self.pending_requests.write().await;
                pending.remove(&request_id);
                Err("Approval request timed out".to_string())
            }
        }
    }

    pub async fn submit_response(&self, request_id: String, approved: bool) -> Result<(), String> {
        let mut pending = self.pending_requests.write().await;
        if let Some(tx) = pending.remove(&request_id) {
            tx.send(approved)
                .map_err(|_| "Failed to send response".to_string())?;
            Ok(())
        } else {
            Err("Request not found or already responded".to_string())
        }
    }
}

impl Default for SamplingApprovalState {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl goose::agents::extension_manager::SamplingApprovalHandler for SamplingApprovalState {
    async fn request_approval(
        &self,
        extension_name: String,
        messages: Vec<rmcp::model::SamplingMessage>,
        system_prompt: Option<String>,
        max_tokens: u32,
    ) -> Result<bool, String> {
        // Convert rmcp::model::SamplingMessage to our SamplingMessageContent
        let converted_messages: Vec<SamplingMessageContent> = messages
            .into_iter()
            .map(|msg| {
                let content = if let Some(text) = msg.content.as_text() {
                    text.text.clone()
                } else {
                    // For non-text content, use a placeholder or serialize
                    format!("{:?}", msg.content)
                };
                SamplingMessageContent {
                    role: format!("{:?}", msg.role).to_lowercase(),
                    content,
                }
            })
            .collect();

        self.request_approval(
            extension_name,
            converted_messages,
            system_prompt,
            max_tokens,
        )
        .await
    }
}

#[utoipa::path(
    get,
    path = "/sampling-approval",
    responses(
        (status = 200, description = "SSE stream of sampling approval requests", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized - invalid secret key")
    )
)]
pub async fn sampling_approval_stream(
    State(state): State<Arc<SamplingApprovalState>>,
) -> SseResponse {
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    {
        let mut senders = state.broadcast_tx.write().await;
        senders.push(tx);
    }

    SseResponse::new(stream)
}

#[utoipa::path(
    post,
    path = "/sampling-approval",
    request_body = SamplingResponse,
    responses(
        (status = 200, description = "Approval response submitted successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized - invalid secret key")
    )
)]
pub async fn submit_sampling_response(
    State(state): State<Arc<SamplingApprovalState>>,
    Json(response): Json<SamplingResponse>,
) -> Result<impl IntoResponse, (http::StatusCode, String)> {
    state
        .submit_response(response.request_id, response.approved)
        .await
        .map_err(|e| (http::StatusCode::BAD_REQUEST, e))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub fn routes(state: Arc<SamplingApprovalState>) -> Router {
    Router::new()
        .route("/sampling-approval", get(sampling_approval_stream))
        .route("/sampling-approval", post(submit_sampling_response))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sampling_approval_flow() {
        let state = SamplingApprovalState::new();

        let state_clone = state.clone();
        let approval_task = tokio::spawn(async move {
            state_clone
                .request_approval(
                    "test-extension".to_string(),
                    vec![SamplingMessageContent {
                        role: "user".to_string(),
                        content: "Test message".to_string(),
                    }],
                    Some("Test system prompt".to_string()),
                    100,
                )
                .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_id = {
            let pending = state.pending_requests.read().await;
            pending.keys().next().unwrap().clone()
        };

        state
            .submit_response(request_id, true)
            .await
            .expect("Should submit response");

        let result = approval_task.await.expect("Task should complete");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_sampling_approval_timeout() {
        let state = SamplingApprovalState::new();

        // request approval but don't respond - should timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            state.request_approval(
                "test-extension".to_string(),
                vec![SamplingMessageContent {
                    role: "user".to_string(),
                    content: "Test message".to_string(),
                }],
                Some("Test system prompt".to_string()),
                100,
            ),
        )
        .await;

        // Should timeout
        assert!(result.is_err());
    }
}
