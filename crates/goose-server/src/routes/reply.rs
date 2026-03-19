use crate::routes::errors::ErrorResponse;
use crate::session_event_bus::SessionEventBus;
use crate::state::AppState;
#[cfg(test)]
use axum::http::StatusCode;
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::{self, HeaderMap},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use futures::{stream::StreamExt, Stream};
use goose::agents::{AgentEvent, SessionConfig};
use goose::conversation::message::{Message, MessageContent, TokenState};
use goose::conversation::Conversation;
use goose::session::SessionManager;
use rmcp::model::ServerNotification;
use serde::{Deserialize, Serialize};
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

// ── Types ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ChatRequest {
    user_message: Message,
    #[serde(default)]
    override_conversation: Option<Vec<Message>>,
    session_id: String,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct StreamReplyRequest {
    pub request_id: String,
    pub user_message: Message,
    #[serde(default)]
    pub override_conversation: Option<Vec<Message>>,
    pub recipe_name: Option<String>,
    pub recipe_version: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StreamReplyResponse {
    pub request_id: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CancelRequest {
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(tag = "type")]
pub enum MessageEvent {
    Message {
        message: Message,
        token_state: TokenState,
    },
    Error {
        error: String,
    },
    Finish {
        reason: String,
        token_state: TokenState,
    },
    ModelChange {
        model: String,
        mode: String,
    },
    Notification {
        request_id: String,
        #[schema(value_type = Object)]
        message: ServerNotification,
    },
    UpdateConversation {
        conversation: Conversation,
    },
    ActiveRequests {
        request_ids: Vec<String>,
    },
    Ping,
}

// ── SSE response types ──────────────────────────────────────────────────

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
        let body = axum::body::Body::from_stream(self);
        http::Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap()
    }
}

// ── Shared agent execution ──────────────────────────────────────────────

pub fn track_tool_telemetry(content: &MessageContent, all_messages: &[Message]) {
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

            let result_status = if tool_response.tool_result.is_ok() {
                "success"
            } else {
                "error"
            };

            tracing::info!(
                monotonic_counter.goose.tool_completions = 1,
                tool_name = %tool_name,
                result = %result_status,
                "Tool call completed"
            );
        }
        _ => {}
    }
}

pub async fn get_token_state(session_manager: &SessionManager, session_id: &str) -> TokenState {
    session_manager
        .get_session(session_id, false)
        .await
        .map(|session| TokenState {
            input_tokens: session.input_tokens.unwrap_or(0),
            output_tokens: session.output_tokens.unwrap_or(0),
            total_tokens: session.total_tokens.unwrap_or(0),
            accumulated_input_tokens: session.accumulated_input_tokens.unwrap_or(0),
            accumulated_output_tokens: session.accumulated_output_tokens.unwrap_or(0),
            accumulated_total_tokens: session.accumulated_total_tokens.unwrap_or(0),
        })
        .inspect_err(|e| {
            tracing::warn!(
                "Failed to fetch session token state for {}: {}",
                session_id,
                e
            );
        })
        .unwrap_or_default()
}

