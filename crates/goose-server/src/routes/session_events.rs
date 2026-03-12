use crate::routes::errors::ErrorResponse;
use crate::routes::reply::{get_token_state, track_tool_telemetry, MessageEvent};
use crate::state::AppState;
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
use goose::conversation::message::Message;
use goose::conversation::Conversation;
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

// ── Request / Response types ────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SessionReplyRequest {
    /// Client-generated UUIDv7 identifying this request.
    pub request_id: String,
    pub user_message: Message,
    #[serde(default)]
    pub override_conversation: Option<Vec<Message>>,
    pub recipe_name: Option<String>,
    pub recipe_version: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionReplyResponse {
    pub request_id: String,
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct CancelRequest {
    pub request_id: String,
}

// ── SSE Event Stream Response ───────────────────────────────────────────

/// An SSE response that includes `id:` lines for Last-Event-ID reconnection.
pub struct SseEventStream {
    rx: ReceiverStream<String>,
}

impl SseEventStream {
    fn new(rx: ReceiverStream<String>) -> Self {
        Self { rx }
    }
}

impl Stream for SseEventStream {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx)
            .poll_next(cx)
            .map(|opt| opt.map(|s| Ok(Bytes::from(s))))
    }
}

impl IntoResponse for SseEventStream {
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

// ── Helpers ─────────────────────────────────────────────────────────────

fn format_sse_event(seq: u64, json: &str) -> String {
    format!("id: {}\ndata: {}\n\n", seq, json)
}

fn serialize_session_event(seq: u64, request_id: Option<&str>, event: &MessageEvent) -> String {
    // Build JSON payload: { request_id?: string, ...event_fields }
    // We flatten request_id into the event JSON.
    let mut event_json = serde_json::to_value(event).unwrap_or_else(
        |e| serde_json::json!({"type": "Error", "error": format!("Serialization error: {}", e)}),
    );

    if let Some(rid) = request_id {
        if let serde_json::Value::Object(ref mut map) = event_json {
            map.insert(
                "request_id".to_string(),
                serde_json::Value::String(rid.to_string()),
            );
        }
    }

    let json_str = serde_json::to_string(&event_json).unwrap_or_default();
    format_sse_event(seq, &json_str)
}

// ── GET /sessions/{id}/events ───────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/sessions/{id}/events",
    params(
        ("id" = String, Path, description = "Session ID"),
    ),
    responses(
        (status = 200, description = "SSE event stream",
         body = MessageEvent,
         content_type = "text/event-stream"),
    )
)]
pub async fn session_events(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> SseEventStream {
    let last_event_id: Option<u64> = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());

    let bus = state.get_or_create_event_bus(&session_id).await;
    let (replay, mut live_rx) = bus.subscribe(last_event_id).await;

    let (tx, rx) = mpsc::channel::<String>(256);
    let stream = ReceiverStream::new(rx);

    tokio::spawn(async move {
        // Send replayed events
        for event in &replay {
            let frame =
                serialize_session_event(event.seq, event.request_id.as_deref(), &event.event);
            if tx.send(frame).await.is_err() {
                return;
            }
        }

        // Send live events + heartbeat pings
        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));

        loop {
            tokio::select! {
                _ = heartbeat_interval.tick() => {
                    // Publish a ping through the bus so it gets a sequence number
                    let seq = bus.publish(None, MessageEvent::Ping).await;
                    let frame = serialize_session_event(seq, None, &MessageEvent::Ping);
                    if tx.send(frame).await.is_err() {
                        return;
                    }
                }
                result = live_rx.recv() => {
                    match result {
                        Ok(event) => {
                            // Skip Ping events from the broadcast — we generate our own
                            if matches!(event.event, MessageEvent::Ping) {
                                continue;
                            }
                            let frame = serialize_session_event(
                                event.seq,
                                event.request_id.as_deref(),
                                &event.event,
                            );
                            if tx.send(frame).await.is_err() {
                                return;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("SSE subscriber lagged by {} events", n);
                            // Continue — client will reconnect if needed
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            return;
                        }
                    }
                }
            }
        }
    });

    SseEventStream::new(stream)
}

