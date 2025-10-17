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

/// The action to take in response to an approval request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAction {
    /// Allow this specific request once
    AllowOnce,
    /// Always allow requests of this type
    AlwaysAllow,
    /// Deny this request
    Deny,
}

/// Content in a sampling message - can be text, image, or audio
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SamplingMessageContent {
    /// Text content
    Text { text: String },
    /// Image content
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Audio content
    Audio {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

/// A message in a sampling request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SamplingMessage {
    pub role: String,
    pub content: SamplingMessageContent,
}

impl From<rmcp::model::SamplingMessage> for SamplingMessage {
    fn from(msg: rmcp::model::SamplingMessage) -> Self {
        let role = format!("{:?}", msg.role).to_lowercase();
        let content = if let Some(text) = msg.content.as_text() {
            SamplingMessageContent::Text {
                text: text.text.clone(),
            }
        } else if let Some(image) = msg.content.as_image() {
            SamplingMessageContent::Image {
                data: image.data.clone(),
                mime_type: image.mime_type.clone(),
            }
        } else {
            // For any other content type (resource, etc.), convert to text representation
            SamplingMessageContent::Text {
                text: format!("{:?}", msg.content),
            }
        };
        SamplingMessage { role, content }
    }
}

/// The type of approval being requested
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ApprovalType {
    /// Approval for a tool call
    #[serde(rename_all = "camelCase")]
    ToolCall {
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        principal_type: String,
    },
    /// Approval for MCP sampling
    #[serde(rename_all = "camelCase")]
    Sampling {
        extension_name: String,
        messages: Vec<SamplingMessage>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system_prompt: Option<String>,
        max_tokens: u32,
    },
}

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

    /// Request approval from the user (internal method)
    async fn request_approval_internal(
        &self,
        session_id: String,
        approval_type: ApprovalType,
    ) -> Result<ApprovalAction, String> {
        let request_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        // Store the response channel
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Create the request
        let request = ApprovalRequest {
            request_id: request_id.clone(),
            session_id,
            approval_type,
        };

        let message = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let sse_message = format!("data: {}\n\n", message);

        // Broadcast to all connected clients
        {
            let senders = self.broadcast_tx.read().await;
            for sender in senders.iter() {
                let _ = sender.send(sse_message.clone()).await;
            }
        }

        // Wait for response with timeout
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
        approval_type: goose::agents::approval::ApprovalType,
    ) -> Result<goose::agents::approval::ApprovalAction, String> {
        // Convert from goose ApprovalType to server ApprovalType
        let server_approval_type = match approval_type {
            goose::agents::approval::ApprovalType::ToolCall {
                tool_name,
                prompt,
                principal_type,
            } => ApprovalType::ToolCall {
                tool_name,
                prompt,
                principal_type,
            },
            goose::agents::approval::ApprovalType::Sampling {
                extension_name,
                messages,
                system_prompt,
                max_tokens,
            } => ApprovalType::Sampling {
                extension_name,
                messages: messages.into_iter().map(|m| m.into()).collect(),
                system_prompt,
                max_tokens,
            },
        };

        let action = self
            .request_approval_internal(session_id, server_approval_type)
            .await?;

        // Convert from server ApprovalAction to goose ApprovalAction
        Ok(match action {
            ApprovalAction::AllowOnce => goose::agents::approval::ApprovalAction::AllowOnce,
            ApprovalAction::AlwaysAllow => goose::agents::approval::ApprovalAction::AlwaysAllow,
            ApprovalAction::Deny => goose::agents::approval::ApprovalAction::Deny,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approval_flow() {
        let state = ApprovalState::new();
        let state_clone = state.clone();
        let approval_task = tokio::spawn(async move {
            state_clone
                .request_approval_internal(
                    "test-session".to_string(),
                    ApprovalType::Sampling {
                        extension_name: "test-extension".to_string(),
                        messages: vec![SamplingMessage {
                            role: "user".to_string(),
                            content: SamplingMessageContent::Text {
                                text: "Test message".to_string(),
                            },
                        }],
                        system_prompt: Some("Test system prompt".to_string()),
                        max_tokens: 100,
                    },
                )
                .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_id = {
            let pending = state.pending_requests.read().await;
            pending.keys().next().unwrap().clone()
        };

        state
            .submit_response(request_id, ApprovalAction::AllowOnce)
            .await
            .expect("Should submit response");

        let result = approval_task.await.expect("Task should complete");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ApprovalAction::AllowOnce));
    }

    #[tokio::test]
    async fn test_approval_timeout() {
        let state = ApprovalState::new();

        // Request approval but don't respond - should timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            state.request_approval_internal(
                "test-session".to_string(),
                ApprovalType::ToolCall {
                    tool_name: "test_tool".to_string(),
                    prompt: None,
                    principal_type: "Tool".to_string(),
                },
            ),
        )
        .await;

        // Should timeout
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_call_approval() {
        let state = ApprovalState::new();

        let state_clone = state.clone();
        let approval_task = tokio::spawn(async move {
            state_clone
                .request_approval_internal(
                    "test-session".to_string(),
                    ApprovalType::ToolCall {
                        tool_name: "developer__shell".to_string(),
                        prompt: Some("This tool will execute a shell command".to_string()),
                        principal_type: "Tool".to_string(),
                    },
                )
                .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let request_id = {
            let pending = state.pending_requests.read().await;
            pending.keys().next().unwrap().clone()
        };

        state
            .submit_response(request_id, ApprovalAction::AlwaysAllow)
            .await
            .expect("Should submit response");

        let result = approval_task.await.expect("Task should complete");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ApprovalAction::AlwaysAllow));
    }
}
