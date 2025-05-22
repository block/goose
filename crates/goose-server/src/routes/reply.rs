use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::State,
    http::{self, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, Stream};
use goose::{
    agents::{AgentEvent, SessionConfig},
    message::{Message, MessageContent},
    permission::permission_confirmation::PrincipalType,
};
use goose::{
    permission::{Permission, PermissionConfirmation},
    session,
};
use mcp_core::{protocol::JsonRpcMessage, role::Role, Content, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::{
    convert::Infallible,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use utoipa::ToSchema;

// Direct message serialization for the chat request
#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<Message>,
    session_id: Option<String>,
    session_working_dir: String,
}

// Custom SSE response type for streaming messages
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

// Message event types for SSE streaming
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum MessageEvent {
    Message {
        message: Message,
    },
    Error {
        error: String,
    },
    Finish {
        reason: String,
    },
    Notification {
        request_id: String,
        message: JsonRpcMessage,
    },
}

// Stream a message as an SSE event
async fn stream_event(
    event: MessageEvent,
    tx: &mpsc::Sender<String>,
) -> Result<(), mpsc::error::SendError<String>> {
    let json = serde_json::to_string(&event).unwrap_or_else(|e| {
        format!(
            r#"{{"type":"Error","error":"Failed to serialize event: {}"}}"#,
            e
        )
    });
    tx.send(format!("data: {}\n\n", json)).await
}

async fn handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Create channel for streaming
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    let messages = request.messages;
    let session_working_dir = request.session_working_dir;

    // Generate a new session ID if not provided in the request
    let session_id = request
        .session_id
        .unwrap_or_else(session::generate_session_id);

    // Spawn task to handle streaming
    tokio::spawn(async move {
        let agent = state.get_agent().await;
        let agent = match agent {
            Ok(agent) => {
                let provider = agent.provider().await;
                match provider {
                    Ok(_) => agent,
                    Err(_) => {
                        let _ = stream_event(
                            MessageEvent::Error {
                                error: "No provider configured".to_string(),
                            },
                            &tx,
                        )
                        .await;
                        let _ = stream_event(
                            MessageEvent::Finish {
                                reason: "error".to_string(),
                            },
                            &tx,
                        )
                        .await;
                        return;
                    }
                }
            }
            Err(_) => {
                let _ = stream_event(
                    MessageEvent::Error {
                        error: "No agent configured".to_string(),
                    },
                    &tx,
                )
                .await;
                let _ = stream_event(
                    MessageEvent::Finish {
                        reason: "error".to_string(),
                    },
                    &tx,
                )
                .await;
                return;
            }
        };

        // Get the provider first, before starting the reply stream
        let provider = agent.provider().await;

        let mut stream = match agent
            .reply(
                &messages,
                Some(SessionConfig {
                    id: session::Identifier::Name(session_id.clone()),
                    working_dir: PathBuf::from(session_working_dir),
                }),
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {:?}", e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: e.to_string(),
                    },
                    &tx,
                )
                .await;
                let _ = stream_event(
                    MessageEvent::Finish {
                        reason: "error".to_string(),
                    },
                    &tx,
                )
                .await;
                return;
            }
        };

        // Collect all messages for storage
        let mut all_messages = messages.clone();
        let session_path = session::get_path(session::Identifier::Name(session_id.clone()));

        loop {
            tokio::select! {
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            all_messages.push(message.clone());
                            if let Err(e) = stream_event(MessageEvent::Message { message }, &tx).await {
                                tracing::error!("Error sending message through channel: {}", e);
                                let _ = stream_event(
                                    MessageEvent::Error {
                                        error: e.to_string(),
                                    },
                                    &tx,
                                ).await;
                                break;
                            }

                            // Store messages and generate description in background
                            let session_path = session_path.clone();
                            let messages = all_messages.clone();
                            let provider = Arc::clone(provider.as_ref().unwrap());
                            tokio::spawn(async move {
                                if let Err(e) = session::persist_messages(&session_path, &messages, Some(provider)).await {
                                    tracing::error!("Failed to store session history: {:?}", e);
                                }
                            });
                        }
                        Ok(Some(Ok(AgentEvent::McpNotification((request_id, n))))) => {
                            if let Err(e) = stream_event(MessageEvent::Notification{
                                request_id: request_id.clone(),
                                message: n,
                            }, &tx).await {
                                tracing::error!("Error sending message through channel: {}", e);
                                let _ = stream_event(
                                    MessageEvent::Error {
                                        error: e.to_string(),
                                    },
                                    &tx,
                                ).await;
                            }
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            let _ = stream_event(
                                MessageEvent::Error {
                                    error: e.to_string(),
                                },
                                &tx,
                            ).await;
                            break;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(_) => { // Heartbeat, used to detect disconnected clients
                            if tx.is_closed() {
                                break;
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // Send finish event
        let _ = stream_event(
            MessageEvent::Finish {
                reason: "stop".to_string(),
            },
            &tx,
        )
        .await;
    });

    Ok(SseResponse::new(stream))
}

#[derive(Debug, Deserialize, Serialize)]
struct AskRequest {
    prompt: String,
    session_id: Option<String>,
    session_working_dir: String,
}

#[derive(Debug, Serialize)]
struct AskResponse {
    response: String,
}

// Simple ask an AI for a response, non streaming
async fn ask_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<AskRequest>,
) -> Result<Json<AskResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let session_working_dir = request.session_working_dir;

    // Generate a new session ID if not provided in the request
    let session_id = request
        .session_id
        .unwrap_or_else(session::generate_session_id);

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    // Get the provider first, before starting the reply stream
    let provider = agent.provider().await;

    // Create a single message for the prompt
    let messages = vec![Message::user().with_text(request.prompt)];

    // Get response from agent
    let mut response_text = String::new();
    let mut stream = match agent
        .reply(
            &messages,
            Some(SessionConfig {
                id: session::Identifier::Name(session_id.clone()),
                working_dir: PathBuf::from(session_working_dir),
            }),
        )
        .await
    {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Failed to start reply stream: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Collect all messages for storage
    let mut all_messages = messages.clone();
    let mut response_message = Message::assistant();

    while let Some(response) = stream.next().await {
        match response {
            Ok(AgentEvent::Message(message)) => {
                if message.role == Role::Assistant {
                    for content in &message.content {
                        if let MessageContent::Text(text) = content {
                            response_text.push_str(&text.text);
                            response_text.push('\n');
                        }
                        response_message.content.push(content.clone());
                    }
                }
            }
            Ok(AgentEvent::McpNotification(n)) => {
                // Handle notifications if needed
                tracing::info!("Received notification: {:?}", n);
            }
            Err(e) => {
                tracing::error!("Error processing as_ai message: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // Add the complete response message to the conversation history
    if !response_message.content.is_empty() {
        all_messages.push(response_message);
    }

    // Get the session path - file will be created when needed
    let session_path = session::get_path(session::Identifier::Name(session_id.clone()));

    // Store messages and generate description in background
    let session_path = session_path.clone();
    let messages = all_messages.clone();
    let provider = Arc::clone(provider.as_ref().unwrap());
    tokio::spawn(async move {
        if let Err(e) = session::persist_messages(&session_path, &messages, Some(provider)).await {
            tracing::error!("Failed to store session history: {:?}", e);
        }
    });

    Ok(Json(AskResponse {
        response: response_text.trim().to_string(),
    }))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PermissionConfirmationRequest {
    id: String,
    #[serde(default = "default_principal_type")]
    principal_type: PrincipalType,
    action: String,
}

fn default_principal_type() -> PrincipalType {
    PrincipalType::Tool
}

#[utoipa::path(
    post,
    path = "/confirm",
    request_body = PermissionConfirmationRequest,
    responses(
        (status = 200, description = "Permission action is confirmed", body = Value),
        (status = 401, description = "Unauthorized - invalid secret key"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn confirm_permission(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<PermissionConfirmationRequest>,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    let permission = match request.action.as_str() {
        "always_allow" => Permission::AlwaysAllow,
        "allow_once" => Permission::AllowOnce,
        "deny" => Permission::DenyOnce,
        _ => Permission::DenyOnce,
    };

    agent
        .handle_confirmation(
            request.id.clone(),
            PermissionConfirmation {
                principal_type: request.principal_type,
                permission,
            },
        )
        .await;
    Ok(Json(Value::Object(serde_json::Map::new())))
}

#[derive(Debug, Deserialize)]
struct ToolResultRequest {
    id: String,
    result: ToolResult<Vec<Content>>,
}

async fn submit_tool_result(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    raw: axum::extract::Json<serde_json::Value>,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Log the raw request for debugging
    tracing::info!(
        "Received tool result request: {}",
        serde_json::to_string_pretty(&raw.0).unwrap()
    );

    // Try to parse into our struct
    let payload: ToolResultRequest = match serde_json::from_value(raw.0.clone()) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to parse tool result request: {}", e);
            tracing::error!(
                "Raw request was: {}",
                serde_json::to_string_pretty(&raw.0).unwrap()
            );
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;
    agent.handle_tool_result(payload.id, payload.result).await;
    Ok(Json(json!({"status": "ok"})))
}

// Configure routes for this module
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/reply", post(handler))
        .route("/ask", post(ask_handler))
        .route("/confirm", post(confirm_permission))
        .route("/tool_result", post(submit_tool_result))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::{
        agents::Agent,
        model::ModelConfig,
        providers::{
            base::{Provider, ProviderUsage, Usage},
            errors::ProviderError,
        },
    };
    use mcp_core::tool::Tool;

    // Mock Provider implementation for testing
    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> goose::providers::base::ProviderMetadata {
            goose::providers::base::ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    mod integration_tests {
        use super::*;
        use axum::{body::Body, http::Request};
        use std::sync::Arc;
        use tower::ServiceExt;

        // This test requires tokio runtime
        #[tokio::test]
        async fn test_ask_endpoint() {
            // Create a mock app state with mock provider
            let mock_model_config = ModelConfig::new("test-model".to_string());
            let mock_provider = Arc::new(MockProvider {
                model_config: mock_model_config,
            });
            let agent = Agent::new();
            let _ = agent.update_provider(mock_provider).await;
            let state = AppState::new(Arc::new(agent), "test-secret".to_string()).await;

            // Build router
            let app = routes(state);

            // Create request
            let request = Request::builder()
                .uri("/ask")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&AskRequest {
                        prompt: "test prompt".to_string(),
                        session_id: Some("test-session".to_string()),
                        session_working_dir: "test-working-dir".to_string(),
                    })
                    .unwrap(),
                ))
                .unwrap();

            // Send request
            let response = app.oneshot(request).await.unwrap();

            // Assert response status
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
