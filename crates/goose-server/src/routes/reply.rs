use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{self, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, Stream};
use goose::{
    agents::{AgentEvent, SessionConfig},
    message::{push_message, Message},
    permission::permission_confirmation::PrincipalType,
};
use goose::{
    permission::{Permission, PermissionConfirmation},
    session,
};
use mcp_core::ToolResult;
use rmcp::model::{Content, ServerNotification};
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
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize)]
struct ChatRequest {
    messages: Vec<Message>,
    session_id: Option<String>,
    session_working_dir: String,
    scheduled_job_id: Option<String>,
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
    ModelChange {
        model: String,
        mode: String,
    },
    Notification {
        request_id: String,
        message: ServerNotification,
    },
}

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

async fn reply_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);
    let cancel_token = CancellationToken::new();

    let messages = request.messages;
    let session_working_dir = request.session_working_dir.clone();

    let session_id = request
        .session_id
        .unwrap_or_else(session::generate_session_id);

    let task_cancel = cancel_token.clone();
    let task_tx = tx.clone();

    std::mem::drop(tokio::spawn(async move {
        let agent = match state.get_agent().await {
            Ok(agent) => agent,
            Err(_) => {
                let _ = stream_event(
                    MessageEvent::Error {
                        error: "No agent configured".to_string(),
                    },
                    &task_tx,
                )
                .await;
                return;
            }
        };

        let session_config = SessionConfig {
            id: session::Identifier::Name(session_id.clone()),
            working_dir: PathBuf::from(&session_working_dir),
            schedule_id: request.scheduled_job_id.clone(),
            execution_mode: None,
            max_turns: None,
            retry_config: None,
        };

        // Messages will be auto-compacted in agent.reply() if needed
        let messages_to_process = messages.clone();

        let mut stream = match agent
            .reply(
                &messages_to_process,
                Some(session_config),
                Some(task_cancel.clone()),
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
                    &task_tx,
                )
                .await;
                return;
            }
        };

        let mut all_messages = messages.clone();
        let session_path = match session::get_path(session::Identifier::Name(session_id.clone())) {
            Ok(path) => path,
            Err(e) => {
                tracing::error!("Failed to get session path: {}", e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: format!("Failed to get session path: {}", e),
                    },
                    &task_tx,
                )
                .await;
                return;
            }
        };
        let saved_message_count = all_messages.len();

        loop {
            tokio::select! {
                            _ = task_cancel.cancelled() => {
                                tracing::info!("Agent task cancelled");
                                break;
                            }
            response = timeout(Duration::from_millis(500), stream.next()) => {
                                match response {
                                    Ok(Some(Ok(AgentEvent::Message(message)))) => {
                                        push_message(&mut all_messages, message.clone());
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
                                    }
                                    Ok(Some(Ok(AgentEvent::HistoryReplaced(new_messages)))) => {
                                        // Replace the message history with the compacted messages
                                        all_messages = new_messages;
                                        // Note: We don't send this as a stream event since it's an internal operation
                                        // The client will see the compaction notification message that was sent before this event
                                    }
                                    Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                                        if let Err(e) = stream_event(MessageEvent::ModelChange { model, mode }, &tx).await {
                                            tracing::error!("Error sending model change through channel: {}", e);
                                            let _ = stream_event(
                                                MessageEvent::Error {
                                                    error: e.to_string(),
                                                },
                                                &tx,
                                            ).await;
                                        }
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
                                    Err(_) => {
                                        if tx.is_closed() {
                                            break;
                                        }
                                        continue;
                                    }
                                }
                            }
                        }
        }

        if all_messages.len() > saved_message_count {
            if let Ok(provider) = agent.provider().await {
                let provider = Arc::clone(&provider);
                tokio::spawn(async move {
                    if let Err(e) = session::persist_messages(
                        &session_path,
                        &all_messages,
                        Some(provider),
                        Some(PathBuf::from(&session_working_dir)),
                    )
                    .await
                    {
                        tracing::error!("Failed to store session history: {:?}", e);
                    }
                });
            }
        }

        let _ = stream_event(
            MessageEvent::Finish {
                reason: "stop".to_string(),
            },
            &task_tx,
        )
        .await;
    }));
    Ok(SseResponse::new(stream))
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
    raw: Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    tracing::info!(
        "Received tool result request: {}",
        serde_json::to_string_pretty(&raw.0).unwrap()
    );

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

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/reply",
            post(reply_handler).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/confirm", post(confirm_permission))
        .route(
            "/tool_result",
            post(submit_tool_result).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
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

    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> goose::providers::base::ProviderMetadata {
            goose::providers::base::ProviderMetadata::empty()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[rmcp::model::Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }
    }

    mod integration_tests {
        use super::*;
        use axum::{body::Body, http::Request};
        use std::sync::Arc;
        use tower::ServiceExt;

        #[tokio::test]
        async fn test_reply_endpoint() {
            let mock_model_config = ModelConfig::new("test-model").unwrap();
            let mock_provider = Arc::new(MockProvider {
                model_config: mock_model_config,
            });
            let agent = Agent::new();
            let _ = agent.update_provider(mock_provider).await;
            let state = AppState::new(Arc::new(agent), "test-secret".to_string()).await;

            let app = routes(state);

            let request = Request::builder()
                .uri("/reply")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&ChatRequest {
                        messages: vec![Message::user().with_text("test message")],
                        session_id: Some("test-session".to_string()),
                        session_working_dir: "test-working-dir".to_string(),
                        scheduled_job_id: None,
                    })
                    .unwrap(),
                ))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