/// Spawn the agent reply loop, publishing events to the given bus.
/// This is the single place where agent execution happens — both the legacy
/// `POST /reply` and the new `POST /reply/stream/{id}` use this.
#[allow(clippy::too_many_arguments)]
fn spawn_reply_task(
    state: Arc<AppState>,
    session_id: String,
    request_id: String,
    user_message: Message,
    override_conversation: Option<Vec<Message>>,
    bus: Arc<SessionEventBus>,
    cancel_token: CancellationToken,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
) {
    let session_start = std::time::Instant::now();

    tracing::info!(
        monotonic_counter.goose.session_starts = 1,
        session_type = "app",
        interface = "ui",
        "Session started"
    );

    if let Some(ref recipe_name) = recipe_name {
        let session_id = session_id.clone();
        let recipe_name = recipe_name.clone();
        let recipe_version = recipe_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let state = state.clone();
        tokio::spawn(async move {
            if state.mark_recipe_run_if_absent(&session_id).await {
                tracing::info!(
                    monotonic_counter.goose.recipe_runs = 1,
                    recipe_name = %recipe_name,
                    recipe_version = %recipe_version,
                    session_type = "app",
                    interface = "ui",
                    "Recipe execution started"
                );
            }
        });
    }

    let task_bus = bus.clone();
    let task_request_id = request_id.clone();

    drop(tokio::spawn(async move {
        let publish = |event: MessageEvent| {
            let bus = task_bus.clone();
            let rid = task_request_id.clone();
            async move {
                bus.publish(Some(rid), event).await;
            }
        };

        let agent = match state.get_agent(session_id.clone()).await {
            Ok(agent) => agent,
            Err(e) => {
                tracing::error!("Failed to get session agent: {}", e);
                publish(MessageEvent::Error {
                    error: format!("Failed to get session agent: {}", e),
                })
                .await;
                task_bus.cleanup_request(&task_request_id).await;
                return;
            }
        };

        let session = match state.session_manager().get_session(&session_id, true).await {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to read session for {}: {}", session_id, e);
                publish(MessageEvent::Error {
                    error: format!("Failed to read session: {}", e),
                })
                .await;
                task_bus.cleanup_request(&task_request_id).await;
                return;
            }
        };

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: session.schedule_id.clone(),
            max_turns: None,
            retry_config: None,
        };

        let mut all_messages = match override_conversation {
            Some(history) => {
                let conv = Conversation::new_unvalidated(history);
                if let Err(e) = state
                    .session_manager()
                    .replace_conversation(&session_id, &conv)
                    .await
                {
                    tracing::warn!(
                        "Failed to replace session conversation for {}: {}",
                        session_id,
                        e
                    );
                }
                conv
            }
            None => session.conversation.unwrap_or_default(),
        };
        all_messages.push(user_message.clone());

        let mut stream = match agent
            .reply(user_message, session_config, Some(cancel_token.clone()))
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {:?}", e);
                publish(MessageEvent::Error {
                    error: e.to_string(),
                })
                .await;
                task_bus.cleanup_request(&task_request_id).await;
                return;
            }
        };

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!("Agent task cancelled for request {}", task_request_id);
                    break;
                }
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            for content in &message.content {
                                track_tool_telemetry(content, all_messages.messages());
                            }
                            all_messages.push(message.clone());
                            let token_state = get_token_state(state.session_manager(), &session_id).await;
                            publish(MessageEvent::Message { message, token_state }).await;
                        }
                        Ok(Some(Ok(AgentEvent::HistoryReplaced(new_messages)))) => {
                            all_messages = new_messages.clone();
                            publish(MessageEvent::UpdateConversation { conversation: new_messages }).await;
                        }
                        Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                            publish(MessageEvent::ModelChange { model, mode }).await;
                        }
                        Ok(Some(Ok(AgentEvent::McpNotification((notification_request_id, n))))) => {
                            publish(MessageEvent::Notification {
                                request_id: notification_request_id,
                                message: n,
                            }).await;
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            publish(MessageEvent::Error { error: e.to_string() }).await;
                            break;
                        }
                        Ok(None) => break,
                        Err(_) => continue,
                    }
                }
            }
        }

        // Telemetry
        let session_duration = session_start.elapsed();
        if let Ok(session) = state.session_manager().get_session(&session_id, true).await {
            let total_tokens = session.total_tokens.unwrap_or(0);
            tracing::info!(
                monotonic_counter.goose.session_completions = 1,
                session_type = "app",
                interface = "ui",
                exit_type = "normal",
                duration_ms = session_duration.as_millis() as u64,
                total_tokens = total_tokens,
                message_count = session.message_count,
                "Session completed"
            );
            tracing::info!(
                monotonic_counter.goose.session_duration_ms = session_duration.as_millis() as u64,
                session_type = "app",
                interface = "ui",
                "Session duration"
            );
            if total_tokens > 0 {
                tracing::info!(
                    monotonic_counter.goose.session_tokens = total_tokens,
                    session_type = "app",
                    interface = "ui",
                    "Session tokens"
                );
            }
        } else {
            tracing::info!(
                monotonic_counter.goose.session_completions = 1,
                session_type = "app",
                interface = "ui",
                exit_type = "normal",
                duration_ms = session_duration.as_millis() as u64,
                total_tokens = 0u64,
                message_count = all_messages.len(),
                "Session completed"
            );
            tracing::info!(
                monotonic_counter.goose.session_duration_ms = session_duration.as_millis() as u64,
                session_type = "app",
                interface = "ui",
                "Session duration"
            );
        }

        let final_token_state = get_token_state(state.session_manager(), &session_id).await;
        publish(MessageEvent::Finish {
            reason: "stop".to_string(),
            token_state: final_token_state,
        })
        .await;

        task_bus.cleanup_request(&task_request_id).await;
    }));
}

// ── SSE helpers ─────────────────────────────────────────────────────────

