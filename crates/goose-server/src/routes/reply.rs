use crate::agent_slot_registry::SlotDelegation;
use crate::auth::RequestIdentity;
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
use goose::agents::orchestrator_agent::{aggregate_results, OrchestratorAgent};
use goose::agents::{AgentEvent, SessionConfig};
use goose::conversation::message::{Message, MessageContent, RoutingInfo, TokenState};
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
    user_message: Message,
    #[serde(default)]
    conversation_so_far: Option<Vec<Message>>,
    session_id: String,
    recipe_name: Option<String>,
    recipe_version: Option<String>,
    /// Optional mode: "plan" returns a structured plan without executing,
    /// "execute_plan" executes a previously confirmed plan.
    /// None or absent = normal reply flow.
    #[serde(default)]
    mode: Option<String>,
    /// The confirmed plan to execute (only used when mode = "execute_plan").
    #[serde(default)]
    plan: Option<serde_json::Value>,
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
    RoutingDecision {
        agent_name: String,
        mode_slug: String,
        confidence: f32,
        reasoning: String,
    },
    Notification {
        request_id: String,
        #[schema(value_type = Object)]
        message: ServerNotification,
    },
    UpdateConversation {
        conversation: Conversation,
    },
    ToolAvailabilityChange {
        previous_count: usize,
        current_count: usize,
    },
    PlanProposal {
        is_compound: bool,
        tasks: Vec<PlanTask>,
        #[serde(skip_serializing_if = "Option::is_none")]
        clarifying_questions: Option<Vec<String>>,
    },
    Ping,
}

