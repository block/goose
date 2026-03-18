use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
#[cfg(test)]
use axum::http::StatusCode;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{self},
    response::IntoResponse,
    routing::post,
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
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;

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
                monotonic_counter.goose.tool_completions = 1,
                tool_name = %tool_name,
                result = %result_status,
                "Tool call completed"
            );
        }
        _ => {}
    }
}

/// A buffered SSE event with its monotonic sequence ID.
#[derive(Clone)]
pub struct BufferedEvent {
    pub id: u64,
    pub json: String,
}

/// Tracks an in-flight or completed reply so clients can reconnect.
pub struct ActiveReply {
    /// Monotonic event counter for SSE `id:` field.
    pub next_event_id: AtomicU64,
    /// All events emitted so far (for replay on reconnect).
    pub event_buffer: tokio::sync::RwLock<Vec<BufferedEvent>>,
    /// Whether the reply has finished (Finish or Error event sent).
    pub finished: tokio::sync::RwLock<bool>,
    /// Broadcast channel for live-tailing new events.
    pub event_tx: tokio::sync::broadcast::Sender<BufferedEvent>,
    /// Cancellation token for the agent task.
    pub cancel_token: CancellationToken,
}

impl ActiveReply {
    pub fn new(cancel_token: CancellationToken) -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(256);
        Self {
            next_event_id: AtomicU64::new(0),
            event_buffer: tokio::sync::RwLock::new(Vec::new()),
            finished: tokio::sync::RwLock::new(false),
            event_tx,
            cancel_token,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ChatRequest {
    user_message: Message,
    /// Override the server's conversation history. Only use this when you need absolute control
    /// over the conversation state (e.g., administrative tools). For normal operations, the server
    /// is the source of truth - use truncate/fork endpoints to modify conversation history instead.
    #[serde(default)]
    override_conversation: Option<Vec<Message>>,
    session_id: String,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
    /// Client-generated idempotency key. If provided, duplicate submissions
    /// with the same (session_id, reply_id) will reconnect to the in-flight
    /// reply instead of starting a new turn.
    #[serde(default)]
    reply_id: Option<String>,
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
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
    Ping,
}

async fn get_token_state(session_manager: &SessionManager, session_id: &str) -> TokenState {
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

async fn stream_event(
    event: MessageEvent,
    tx: &mpsc::Sender<String>,
    cancel_token: &CancellationToken,
) {
    stream_event_with_active_reply(event, tx, cancel_token, None).await;
}

async fn stream_event_with_active_reply(
    event: MessageEvent,
    tx: &mpsc::Sender<String>,
    cancel_token: &CancellationToken,
    active_reply: Option<&Arc<ActiveReply>>,
) {
    let json = serde_json::to_string(&event).unwrap_or_else(|e| {
        format!(
            r#"{{"type":"Error","error":"Failed to serialize event: {}"}}"#,
            e
        )
    });

    let formatted = if let Some(ar) = active_reply {
        let event_id = ar.next_event_id.fetch_add(1, Ordering::Relaxed);
        let buffered = BufferedEvent {
            id: event_id,
            json: json.clone(),
        };
        // Buffer the event for replay on reconnect.
        ar.event_buffer.write().await.push(buffered.clone());
        // Broadcast to any live-tailing subscribers (ignore error if no receivers).
        let _ = ar.event_tx.send(buffered);
        format!("id: {}\ndata: {}\n\n", event_id, json)
    } else {
        format!("data: {}\n\n", json)
    };

    if tx.send(formatted).await.is_err() {
        tracing::info!("client hung up");
        cancel_token.cancel();
    }
}

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
    let reply_id = request.reply_id.clone();

    // If reply_id is provided, check for an existing in-flight reply to reconnect to.
    if let Some(ref rid) = reply_id {
        let key = format!("{}:{}", session_id, rid);
        let existing = {
            let replies = state.active_replies.read().await;
            replies.get(&key).cloned()
        };

        if let Some(active_reply) = existing {
            tracing::info!(
                reply_id = %rid,
                session_id = %session_id,
                "Reconnecting to existing in-flight reply"
            );
            return Ok(build_reconnect_stream(&active_reply).await);
        }
    }

    // No existing reply found (or no reply_id). Start a new agent turn.
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

    let (tx, rx) = mpsc::channel(100);
    let stream = ReceiverStream::new(rx);
    let cancel_token = CancellationToken::new();

    // If reply_id is provided, create and register an ActiveReply for idempotent reconnection.
    let active_reply: Option<Arc<ActiveReply>> = if let Some(ref rid) = reply_id {
        let ar = Arc::new(ActiveReply::new(cancel_token.clone()));
        let key = format!("{}:{}", session_id, rid);
        state.active_replies.write().await.insert(key.clone(), ar.clone());
        Some(ar)
    } else {
        None
    };

    let user_message = request.user_message;
    let override_conversation = request.override_conversation;

    let task_cancel = cancel_token.clone();
    let task_tx = tx.clone();
    let task_active_reply = active_reply.clone();
    let task_reply_key = reply_id.as_ref().map(|rid| format!("{}:{}", session_id, rid));

    drop(tokio::spawn(async move {
        let agent = match state.get_agent(session_id.clone()).await {
            Ok(agent) => agent,
            Err(e) => {
                tracing::error!("Failed to get session agent: {}", e);
                let _ = stream_event_with_active_reply(
                    MessageEvent::Error {
                        error: format!("Failed to get session agent: {}", e),
                    },
                    &task_tx,
                    &task_cancel,
                    task_active_reply.as_ref(),
                )
                .await;
                return;
            }
        };

        let session = match state.session_manager().get_session(&session_id, true).await {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to read session for {}: {}", session_id, e);
                let _ = stream_event_with_active_reply(
                    MessageEvent::Error {
                        error: format!("Failed to read session: {}", e),
                    },
                    &task_tx,
                    &cancel_token,
                    task_active_reply.as_ref(),
                )
                .await;
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
                stream_event_with_active_reply(
                    MessageEvent::Error {
                        error: e.to_string(),
                    },
                    &task_tx,
                    &cancel_token,
                    task_active_reply.as_ref(),
                )
                .await;
                return;
            }
        };

        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = task_cancel.cancelled() => {
                    tracing::info!("Agent task cancelled");
                    break;
                }
                _ = heartbeat_interval.tick() => {
                    stream_event_with_active_reply(MessageEvent::Ping, &tx, &cancel_token, task_active_reply.as_ref()).await;
                }
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            for content in &message.content {
                                track_tool_telemetry(content, all_messages.messages());
                            }

                            all_messages.push(message.clone());

                            let token_state = get_token_state(state.session_manager(), &session_id).await;

                            stream_event_with_active_reply(MessageEvent::Message { message, token_state }, &tx, &cancel_token, task_active_reply.as_ref()).await;
                        }
                        Ok(Some(Ok(AgentEvent::HistoryReplaced(new_messages)))) => {
                            all_messages = new_messages.clone();
                            stream_event_with_active_reply(MessageEvent::UpdateConversation {conversation: new_messages}, &tx, &cancel_token, task_active_reply.as_ref()).await;

                        }
                        Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                            stream_event_with_active_reply(MessageEvent::ModelChange { model, mode }, &tx, &cancel_token, task_active_reply.as_ref()).await;
                        }
                        Ok(Some(Ok(AgentEvent::McpNotification((request_id, n))))) => {
                            stream_event_with_active_reply(MessageEvent::Notification{
                                request_id: request_id.clone(),
                                message: n,
                            }, &tx, &cancel_token, task_active_reply.as_ref()).await;
                        }

                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            stream_event_with_active_reply(
                                MessageEvent::Error {
                                    error: e.to_string(),
                                },
                                &tx,
                                &cancel_token,
                                task_active_reply.as_ref(),
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

        stream_event_with_active_reply(
            MessageEvent::Finish {
                reason: "stop".to_string(),
                token_state: final_token_state,
            },
            &task_tx,
            &cancel_token,
            task_active_reply.as_ref(),
        )
        .await;

        // Mark the reply as finished and schedule cleanup.
        if let Some(ref ar) = task_active_reply {
            *ar.finished.write().await = true;
        }
        if let Some(key) = task_reply_key {
            let cleanup_state = state.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60)).await;
                cleanup_state.active_replies.write().await.remove(&key);
                tracing::debug!(reply_key = %key, "Cleaned up finished active reply");
            });
        }
    }));
    Ok(SseResponse::new(stream))
}