fn format_sse_event(seq: u64, json: &str) -> String {
    format!("id: {}\ndata: {}\n\n", seq, json)
}

fn serialize_session_event(seq: u64, request_id: Option<&str>, event: &MessageEvent) -> String {
    let mut event_json = serde_json::to_value(event).unwrap_or_else(
        |e| serde_json::json!({"type": "Error", "error": format!("Serialization error: {}", e)}),
    );

    if let Some(rid) = request_id {
        if let serde_json::Value::Object(ref mut map) = event_json {
            map.insert(
                "chat_request_id".to_string(),
                serde_json::Value::String(rid.to_string()),
            );
            map.entry("request_id")
                .or_insert_with(|| serde_json::Value::String(rid.to_string()));
        }
    }

    let json_str = serde_json::to_string(&event_json).unwrap_or_default();
    format_sse_event(seq, &json_str)
}

async fn stream_message_event(
    event: MessageEvent,
    tx: &mpsc::Sender<String>,
    cancel_token: &CancellationToken,
) {
    let json = serde_json::to_string(&event).unwrap_or_else(|e| {
        format!(
            r#"{{"type":"Error","error":"Failed to serialize event: {}"}}"#,
            e
        )
    });

    if tx.send(format!("data: {}\n\n", json)).await.is_err() {
        tracing::info!("client hung up");
        cancel_token.cancel();
    }
}

// ── POST /reply (legacy inline SSE) ────────────────────────────────────

#[allow(clippy::too_many_lines)]
#[utoipa::path(
    post,
    path = "/reply",
    request_body = ChatRequest,
    responses(
        (status = 200, description = "Streaming response initiated",
         body = MessageEvent,
         content_type = "text/event-stream"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn reply(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, ErrorResponse> {
    let session_id = request.session_id.clone();
    let request_id = uuid::Uuid::new_v4().to_string();

    let bus = state.get_or_create_event_bus(&session_id).await;
    let cancel_token = bus.register_request(request_id.clone()).await;

    spawn_reply_task(
        state,
        session_id,
        request_id.clone(),
        request.user_message,
        request.override_conversation,
        bus.clone(),
        cancel_token.clone(),
        request.recipe_name,
        request.recipe_version,
    );

    // Subscribe to the bus and pipe events back as the SSE response
    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);

    tokio::spawn(async move {
        let (replay, replay_max_seq, mut live_rx) = match bus.subscribe(None).await {
            Ok(result) => result,
            Err(_) => return,
        };

        for event in &replay {
            let json = serde_json::to_string(&event.event).unwrap_or_default();
            if tx.send(format!("data: {}\n\n", json)).await.is_err() {
                return;
            }
        }

        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    // Drain any remaining events after cancellation
                    while let Ok(event) = live_rx.try_recv() {
                        if event.seq <= replay_max_seq { continue; }
                        let json = serde_json::to_string(&event.event).unwrap_or_default();
                        if tx.send(format!("data: {}\n\n", json)).await.is_err() { return; }
                    }
                    break;
                }
                _ = heartbeat_interval.tick() => {
                    stream_message_event(MessageEvent::Ping, &tx, &cancel_token).await;
                }
                result = live_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if event.seq <= replay_max_seq { continue; }
                            let json = serde_json::to_string(&event.event).unwrap_or_default();
                            if tx.send(format!("data: {}\n\n", json)).await.is_err() { return; }
                            if matches!(event.event, MessageEvent::Finish { .. } | MessageEvent::Error { .. }) {
                                return;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        }
    });

    Ok(SseResponse::new(stream))
}

// ── GET /reply/stream/{session_id} ──────────────────────────────────────

#[utoipa::path(
    get,
    path = "/reply/stream/{session_id}",
    params(
        ("session_id" = String, Path, description = "Session ID"),
    ),
    responses(
        (status = 200, description = "SSE event stream",
         body = MessageEvent,
         content_type = "text/event-stream"),
        (status = 404, description = "Session not found"),
    )
)]
pub async fn stream_events(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<SseResponse, axum::http::StatusCode> {
    state
        .session_manager()
        .get_session(&session_id, false)
        .await
        .map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

    let last_event_id: Option<u64> = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());

    let bus = state.get_or_create_event_bus(&session_id).await;

    let (replay, replay_max_seq, mut live_rx) = match bus.subscribe(last_event_id).await {
        Ok(result) => result,
        Err(_) => {
            let (tx, rx) = mpsc::channel::<String>(1);
            let stream = ReceiverStream::new(rx);
            let error_event = MessageEvent::Error {
                error: "Client too far behind — reload conversation".to_string(),
            };
            let frame = serialize_session_event(0, None, &error_event);
            tokio::spawn(async move {
                let _ = tx.send(frame).await;
            });
            return Ok(SseResponse::new(stream));
        }
    };

    let (tx, rx) = mpsc::channel::<String>(256);
    let stream = ReceiverStream::new(rx);

    tokio::spawn(async move {
        // Send active request IDs before replay so the client can register handlers
        let active_ids = bus.active_request_ids().await;
        if !active_ids.is_empty() {
            let event = MessageEvent::ActiveRequests {
                request_ids: active_ids,
            };
            let json_str = serde_json::to_string(&serde_json::to_value(&event).unwrap_or_default())
                .unwrap_or_default();
            let frame = format!("data: {}\n\n", json_str);
            if tx.send(frame).await.is_err() {
                return;
            }
        }

        for event in &replay {
            let frame =
                serialize_session_event(event.seq, event.request_id.as_deref(), &event.event);
            if tx.send(frame).await.is_err() {
                return;
            }
        }

        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));
        let mut heartbeat_seq = 0u64;

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    let frame = format!(": ping {}\n\n", heartbeat_seq);
                    heartbeat_seq += 1;
                    if tx.send(frame).await.is_err() { return; }
                }
                result = live_rx.recv() => {
                    match result {
                        Ok(event) => {
                            if event.seq <= replay_max_seq { continue; }
                            let frame = serialize_session_event(
                                event.seq,
                                event.request_id.as_deref(),
                                &event.event,
                            );
                            if tx.send(frame).await.is_err() { return; }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("SSE subscriber lagged by {} events, closing stream so client reconnects with Last-Event-ID", n);
                            return;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        }
    });

    Ok(SseResponse::new(stream))
}

