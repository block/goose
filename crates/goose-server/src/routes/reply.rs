use crate::state::AppState;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{self, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, Stream};
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use goose::permission::{Permission, PermissionConfirmation};
use goose::session::SessionManager;
use goose::{
    agents::{AgentEvent, SessionConfig},
    permission::permission_confirmation::PrincipalType,
};
use rmcp::model::ServerNotification;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    convert::Infallible,
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

fn track_tool_telemetry(content: &MessageContent, all_messages: &[Message]) {
    match content {
        MessageContent::ToolRequest(tool_request) => {
            if let Ok(tool_call) = &tool_request.tool_call {
                tracing::info!(monotonic_counter.goose.tool_calls = 1,
                    tool_name = %tool_call.name,
                    "Tool call started"
                );
            }
        }
        MessageContent::ToolResponse(tool_response) => {
            let tool_name = all_messages
                .iter()
                .rev()
                .find_map(|msg| {
                    msg.content.iter().find_map(|c| {
                        if let MessageContent::ToolRequest(req) = c {
                            if req.id == tool_response.id {
                                if let Ok(tool_call) = &req.tool_call {
                                    Some(tool_call.name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "unknown".to_string().into());

            let success = tool_response.tool_result.is_ok();
            let result_status = if success { "success" } else { "error" };

            tracing::info!(
                counter.goose.tool_completions = 1,
                tool_name = %tool_name,
                result = %result_status,
                "Tool call completed"
            );
        }
        _ => {}
    }
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ChatRequest {
    messages: Vec<Message>,
    session_id: String,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
    #[serde(default)]
    background_mode: bool,
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
    Ping,
}

async fn stream_event(
    event: MessageEvent,
    tx: &mpsc::Sender<String>,
    cancel_token: &CancellationToken,
    background_mode: bool,
) -> bool {
    let json = serde_json::to_string(&event).unwrap_or_else(|e| {
        format!(
            r#"{{"type":"Error","error":"Failed to serialize event: {}"}}"#,
            e
        )
    });
    if tx.send(format!("data: {}\n\n", json)).await.is_err() {
        if background_mode {
            tracing::info!(
                "client disconnected - continuing agent task in background (ALPHA mode)"
            );
        } else {
            tracing::info!("client disconnected - cancelling agent task (backwards compatibility)");
            cancel_token.cancel();
        }
        return false;
    }
    true
}

#[allow(clippy::too_many_lines)]
#[utoipa::path(
    post,
    path = "/reply",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Streaming response initiated", content_type = "text/event-stream"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn reply(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, StatusCode> {
    let session_start = std::time::Instant::now();

    tracing::info!(
        counter.goose.session_starts = 1,
        session_type = "app",
        interface = "ui",
        "Session started"
    );

    let session_id = request.session_id.clone();

    if let Some(recipe_name) = request.recipe_name.clone() {
        if state.mark_recipe_run_if_absent(&session_id).await {
            let recipe_version = request
                .recipe_version
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            tracing::info!(
                counter.goose.recipe_runs = 1,
                recipe_name = %recipe_name,
                recipe_version = %recipe_version,
                session_type = "app",
                interface = "ui",
                "Recipe execution started"
            );
        }
    }

    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);
    let cancel_token = CancellationToken::new();

    let messages = Conversation::new_unvalidated(request.messages);
    let background_mode = request.background_mode;

    if background_mode {
        let mut tokens = state.session_cancellation_tokens.lock().await;
        tokens.insert(session_id.clone(), cancel_token.clone());
    }

    let task_cancel = cancel_token.clone();
    let task_tx = tx.clone();
    let task_state = state.clone();

    drop(tokio::spawn(async move {
        let agent = match task_state.get_agent(session_id.clone()).await {
            Ok(agent) => agent,
            Err(e) => {
                tracing::error!("Failed to get session agent: {}", e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: format!("Failed to get session agent: {}", e),
                    },
                    &task_tx,
                    &task_cancel,
                    background_mode,
                )
                .await;
                return;
            }
        };

        let session = match SessionManager::get_session(&session_id, false).await {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to read session for {}: {}", session_id, e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: format!("Failed to read session: {}", e),
                    },
                    &task_tx,
                    &task_cancel,
                    background_mode,
                )
                .await;
                return;
            }
        };

        let session_config = SessionConfig {
            id: session_id.clone(),
            working_dir: session.working_dir.clone(),
            schedule_id: session.schedule_id.clone(),
            execution_mode: None,
            max_turns: None,
            retry_config: None,
        };

        let mut stream = match agent
            .reply(
                messages.clone(),
                Some(session_config.clone()),
                Some(task_cancel.clone()),
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {:?}", e);
                stream_event(
                    MessageEvent::Error {
                        error: e.to_string(),
                    },
                    &task_tx,
                    &task_cancel,
                    background_mode,
                )
                .await;
                return;
            }
        };

        let mut all_messages = messages.clone();

        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));
        let mut client_connected = true;
        loop {
            tokio::select! {
                _ = task_cancel.cancelled() => {
                    tracing::info!("Agent task cancelled");
                    break;
                }
                _ = heartbeat_interval.tick() => {
                    if client_connected {
                        client_connected = stream_event(MessageEvent::Ping, &tx, &task_cancel, background_mode).await;
                    }
                }
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            for content in &message.content {
                                track_tool_telemetry(content, all_messages.messages());
                            }

                            all_messages.push(message.clone());

                            // Only send message to client if it's user_visible and client is still connected
                            if message.is_user_visible() && client_connected {
                                client_connected = stream_event(MessageEvent::Message { message }, &tx, &task_cancel, background_mode).await;
                            }
                        }
                        Ok(Some(Ok(AgentEvent::HistoryReplaced(new_messages)))) => {
                            // Replace the message history with the compacted messages
                            all_messages = Conversation::new_unvalidated(new_messages);
                            // Note: We don't send this as a stream event since it's an internal operation
                            // The client will see the compaction notification message that was sent before this event
                        }
                        Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                            if client_connected {
                                client_connected = stream_event(MessageEvent::ModelChange { model, mode }, &tx, &task_cancel, background_mode).await;
                            }
                        }
                        Ok(Some(Ok(AgentEvent::McpNotification((request_id, n))))) => {
                            if client_connected {
                                client_connected = stream_event(MessageEvent::Notification{
                                    request_id: request_id.clone(),
                                    message: n,
                                }, &tx, &task_cancel, background_mode).await;
                            }
                        }

                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            if client_connected {
                                stream_event(
                                    MessageEvent::Error {
                                        error: e.to_string(),
                                    },
                                    &tx,
                                    &task_cancel,
                                    background_mode,
                                ).await;
                            }
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

        let session_duration = session_start.elapsed();

        if let Ok(session) = SessionManager::get_session(&session_id, true).await {
            let total_tokens = session.total_tokens.unwrap_or(0);
            tracing::info!(
                counter.goose.session_completions = 1,
                session_type = "app",
                interface = "ui",
                exit_type = "normal",
                duration_ms = session_duration.as_millis() as u64,
                total_tokens = total_tokens,
                message_count = session.message_count,
                "Session completed"
            );

            tracing::info!(
                counter.goose.session_duration_ms = session_duration.as_millis() as u64,
                session_type = "app",
                interface = "ui",
                "Session duration"
            );

            if total_tokens > 0 {
                tracing::info!(
                    counter.goose.session_tokens = total_tokens,
                    session_type = "app",
                    interface = "ui",
                    "Session tokens"
                );
            }
        } else {
            tracing::info!(
                counter.goose.session_completions = 1,
                session_type = "app",
                interface = "ui",
                exit_type = "normal",
                duration_ms = session_duration.as_millis() as u64,
                total_tokens = 0u64,
                message_count = all_messages.len(),
                "Session completed"
            );

            tracing::info!(
                counter.goose.session_duration_ms = session_duration.as_millis() as u64,
                session_type = "app",
                interface = "ui",
                "Session duration"
            );
        }

        let _ = stream_event(
            MessageEvent::Finish {
                reason: "stop".to_string(),
            },
            &task_tx,
            &task_cancel,
            background_mode,
        )
        .await;

        if background_mode {
            let mut tokens = task_state.session_cancellation_tokens.lock().await;
            tokens.remove(&session_id);
        }
    }));
    Ok(SseResponse::new(stream))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PermissionConfirmationRequest {
    id: String,
    #[serde(default = "default_principal_type")]
    principal_type: PrincipalType,
    action: String,
    session_id: String,
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
    Json(request): Json<PermissionConfirmationRequest>,
) -> Result<Json<Value>, StatusCode> {
    let agent = state.get_agent_for_route(request.session_id).await?;
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CancelRequest {
    session_id: String,
}

#[utoipa::path(
    post,
    path = "/cancel",
    request_body = CancelRequest,
    responses(
        (status = 200, description = "Agent task cancelled successfully"),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn cancel_agent(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CancelRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::info!("Cancelling agent task for session: {}", request.session_id);

    // Get the cancellation token for this session and trigger it
    let mut tokens = state.session_cancellation_tokens.lock().await;
    if let Some(cancel_token) = tokens.remove(&request.session_id) {
        cancel_token.cancel();
        tracing::info!("Agent task cancelled for session: {}", request.session_id);
        Ok(Json(serde_json::json!({
            "status": "cancelled",
            "session_id": request.session_id
        })))
    } else {
        tracing::warn!(
            "No active cancellation token found for session: {}",
            request.session_id
        );
        // Still return success - the session may have already completed
        Ok(Json(serde_json::json!({
            "status": "not_active",
            "session_id": request.session_id
        })))
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/reply",
            post(reply).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/confirm", post(confirm_permission))
        .route("/cancel", post(cancel_agent))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod integration_tests {
        use super::*;
        use axum::{body::Body, http::Request};
        use goose::conversation::message::Message;
        use tower::ServiceExt;

        #[tokio::test(flavor = "multi_thread")]
        async fn test_reply_endpoint() {
            let state = AppState::new().await.unwrap();

            let app = routes(state);

            let request = Request::builder()
                .uri("/reply")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&ChatRequest {
                        messages: vec![Message::user().with_text("test message")],
                        session_id: "test-session".to_string(),
                        recipe_name: None,
                        recipe_version: None,
                        background_mode: false,
                    })
                    .unwrap(),
                ))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
