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
    message::{push_message, Message, MessageContent},
    permission::permission_confirmation::PrincipalType,
    telemetry::{
        global_telemetry, SessionExecution, SessionMetadataSupport, SessionResult, SessionType,
        TelemetryExecution,
    },
};
use goose::{
    permission::{Permission, PermissionConfirmation},
    session,
};
use mcp_core::{protocol::JsonRpcMessage, ToolResult};
use rmcp::model::{Content, Role};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::{
    convert::Infallible,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use utoipa::ToSchema;

fn create_session_execution(
    session_id: &str,
    session_type: &str,
    recipe_name: Option<&str>,
    recipe_version: Option<&str>,
) -> SessionExecution {
    let mut session_execution = SessionExecution::new(session_id, SessionType::Interactive)
        .with_metadata("execution_mode", "server")
        .with_metadata("session_type", session_type)
        .with_metadata("interface", "ui");

    if let Some(recipe_name) = recipe_name {
        session_execution = session_execution
            .with_metadata("recipe_name", recipe_name)
            .with_metadata("session_mode", "recipe");
    }
    if let Some(recipe_version) = recipe_version {
        session_execution = session_execution.with_metadata("recipe_version", recipe_version);
    }

    if recipe_name.is_none() {
        session_execution = session_execution.with_metadata("session_mode", "chat");
    }

    session_execution
}

async fn track_failed_session(
    session_execution: SessionExecution,
    error_message: String,
    start_time: Instant,
    message_count: Option<u64>,
    turn_count: Option<u64>,
) {
    if let Some(manager) = global_telemetry() {
        let mut failed_execution = session_execution
            .with_result(SessionResult::Error(error_message))
            .with_duration(start_time.elapsed());

        if let Some(count) = message_count {
            failed_execution = failed_execution.with_message_count(count);
        }
        if let Some(count) = turn_count {
            failed_execution = failed_execution.with_turn_count(count);
        }

        let _ = manager.track_session_execution(failed_execution).await;
    } else {
        tracing::warn!(
            "Telemetry is disabled or not initialized - failed to track session failure"
        );
    }
}

async fn track_successful_session(
    session_execution: SessionExecution,
    start_time: Instant,
    message_count: u64,
    turn_count: u64,
) {
    if let Some(manager) = global_telemetry() {
        let successful_execution = session_execution
            .with_result(SessionResult::Success)
            .with_message_count(message_count)
            .with_turn_count(turn_count)
            .with_duration(start_time.elapsed());
        let _ = manager.track_session_execution(successful_execution).await;
    }
}

async fn track_recipe_execution(
    recipe_name: &str,
    recipe_version: &str,
    result: SessionResult,
    start_time: Instant,
    session_type: &str,
) {
    if let Some(manager) = global_telemetry() {
        let recipe_execution = manager
            .recipe_execution(recipe_name, recipe_version)
            .with_result(result)
            .with_duration(start_time.elapsed())
            .with_metadata("interface", "ui")
            .with_metadata("execution_mode", "server")
            .with_metadata("session_type", session_type);

        let _ = manager
            .track_recipe_execution(recipe_execution.build())
            .await;
    }
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    messages: Vec<Message>,
    session_id: Option<String>,
    session_working_dir: String,
    scheduled_job_id: Option<String>,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
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
        message: JsonRpcMessage,
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

async fn handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    let messages = request.messages;
    let session_working_dir = request.session_working_dir.clone();

    let session_id = request
        .session_id
        .unwrap_or_else(session::generate_session_id);

    tokio::spawn(async move {
        let start_time = Instant::now();
        let mut session_execution = create_session_execution(
            &session_id,
            "streaming",
            request.recipe_name.as_deref(),
            request.recipe_version.as_deref(),
        );

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

                        // Track failed session and recipe execution
                        track_failed_session(
                            session_execution.clone(),
                            "No provider configured".to_string(),
                            start_time,
                            None,
                            None,
                        )
                        .await;

                        if let (Some(recipe_name), Some(recipe_version)) =
                            (&request.recipe_name, &request.recipe_version)
                        {
                            track_recipe_execution(
                                recipe_name,
                                recipe_version,
                                SessionResult::Error("No provider configured".to_string()),
                                start_time,
                                "streaming",
                            )
                            .await;
                        }
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

                // Track failed session
                track_failed_session(
                    session_execution.clone(),
                    "No agent configured".to_string(),
                    start_time,
                    None,
                    None,
                )
                .await;
                return;
            }
        };

        let provider = agent.provider().await;

        let mut stream = match agent
            .reply(
                &messages,
                Some(SessionConfig {
                    id: session::Identifier::Name(session_id.clone()),
                    working_dir: PathBuf::from(&session_working_dir),
                    schedule_id: request.scheduled_job_id.clone(),
                    execution_mode: None,
                    max_turns: None,
                    retry_config: None,
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

                // Track failed session
                track_failed_session(
                    session_execution.clone(),
                    e.to_string(),
                    start_time,
                    None,
                    None,
                )
                .await;
                return;
            }
        };

        let mut all_messages = messages.clone();
        let mut message_count = messages.len();
        let mut turn_count = 0;

        let session_path = match session::get_path(session::Identifier::Name(session_id.clone())) {
            Ok(path) => path,
            Err(e) => {
                tracing::error!("Failed to get session path: {}", e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: format!("Failed to get session path: {}", e),
                    },
                    &tx,
                )
                .await;

                // Track failed session
                track_failed_session(
                    session_execution.clone(),
                    format!("Failed to get session path: {}", e),
                    start_time,
                    None,
                    None,
                )
                .await;
                return;
            }
        };
        let saved_message_count = all_messages.len();

        loop {
            tokio::select! {
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            push_message(&mut all_messages, message.clone());
                            message_count += 1;
                            if message.role == Role::Assistant {
                                turn_count += 1;
                            }
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
                        Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                            session_execution = session_execution.with_metadata("model", &model);

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

                            // Track failed session
                            track_failed_session(
                                session_execution.clone(),
                                e.to_string(),
                                start_time,
                                Some(message_count as u64),
                                Some(turn_count as u64),
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

        if all_messages.len() > saved_message_count {
            let provider = Arc::clone(provider.as_ref().unwrap());
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

        let _ = stream_event(
            MessageEvent::Finish {
                reason: "stop".to_string(),
            },
            &tx,
        )
        .await;

        // Track successful session and recipe execution
        track_successful_session(
            session_execution.clone(),
            start_time,
            message_count as u64,
            turn_count as u64,
        )
        .await;

        if let (Some(recipe_name), Some(recipe_version)) =
            (&request.recipe_name, &request.recipe_version)
        {
            track_recipe_execution(
                recipe_name,
                recipe_version,
                SessionResult::Success,
                start_time,
                "streaming",
            )
            .await;
        }
    });

    Ok(SseResponse::new(stream))
}

#[derive(Debug, Deserialize, Serialize)]
struct AskRequest {
    prompt: String,
    session_id: Option<String>,
    session_working_dir: String,
    scheduled_job_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct AskResponse {
    response: String,
}

async fn ask_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<AskRequest>,
) -> Result<Json<AskResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let start_time = Instant::now();
    let session_working_dir = request.session_working_dir.clone();

    let session_id = request
        .session_id
        .unwrap_or_else(session::generate_session_id);

    let mut session_execution = create_session_execution(&session_id, "ask", None, None);

    let agent = state
        .get_agent()
        .await
        .map_err(|_| StatusCode::PRECONDITION_FAILED)?;

    let provider = agent.provider().await;

    let messages = vec![Message::user().with_text(request.prompt)];

    let mut response_text = String::new();
    let mut stream = match agent
        .reply(
            &messages,
            Some(SessionConfig {
                id: session::Identifier::Name(session_id.clone()),
                working_dir: PathBuf::from(&session_working_dir),
                schedule_id: request.scheduled_job_id.clone(),
                execution_mode: None,
                max_turns: None,
                retry_config: None,
            }),
        )
        .await
    {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Failed to start reply stream: {:?}", e);

            // Track failed session
            track_failed_session(session_execution, e.to_string(), start_time, None, None).await;

            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut all_messages = messages.clone();
    let mut response_message = Message::assistant();
    let mut message_count = messages.len();
    let mut turn_count = 0;

    while let Some(response) = stream.next().await {
        match response {
            Ok(AgentEvent::Message(message)) => {
                if message.role == Role::Assistant {
                    turn_count += 1;
                    for content in &message.content {
                        if let MessageContent::Text(text) = content {
                            response_text.push_str(&text.text);
                            response_text.push('\n');
                        }
                        response_message.content.push(content.clone());
                    }
                }
            }
            Ok(AgentEvent::ModelChange { model, mode }) => {
                session_execution = session_execution.with_metadata("model", &model);
                // Log model change for non-streaming
                tracing::info!("Model changed to {} in {} mode", model, mode);
            }
            Ok(AgentEvent::McpNotification(n)) => {
                // Handle notifications if needed
                tracing::info!("Received notification: {:?}", n);
            }

            Err(e) => {
                tracing::error!("Error processing as_ai message: {}", e);

                // Track failed session
                track_failed_session(
                    session_execution,
                    e.to_string(),
                    start_time,
                    Some(message_count as u64),
                    Some(turn_count as u64),
                )
                .await;

                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    if !response_message.content.is_empty() {
        push_message(&mut all_messages, response_message);
        message_count += 1;
    }

    let session_path = match session::get_path(session::Identifier::Name(session_id.clone())) {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Failed to get session path: {}", e);

            // Track failed session
            track_failed_session(
                session_execution,
                format!("Failed to get session path: {}", e),
                start_time,
                Some(message_count as u64),
                Some(turn_count as u64),
            )
            .await;

            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let session_path_clone = session_path.clone();
    let messages = all_messages.clone();
    let provider = Arc::clone(provider.as_ref().unwrap());
    let session_working_dir_clone = session_working_dir.clone();
    tokio::spawn(async move {
        if let Err(e) = session::persist_messages(
            &session_path_clone,
            &messages,
            Some(provider),
            Some(PathBuf::from(session_working_dir_clone)),
        )
        .await
        {
            tracing::error!("Failed to store session history: {:?}", e);
        }
    });

    // Track successful session
    track_successful_session(
        session_execution,
        start_time,
        message_count as u64,
        turn_count as u64,
    )
    .await;

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

        #[tokio::test]
        async fn test_ask_endpoint() {
            let mock_model_config = ModelConfig::new("test-model".to_string());
            let mock_provider = Arc::new(MockProvider {
                model_config: mock_model_config,
            });
            let agent = Agent::new();
            let _ = agent.update_provider(mock_provider).await;
            let state = AppState::new(Arc::new(agent), "test-secret".to_string()).await;
            let scheduler_path = goose::scheduler::get_default_scheduler_storage_path()
                .expect("Failed to get default scheduler storage path");
            let scheduler =
                goose::scheduler_factory::SchedulerFactory::create_legacy(scheduler_path)
                    .await
                    .unwrap();
            state.set_scheduler(scheduler).await;

            let app = routes(state);

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