/// A task within a plan proposal, serializable for SSE transport.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PlanTask {
    pub task_id: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub agent_name: String,
    pub mode_slug: String,
    pub mode_name: String,
    pub confidence: f32,
    pub reasoning: String,
    pub description: String,
    pub tool_groups: Vec<String>,
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
    headers: axum::http::HeaderMap,
    Json(request): Json<ChatRequest>,
) -> Result<SseResponse, ErrorResponse> {
    let identity = RequestIdentity::from_headers_validated(
        &headers,
        &state.oidc_validator,
        &state.session_token_store,
    )
    .await;
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

    let user_message = request.user_message;
    let conversation_so_far = request.conversation_so_far;
    let request_mode = request.mode;
    let _request_plan = request.plan;

    let task_cancel = cancel_token.clone();
    let task_tx = tx.clone();

    drop(tokio::spawn(async move {
        let agent = match state.get_agent(session_id.clone()).await {
            Ok(agent) => agent,
            Err(e) => {
                tracing::error!("Failed to get session agent: {}", e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: format!("Failed to get session agent: {}", e),
                    },
                    &task_tx,
                    &task_cancel,
                )
                .await;
                return;
            }
        };

        // Wire execution identity onto the agent (user from request headers + agent metadata)
        let exec_identity = identity.into_execution("goose", "Goose Agent");
        agent.set_execution_identity(exec_identity.clone()).await;
        tracing::info!(
            user_id = %exec_identity.user.id,
            agent_id = %exec_identity.agent.id,
            "Execution identity set on agent"
        );

        // Tag the session with the caller's tenant/user identity for scoping
        if !exec_identity.user.is_guest() || exec_identity.user.tenant.is_some() {
            if let Err(e) = state
                .session_manager()
                .set_session_identity(
                    &session_id,
                    exec_identity.user.tenant.as_deref(),
                    Some(exec_identity.user.id.as_str()),
                )
                .await
            {
                tracing::warn!(session_id = %session_id, "Failed to set session identity: {}", e);
            }
        }

        let session = match state.session_manager().get_session(&session_id, true).await {
            Ok(metadata) => metadata,
            Err(e) => {
                tracing::error!("Failed to read session for {}: {}", session_id, e);
                let _ = stream_event(
                    MessageEvent::Error {
                        error: format!("Failed to read session: {}", e),
                    },
                    &task_tx,
                    &cancel_token,
                )
                .await;
                return;
            }
        };

        // Route user message to the best agent/mode via IntentRouter
        {
            let user_text: String = user_message
                .content
                .iter()
                .filter_map(|c| {
                    if let MessageContent::Text(t) = c {
                        Some(t.text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");

            if !user_text.is_empty() {
                let provider = Arc::new(tokio::sync::Mutex::new(agent.provider().await.ok()));
                let mut router = OrchestratorAgent::new(provider);

                state
                    .agent_slot_registry
                    .configure_orchestrator(&mut router)
                    .await;

                let plan = router.route(&user_text).await;
                let primary = plan.primary_routing();

                tracing::info!(
                    agent_name = %primary.agent_name,
                    mode_slug = %primary.mode_slug,
                    confidence = %primary.confidence,
                    is_compound = plan.is_compound,
                    task_count = plan.tasks.len(),
                    "Routed message to agent/mode"
                );

                // Apply routing bindings (tool groups, extensions, orchestrator context)
                router.apply_routing(&agent, &plan).await;

                // Emit routing decision as SSE event
                let _ = stream_event(
                    MessageEvent::RoutingDecision {
                        agent_name: primary.agent_name.clone(),
                        mode_slug: primary.mode_slug.clone(),
                        confidence: primary.confidence,
                        reasoning: primary.reasoning.clone(),
                    },
                    &task_tx,
                    &task_cancel,
                )
                .await;

                // Plan mode: return structured plan without executing
                if request_mode.as_deref() == Some("plan") {
                    let proposal = router.plan(&user_text).await;

                    let plan_tasks: Vec<crate::routes::reply::PlanTask> = proposal
                        .tasks
                        .iter()
                        .map(|t| PlanTask {
                            task_id: t.task_id.clone(),
                            depends_on: t.depends_on.clone(),
                            agent_name: t.agent_name.clone(),
                            mode_slug: t.mode_slug.clone(),
                            mode_name: t.mode_name.clone(),
                            confidence: t.confidence,
                            reasoning: t.reasoning.clone(),
                            description: t.description.clone(),
                            tool_groups: t.tool_groups.clone(),
                        })
                        .collect();

                    let _ = stream_event(
                        MessageEvent::PlanProposal {
                            is_compound: proposal.is_compound,
                            tasks: plan_tasks,
                            clarifying_questions: proposal.clarifying_questions,
                        },
                        &task_tx,
                        &task_cancel,
                    )
                    .await;

                    let token_state = get_token_state(state.session_manager(), &session_id).await;

                    let _ = stream_event(
                        MessageEvent::Finish {
                            reason: "plan_complete".to_string(),
                            token_state,
                        },
                        &task_tx,
                        &task_cancel,
                    )
                    .await;

                    return;
                }

                // Compound execution: delegate to dispatch module
                if plan.is_compound && plan.tasks.len() > 1 {
                    tracing::info!(
                        task_count = plan.tasks.len(),
                        "Executing compound request via dispatch_compound_sequential"
                    );

                    // Build (SubTask, Option<a2a_url>) tuples from slot delegation
                    let mut dispatch_tasks = Vec::with_capacity(plan.tasks.len());
                    for sub_task in &plan.tasks {
                        let agent_name = &sub_task.routing.agent_name;
                        let delegation = state.agent_slot_registry.get_delegation(agent_name).await;
                        let a2a_url = match delegation {
                            SlotDelegation::RemoteA2A { ref url } => Some(url.clone()),
                            _ => {
                                // Apply routing for in-process sub-tasks
                                let sub_plan =
                                    goose::agents::orchestrator_agent::OrchestratorPlan::single(
                                        sub_task.routing.clone(),
                                    );
                                router.apply_routing(&agent, &sub_plan).await;
                                None
                            }
                        };
                        dispatch_tasks.push((sub_task.clone(), a2a_url));
                    }

                    let reply_dispatcher = goose::agents::dispatch::AgentReplyDispatcher::new(
                        agent.clone(),
                        session_id.clone(),
                    );

                    let results = goose::agents::dispatch::dispatch_compound_sequential(
                        &reply_dispatcher,
                        &dispatch_tasks,
                        Some(task_cancel.clone()),
                    )
                    .await;

                    // Aggregate outputs from dispatch results
                    let sub_results: Vec<String> =
                        results.iter().map(|r| r.output.clone()).collect();
                    let aggregated = aggregate_results(&plan.tasks, &sub_results);
                    let aggregated_message = Message::assistant().with_text(&aggregated);

                    let token_state = get_token_state(state.session_manager(), &session_id).await;

                    let _ = stream_event(
                        MessageEvent::Message {
                            message: aggregated_message,
                            token_state: token_state.clone(),
                        },
                        &task_tx,
                        &task_cancel,
                    )
                    .await;

                    let _ = stream_event(
                        MessageEvent::Finish {
                            reason: "compound_complete".to_string(),
                            token_state,
                        },
                        &task_tx,
                        &task_cancel,
                    )
                    .await;

                    return;
                }
            }
        }

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: session.schedule_id.clone(),
            max_turns: None,
            retry_config: None,
        };

        let mut all_messages = match conversation_so_far {
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
                stream_event(
                    MessageEvent::Error {
                        error: e.to_string(),
                    },
                    &task_tx,
                    &cancel_token,
                )
                .await;
                return;
            }
        };

        let mut current_routing_info: Option<RoutingInfo> = None;
        let mut heartbeat_interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            tokio::select! {
                _ = task_cancel.cancelled() => {
                    tracing::info!("Agent task cancelled");
                    break;
                }
                _ = heartbeat_interval.tick() => {
                    stream_event(MessageEvent::Ping, &tx, &cancel_token).await;
                }
                response = timeout(Duration::from_millis(500), stream.next()) => {
                    match response {
                        Ok(Some(Ok(AgentEvent::Message(message)))) => {
                            for content in &message.content {
                                track_tool_telemetry(content, all_messages.messages());
                            }

                            // Attach routing info to assistant messages for persistence
                            let message = if message.role == rmcp::model::Role::Assistant {
                                if let Some(ref ri) = current_routing_info {
                                    let mut msg = message;
                                    msg.metadata.routing_info = Some(ri.clone());
                                    msg
                                } else {
                                    message
                                }
                            } else {
                                message
                            };

                            all_messages.push(message.clone());

                            let token_state = get_token_state(state.session_manager(), &session_id).await;

                            stream_event(MessageEvent::Message { message, token_state }, &tx, &cancel_token).await;
                        }
                        Ok(Some(Ok(AgentEvent::HistoryReplaced(new_messages)))) => {
                            all_messages = new_messages.clone();
                            stream_event(MessageEvent::UpdateConversation {conversation: new_messages}, &tx, &cancel_token).await;

                        }
                        Ok(Some(Ok(AgentEvent::ModelChange { model, mode }))) => {
                            stream_event(MessageEvent::ModelChange { model, mode }, &tx, &cancel_token).await;
                        }
                        Ok(Some(Ok(AgentEvent::McpNotification((request_id, n))))) => {
                            stream_event(MessageEvent::Notification{
                                request_id: request_id.clone(),
                                message: n,
                            }, &tx, &cancel_token).await;
                        }
                        Ok(Some(Ok(AgentEvent::RoutingDecision { agent_name, mode_slug, confidence, reasoning }))) => {
                            current_routing_info = Some(RoutingInfo {
                                agent_name: agent_name.clone(),
                                mode_slug: mode_slug.clone(),
                            });
                            stream_event(MessageEvent::RoutingDecision { agent_name, mode_slug, confidence, reasoning }, &tx, &cancel_token).await;
                        }
                        Ok(Some(Ok(AgentEvent::ToolAvailabilityChange { previous_count, current_count }))) => {
                            tracing::warn!(
                                "Tool availability changed: {} -> {}",
                                previous_count, current_count
                            );
                            stream_event(MessageEvent::ToolAvailabilityChange { previous_count, current_count }, &tx, &cancel_token).await;
                        }
                        Ok(Some(Ok(AgentEvent::PlanCreated { is_compound, task_count, primary_agent, primary_mode, confidence }))) => {
                            tracing::info!(
                                is_compound,
                                task_count,
                                primary_agent = %primary_agent,
                                primary_mode = %primary_mode,
                                confidence,
                                "Agent created execution plan"
                            );
                        }

                        Ok(Some(Err(e))) => {
                            tracing::error!("Error processing message: {}", e);
                            stream_event(
                                MessageEvent::Error {
                                    error: e.to_string(),
                                },
                                &tx,
                                &cancel_token,
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

        let final_token_state = get_token_state(state.session_manager(), &session_id).await;

        let _ = stream_event(
            MessageEvent::Finish {
                reason: "stop".to_string(),
                token_state: final_token_state,
            },
            &task_tx,
            &cancel_token,
        )
        .await;
    }));
    Ok(SseResponse::new(stream))
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
            let state = AppState::new().await.unwrap();

            let app = routes(state);

            let request = Request::builder()
                .uri("/reply")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    serde_json::to_string(&ChatRequest {
                        user_message: Message::user().with_text("test message"),
                        conversation_so_far: None,
                        session_id: "test-session".to_string(),
                        recipe_name: None,
                        recipe_version: None,
                        mode: None,
                        plan: None,
                    })
                    .unwrap(),
                ))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}