/// Build a reconnect SSE stream for an existing `ActiveReply`.
///
/// Replays all buffered events, then live-tails new events from the broadcast channel
/// until the reply finishes.
async fn build_reconnect_stream(active_reply: &Arc<ActiveReply>) -> SseResponse {
    let (tx, rx) = mpsc::channel::<String>(100);
    let stream = ReceiverStream::new(rx);

    let ar = active_reply.clone();
    tokio::spawn(async move {
        // 1. Replay buffered events.
        {
            let buffer = ar.event_buffer.read().await;
            for event in buffer.iter() {
                let formatted = format!("id: {}\ndata: {}\n\n", event.id, event.json);
                if tx.send(formatted).await.is_err() {
                    return; // client disconnected
                }
            }
        }

        // 2. If already finished, we're done after replay.
        if *ar.finished.read().await {
            return;
        }

        // 3. Subscribe to live events and forward them.
        // We need to get the current buffer length to know which events to skip
        // (they were already replayed above).
        let replayed_up_to = {
            let buffer = ar.event_buffer.read().await;
            buffer.last().map(|e| e.id)
        };

        let mut rx_broadcast = ar.event_tx.subscribe();

        loop {
            match rx_broadcast.recv().await {
                Ok(event) => {
                    // Skip events we already replayed.
                    if let Some(last_id) = replayed_up_to {
                        if event.id <= last_id {
                            continue;
                        }
                    }
                    let formatted = format!("id: {}\ndata: {}\n\n", event.id, event.json);
                    if tx.send(formatted).await.is_err() {
                        return; // client disconnected
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Reconnect stream lagged by {} events, some may be lost", n);
                    // Continue receiving; the client can reconnect again if needed.
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // The producer is done. Drain any remaining buffered events
                    // that might have been added after our replay snapshot.
                    let buffer = ar.event_buffer.read().await;
                    for event in buffer.iter() {
                        if let Some(last_id) = replayed_up_to {
                            if event.id <= last_id {
                                continue;
                            }
                        }
                        let formatted = format!("id: {}\ndata: {}\n\n", event.id, event.json);
                        if tx.send(formatted).await.is_err() {
                            return;
                        }
                    }
                    return;
                }
            }
        }
    });

    SseResponse::new(stream)
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/reply",
            post(reply).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
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
                        reply_id: None,
                    })
                    .unwrap(),
                ))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