// ── POST /sessions/{id}/reply ───────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/sessions/{id}/reply",
    params(
        ("id" = String, Path, description = "Session ID"),
    ),
    request_body = SessionReplyRequest,
    responses(
        (status = 200, description = "Request accepted",
         body = SessionReplyResponse),
        (status = 400, description = "Invalid request"),
        (status = 424, description = "Agent not initialized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn session_reply(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<SessionReplyRequest>,
) -> Result<Json<SessionReplyResponse>, ErrorResponse> {
    let request_id = request.request_id.clone();

    // Validate request_id is a valid UUID
    if uuid::Uuid::parse_str(&request_id).is_err() {
        return Err(ErrorResponse::bad_request(
            "request_id must be a valid UUID",
        ));
    }

    let session_start = std::time::Instant::now();

    tracing::info!(
        monotonic_counter.goose.session_starts = 1,
        session_type = "app",
        interface = "ui",
        "Session started"
    );

    if let Some(recipe_name) = request.recipe_name.clone() {
        if state.mark_recipe_run_if_absent(&session_id).await {
            let recipe_version = request
                .recipe_version
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            tracing::info!(
                monotonic_counter.goose.recipe_runs = 1,
                recipe_name = %recipe_name,
                recipe_version = %recipe_version,
                session_type = "app",
                interface = "ui",
                "Recipe execution started"
            );
        }
    }

    let bus = state.get_or_create_event_bus(&session_id).await;
    let cancel_token = bus.register_request(request_id.clone()).await;

    let user_message = request.user_message;
    let override_conversation = request.override_conversation;

    let task_state = state.clone();
    let task_session_id = session_id.clone();
    let task_request_id = request_id.clone();
    let task_cancel = cancel_token.clone();
    let task_bus = bus.clone();

    drop(tokio::spawn(async move {
        let publish = |rid: Option<String>, event: MessageEvent| {
            let bus = task_bus.clone();
            async move {
                bus.publish(rid, event).await;
            }
        };

        let agent = match task_state.get_agent(task_session_id.clone()).await {
            Ok(agent) => agent,
            Err(e) => {
                tracing::error!("Failed to get session agent: {}", e);
                publish(
                    Some(task_request_id.clone()),
                    MessageEvent::Error {
                        error: format!("Failed to get session agent: {}", e),
                    },
                )
                .await;
                task_bus.cleanup_request(&task_request_id).await;
                return;
            }
        };

        let session = match task_state
            .session_manager()
            .get_session(&task_session_id, true)
            .await
        {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to read session for {}: {}", task_session_id, e);
                publish(
                    Some(task_request_id.clone()),
                    MessageEvent::Error {
                        error: format!("Failed to read session: {}", e),
                    },
                )
                .await;
                task_bus.cleanup_request(&task_request_id).await;
                return;
            }
        };

        let session_config = SessionConfig {
            id: task_session_id.clone(),
            schedule_id: session.schedule_id.clone(),
            max_turns: None,
            retry_config: None,
        };

        let mut all_messages = match override_conversation {
            Some(history) => {
                let conv = Conversation::new_unvalidated(history);
                if let Err(e) = task_state
                    .session_manager()
                    .replace_conversation(&task_session_id, &conv)
                    .await
                {
                    tracing::warn!(
                        "Failed to replace session conversation for {}: {}",
                        task_session_id,
                        e
                    );
                }
                conv
            }
            None => session.conversation.unwrap_or_default(),
        };
        all_messages.push(user_message.clone());

        let mut stream = match agent
            .reply(
                user_message.clone(),
                session_config,
                Some(task_cancel.clone()),
            )
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                tracing::error!("Failed to start reply stream: {:?}", e);
                publish(
                    Some(task_request_id.clone()),
                    MessageEvent::Error {
                        error: e.to_string(),
                    },
                )
                .await;
                task_bus.cleanup_request(&task_request_id).await;
                return;
            }
        };

        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = task_cancel.cancelled() => {
                    tracing::info!("Agent task cancelled for request {}", task_request_id);
                    break;
                }
                _ = heartbeat_interval.tick() => {
                    // Heartbeat is handled by the SSE event stream endpoint
                }
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            for content in &message.content {
                                track_tool_telemetry(content, all_messages.messages());
                            }
                            all_messages.push(message.clone());
                            let token_state = get_token_state(
                                task_state.session_manager(),
                                &task_session_id,
                            )
                            .await;
                            publish(
                                Some(task_request_id.clone()),
                                MessageEvent::Message {
                                    message,
                                    token_state,
                                },
                            )
                            .await;
                        }
                        Ok(Some(Ok(AgentEvent::HistoryReplaced(new_messages)))) => {
                            all_messages = new_messages.clone();
                            publish(
                                Some(task_request_id.clone()),
                                MessageEvent::UpdateConversation {
                                    conversation: new_messages,
                                },
                            )
                            .await;
                        }
                        Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                            publish(
                                Some(task_request_id.clone()),
                                MessageEvent::ModelChange { model, mode },
                            )
                            .await;
                        }
                        Ok(Some(Ok(AgentEvent::McpNotification((notification_request_id, n))))) => {
                            publish(
                                Some(task_request_id.clone()),
                                MessageEvent::Notification {
                                    request_id: notification_request_id,
                                    message: n,
                                },
                            )
                            .await;
                        }
                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            publish(
                                Some(task_request_id.clone()),
                                MessageEvent::Error {
                                    error: e.to_string(),
                                },
                            )
                            .await;
                            break;
                        }
                        Ok(None) => {
                            break;
                        }
                        Err(_) => {
                            // Timeout — check if the bus still has subscribers
                            continue;
                        }
                    }
                }
            }
        }

        // Telemetry
        let session_duration = session_start.elapsed();

        if let Ok(session) = task_state
            .session_manager()
            .get_session(&task_session_id, true)
            .await
        {
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

        let final_token_state =
            get_token_state(task_state.session_manager(), &task_session_id).await;

        publish(
            Some(task_request_id.clone()),
            MessageEvent::Finish {
                reason: "stop".to_string(),
                token_state: final_token_state,
            },
        )
        .await;

        task_bus.cleanup_request(&task_request_id).await;
    }));

    Ok(Json(SessionReplyResponse { request_id }))
}

// ── POST /sessions/{id}/cancel ──────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/sessions/{id}/cancel",
    params(
        ("id" = String, Path, description = "Session ID"),
    ),
    request_body = CancelRequest,
    responses(
        (status = 200, description = "Cancellation accepted"),
    )
)]
pub async fn session_cancel(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(request): Json<CancelRequest>,
) -> axum::http::StatusCode {
    let bus = state.get_or_create_event_bus(&session_id).await;
    bus.cancel_request(&request.request_id).await;
    axum::http::StatusCode::OK
}

// ── Route registration ──────────────────────────────────────────────────

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/sessions/{id}/events", get(session_events))
        .route(
            "/sessions/{id}/reply",
            post(session_reply).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/sessions/{id}/cancel", post(session_cancel))
        .with_state(state)
}