// ── POST /reply/stream/{session_id} ─────────────────────────────────────

#[utoipa::path(
    post,
    path = "/reply/stream/{session_id}",
    params(
        ("session_id" = String, Path, description = "Session ID"),
    ),
    request_body = StreamReplyRequest,
    responses(
        (status = 200, description = "Request accepted", body = StreamReplyResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Session not found"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn stream_reply(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<StreamReplyRequest>,
) -> Result<Json<StreamReplyResponse>, ErrorResponse> {
    let request_id = request.request_id.clone();

    if uuid::Uuid::parse_str(&request_id).is_err() {
        return Err(ErrorResponse::bad_request(
            "request_id must be a valid UUID",
        ));
    }

    state
        .session_manager()
        .get_session(&session_id, false)
        .await
        .map_err(|_| ErrorResponse::not_found(format!("Session {} not found", session_id)))?;

    let bus = state.get_or_create_event_bus(&session_id).await;
    let cancel_token = bus.register_request(request_id.clone()).await;

    spawn_reply_task(
        state,
        session_id,
        request_id.clone(),
        request.user_message,
        request.override_conversation,
        bus,
        cancel_token,
        request.recipe_name,
        request.recipe_version,
    );

    Ok(Json(StreamReplyResponse { request_id }))
}

// ── POST /reply/stream/{session_id}/cancel ──────────────────────────────

#[utoipa::path(
    post,
    path = "/reply/stream/{session_id}/cancel",
    params(
        ("session_id" = String, Path, description = "Session ID"),
    ),
    request_body = CancelRequest,
    responses(
        (status = 200, description = "Cancellation accepted"),
    )
)]
pub async fn stream_cancel(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<CancelRequest>,
) -> axum::http::StatusCode {
    let bus = match state.get_event_bus(&session_id).await {
        Some(bus) => bus,
        None => return axum::http::StatusCode::NOT_FOUND,
    };
    bus.cancel_request(&request.request_id).await;
    axum::http::StatusCode::OK
}

// ── Routes ──────────────────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/reply",
            post(reply).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/reply/stream/{session_id}", get(stream_events))
        .route(
            "/reply/stream/{session_id}",
            post(stream_reply).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/reply/stream/{session_id}/cancel", post(stream_cancel))
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
            let state = AppState::new(true).await.unwrap();

            let app = routes(state);

            let request = Request::builder()
                .uri("/reply")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&ChatRequest {
                        user_message: Message::user().with_text("test message"),
                        override_conversation: None,
                        session_id: "test-session".to_string(),
                        recipe_name: None,
                        recipe_version: None,
                    })
                    .unwrap(),
                ))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
