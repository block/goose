//! A2A (Agent-to-Agent) protocol routes.
//!
//! Mounts spec-compliant A2A endpoints:
//!
//! ## Aggregated (all personas)
//! - `GET  /a2a/.well-known/agent-card.json` — Main Agent Card (all skills)
//! - `POST /a2a`                             — JSON-RPC 2.0 (message/send, tasks/*)
//! - `POST /a2a/stream`                      — SSE streaming (message/sendStream)
//!
//! ## Per-persona
//! - `GET  /a2a/agents`                               — List all persona cards
//! - `GET  /a2a/agents/{persona}/agent-card.json`     — Per-persona Agent Card
//! - `POST /a2a/agents/{persona}`                     — JSON-RPC scoped to persona
//! - `POST /a2a/agents/{persona}/stream`              — SSE scoped to persona

use std::sync::Arc;

use a2a::server::context::RequestContext;
use a2a::server::executor::AgentExecutor;
use a2a::server::request_handler::DefaultRequestHandler;
use a2a::server::store::InMemoryTaskStore;
use a2a::server::transport::create_a2a_router;
use a2a::types::agent_card::{
    AgentCapabilities, AgentCard, AgentInterface, AgentProvider, AgentSkill,
};
use a2a::types::core::{Artifact, Message, Part, PartContent, TaskState, TaskStatus};
use a2a::types::events::{AgentExecutionEvent, TaskArtifactUpdateEvent, TaskStatusUpdateEvent};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::Json;
use axum::Router;
use futures::StreamExt;
use goose::a2a_compat::message::{a2a_message_to_goose, goose_message_to_a2a};
use goose::agents::intent_router::IntentRouter;
use goose::agents::{AgentEvent, SessionConfig};
use goose::execution::pool::{InstanceStatus, PoolEvent, SpawnConfig};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Routes
// ─────────────────────────────────────────────────────────────────────────────

/// Build the A2A sub-router and nest it under `/a2a`.
pub fn routes(state: Arc<AppState>) -> Router {
    // --- Main aggregated A2A endpoint (all personas) ---
    let main_card = build_a2a_agent_card();
    let main_store = InMemoryTaskStore::new();
    let main_executor = GooseServerExecutor {
        state: state.clone(),
        persona: None,
    };
    let main_handler = DefaultRequestHandler::new(main_card, main_store, main_executor);
    let main_router = create_a2a_router(main_handler);

    // --- Per-persona discovery endpoint ---
    let persona_state = state.clone();
    let list_personas_router = Router::new()
        .route("/agents", axum::routing::get(list_personas))
        .with_state(persona_state);

    // --- Per-persona A2A endpoints ---
    let mut persona_router = Router::new();
    for (slug, card) in build_all_persona_cards() {
        let persona_store = InMemoryTaskStore::new();
        let persona_executor = GooseServerExecutor {
            state: state.clone(),
            persona: Some(slug.clone()),
        };
        let persona_handler = DefaultRequestHandler::new(card, persona_store, persona_executor);
        let sub = create_a2a_router(persona_handler);
        persona_router = persona_router.nest(&format!("/agents/{slug}"), sub);
    }

    // --- Instance management routes ---
    let instance_state = state.clone();
    let instance_router = Router::new()
        .route("/instances", axum::routing::get(list_instances))
        .route("/instances", axum::routing::post(spawn_instance))
        .route("/instances/{instance_id}", axum::routing::get(get_instance))
        .route(
            "/instances/{instance_id}",
            axum::routing::delete(cancel_instance),
        )
        .route(
            "/instances/{instance_id}/card",
            axum::routing::get(get_instance_card),
        )
        .route(
            "/instances/{instance_id}/result",
            axum::routing::get(get_instance_result),
        )
        .route(
            "/instances/{instance_id}/events",
            axum::routing::get(stream_instance_events),
        )
        .with_state(instance_state);

    // Compose: /a2a/instances/* + /a2a/agents/* + /a2a/*
    Router::new()
        .nest("/a2a", instance_router)
        .nest("/a2a", list_personas_router)
        .nest("/a2a", persona_router)
        .nest("/a2a", main_router)
}

// ─────────────────────────────────────────────────────────────────────────────
// GooseServerExecutor — bridges AppState → A2A AgentExecutor
// ─────────────────────────────────────────────────────────────────────────────

/// An AgentExecutor that creates a Goose agent per task using AppState.
///
/// When `persona` is set, the executor prepends a persona instruction to the
/// user message so the agent acts in that specific mode (e.g., "developer",
/// "security", "research").
#[derive(Clone)]
struct GooseServerExecutor {
    state: Arc<AppState>,
    persona: Option<String>,
}

#[async_trait::async_trait]
impl AgentExecutor for GooseServerExecutor {
    async fn execute(
        &self,
        context: RequestContext,
        event_tx: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), a2a::A2AError> {
        let session_id = context.context_id.clone();

        let agent = self
            .state
            .get_agent(session_id)
            .await
            .map_err(|e| a2a::A2AError::internal_error(e.to_string()))?;

        // If persona is set, prepend routing instruction to the message
        let goose_msg = if let Some(ref persona) = self.persona {
            let mut msg = a2a_message_to_goose(&context.user_message);
            let persona_instruction = format!(
                "[System: Route this request to the {persona} persona. \
                 Apply the {persona} mode's system prompt and tool groups.]"
            );
            msg.content.insert(
                0,
                goose::conversation::message::MessageContent::text(persona_instruction),
            );
            msg
        } else {
            a2a_message_to_goose(&context.user_message)
        };

        let session_config = SessionConfig {
            id: context.context_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        // Signal working
        let _ = event_tx
            .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                task_id: context.task_id.clone(),
                context_id: context.context_id.clone(),
                status: TaskStatus {
                    state: TaskState::Working,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }))
            .await;

        let mut stream = match agent.reply(goose_msg, session_config, None).await {
            Ok(s) => s,
            Err(e) => {
                let _ = event_tx
                    .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                        task_id: context.task_id.clone(),
                        context_id: context.context_id.clone(),
                        status: TaskStatus {
                            state: TaskState::Failed,
                            message: Some(Box::new(Message {
                                role: a2a::types::core::Role::Agent,
                                parts: vec![Part {
                                    content: PartContent::Text {
                                        text: e.to_string(),
                                    },
                                    metadata: None,
                                    filename: None,
                                    media_type: None,
                                }],
                                message_id: String::new(),
                                context_id: Some(context.context_id.clone()),
                                task_id: Some(context.task_id.clone()),
                                extensions: vec![],
                                reference_task_ids: vec![],
                                metadata: None,
                            })),
                            timestamp: None,
                        },
                        metadata: None,
                    }))
                    .await;
                return Ok(());
            }
        };

        let mut artifact_index = 0u32;
        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(AgentEvent::Message(msg)) => {
                    if msg.role == rmcp::model::Role::Assistant {
                        let a2a_msg = goose_message_to_a2a(&msg);
                        if !a2a_msg.parts.is_empty() {
                            let _ = event_tx
                                .send(AgentExecutionEvent::ArtifactUpdate(
                                    TaskArtifactUpdateEvent {
                                        task_id: context.task_id.clone(),
                                        context_id: context.context_id.clone(),
                                        artifact: Artifact {
                                            artifact_id: format!("artifact-{artifact_index}"),
                                            name: Some(format!("response-{artifact_index}")),
                                            parts: a2a_msg.parts,
                                            description: None,
                                            metadata: None,
                                            extensions: vec![],
                                        },
                                        append: false,
                                        last_chunk: false,
                                        metadata: None,
                                    },
                                ))
                                .await;
                            artifact_index += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("Agent stream error: {e}");
                    let _ = event_tx
                        .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                            task_id: context.task_id.clone(),
                            context_id: context.context_id.clone(),
                            status: TaskStatus {
                                state: TaskState::Failed,
                                message: Some(Box::new(Message {
                                    role: a2a::types::core::Role::Agent,
                                    parts: vec![Part {
                                        content: PartContent::Text {
                                            text: e.to_string(),
                                        },
                                        metadata: None,
                                        filename: None,
                                        media_type: None,
                                    }],
                                    message_id: String::new(),
                                    context_id: Some(context.context_id.clone()),
                                    task_id: Some(context.task_id.clone()),
                                    extensions: vec![],
                                    reference_task_ids: vec![],
                                    metadata: None,
                                })),
                                timestamp: None,
                            },
                            metadata: None,
                        }))
                        .await;
                    return Ok(());
                }
                _ => {
                    debug!("Skipping non-message agent event");
                }
            }
        }

        // Completed
        let _ = event_tx
            .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                task_id: context.task_id.clone(),
                context_id: context.context_id.clone(),
                status: TaskStatus {
                    state: TaskState::Completed,
                    message: None,
                    timestamp: None,
                },
                metadata: None,
            }))
            .await;

        Ok(())
    }

    async fn cancel(
        &self,
        _task_id: &str,
        _event_tx: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), a2a::A2AError> {
        // Cancellation not yet supported for Goose agents
        Err(a2a::A2AError::TaskNotCancelable {
            task_id: _task_id.to_string(),
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Instance Management — A2A-native agent instances
// ─────────────────────────────────────────────────────────────────────────────

/// Request body for POST /a2a/instances
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SpawnInstanceRequest {
    pub persona: String,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_turns: Option<usize>,
}

/// Response for instance creation and status queries.
#[derive(Serialize, utoipa::ToSchema)]
pub struct InstanceResponse {
    pub id: String,
    pub persona: String,
    pub status: String,
    pub turns: u32,
    pub provider_name: String,
    pub model_name: String,
    pub elapsed_secs: f64,
    pub last_activity_ms: u64,
}

/// Response for instance results.
#[derive(Serialize, utoipa::ToSchema)]
pub struct InstanceResultResponse {
    pub id: String,
    pub persona: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub turns_taken: u32,
    pub duration_secs: f64,
}

/// List all pool instances with their current status.
#[utoipa::path(
    get,
    path = "/a2a/instances",
    responses(
        (status = 200, description = "List of all agent pool instances", body = Vec<InstanceResponse>)
    ),
    tag = "A2A Instances"
)]
pub async fn list_instances(State(state): State<Arc<AppState>>) -> Json<Vec<InstanceResponse>> {
    let snapshots = state.agent_pool.status_all().await;
    let instances: Vec<InstanceResponse> =
        snapshots.into_iter().map(snapshot_to_response).collect();
    Json(instances)
}

fn snapshot_to_response(snap: goose::execution::pool::InstanceSnapshot) -> InstanceResponse {
    InstanceResponse {
        id: snap.id,
        persona: snap.persona,
        status: snap.status.to_string(),
        turns: snap.turns,
        provider_name: snap.provider_name,
        model_name: snap.model_name,
        elapsed_secs: snap.elapsed.as_secs_f64(),
        last_activity_ms: snap.last_activity_ms,
    }
}

/// Spawn a new agent instance in the pool.
#[utoipa::path(
    post,
    path = "/a2a/instances",
    request_body = SpawnInstanceRequest,
    responses(
        (status = 201, description = "Instance spawned", body = InstanceResponse),
        (status = 400, description = "Invalid provider or model config"),
        (status = 503, description = "No default provider or spawn failed")
    ),
    tag = "A2A Instances"
)]
pub async fn spawn_instance(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(req): axum::extract::Json<SpawnInstanceRequest>,
) -> Result<(StatusCode, Json<InstanceResponse>), StatusCode> {
    let provider = if req.provider.is_some() || req.model.is_some() {
        let provider_name = req.provider.unwrap_or_else(|| "openai".to_string());
        let model_name = req.model.unwrap_or_else(|| "gpt-4o".to_string());
        let model_config = goose::model::ModelConfig::new(&model_name).map_err(|e| {
            warn!("Invalid model config: {e}");
            StatusCode::BAD_REQUEST
        })?;
        goose::providers::create(&provider_name, model_config, vec![])
            .await
            .map_err(|e| {
                warn!("Failed to create provider: {e}");
                StatusCode::BAD_REQUEST
            })?
    } else {
        state
            .agent_manager
            .get_default_provider()
            .await
            .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
    };

    let instructions = req
        .instructions
        .unwrap_or_else(|| format!("You are operating as the {} persona.", req.persona));

    let config = SpawnConfig {
        persona: req.persona.clone(),
        instructions: instructions.clone(),
        prompt: instructions,
        working_dir: std::env::current_dir().unwrap_or_default(),
        provider,
        extensions: vec![],
        exclude_extensions: vec![],
        inherit_extensions: None,
        max_turns: req.max_turns,
        session_manager: state.agent_manager.session_manager_arc(),
        identity: None,
    };

    let instance_id = state.agent_pool.spawn(config).await.map_err(|e| {
        warn!("Failed to spawn instance: {e}");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    Ok((
        StatusCode::CREATED,
        Json(InstanceResponse {
            id: instance_id,
            persona: req.persona,
            status: InstanceStatus::Running.to_string(),
            turns: 0,
            provider_name: String::new(),
            model_name: String::new(),
            elapsed_secs: 0.0,
            last_activity_ms: 0,
        }),
    ))
}

/// Get status of a specific instance.
#[utoipa::path(
    get,
    path = "/a2a/instances/{instance_id}",
    params(("instance_id" = String, Path, description = "Instance ID")),
    responses(
        (status = 200, description = "Instance status", body = InstanceResponse),
        (status = 404, description = "Instance not found")
    ),
    tag = "A2A Instances"
)]
pub async fn get_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> Result<Json<InstanceResponse>, StatusCode> {
    let snap = state
        .agent_pool
        .status(&instance_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(snapshot_to_response(snap)))
}

/// Cancel a running instance.
#[utoipa::path(
    delete,
    path = "/a2a/instances/{instance_id}",
    params(("instance_id" = String, Path, description = "Instance ID")),
    responses(
        (status = 204, description = "Instance cancelled"),
        (status = 404, description = "Instance not found")
    ),
    tag = "A2A Instances"
)]
pub async fn cancel_instance(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state
        .agent_pool
        .cancel(&instance_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get the A2A agent card for a specific instance.
#[utoipa::path(
    get,
    path = "/a2a/instances/{instance_id}/card",
    params(("instance_id" = String, Path, description = "Instance ID")),
    responses(
        (status = 200, description = "Instance agent card"),
        (status = 404, description = "Instance not found")
    ),
    tag = "A2A Instances"
)]
pub async fn get_instance_card(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> Result<Json<AgentCard>, StatusCode> {
    let snap = state
        .agent_pool
        .status(&instance_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let card = build_card_base(
        &format!("Goose Instance: {}", snap.persona),
        &format!(
            "A2A-native agent instance running as '{}' persona (id: {})",
            snap.persona, snap.id
        ),
        vec![AgentSkill {
            id: format!("instance.{}", snap.id),
            name: snap.persona.clone(),
            description: format!("Running instance of {} persona", snap.persona),
            tags: vec!["instance".to_string()],
            ..Default::default()
        }],
    );

    Ok(Json(card))
}

/// Get the result of a completed instance.
#[utoipa::path(
    get,
    path = "/a2a/instances/{instance_id}/result",
    params(("instance_id" = String, Path, description = "Instance ID")),
    responses(
        (status = 200, description = "Instance result", body = InstanceResultResponse),
        (status = 404, description = "Instance not found")
    ),
    tag = "A2A Instances"
)]
pub async fn get_instance_result(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> Result<Json<InstanceResultResponse>, StatusCode> {
    // Check completed results first
    let results = state.agent_pool.completed_results().await;
    if let Some(result) = results.iter().find(|r| r.id == instance_id) {
        return Ok(Json(InstanceResultResponse {
            id: result.id.clone(),
            persona: result.persona.clone(),
            status: result.status.to_string(),
            output: result.output.clone(),
            error: result.error.clone(),
            turns_taken: result.turns_taken,
            duration_secs: result.duration.as_secs_f64(),
        }));
    }

    // Check if still running
    if let Some(snap) = state.agent_pool.status(&instance_id).await {
        return Ok(Json(InstanceResultResponse {
            id: snap.id,
            persona: snap.persona,
            status: snap.status.to_string(),
            output: None,
            error: None,
            turns_taken: snap.turns,
            duration_secs: snap.elapsed.as_secs_f64(),
        }));
    }

    Err(StatusCode::NOT_FOUND)
}

/// Stream real-time events from a running pool instance via SSE.
///
/// Subscribes to the instance's broadcast channel and streams PoolEvents
/// as Server-Sent Events. The stream closes when the instance completes
/// or the client disconnects.
#[utoipa::path(
    get,
    path = "/a2a/instances/{instance_id}/events",
    params(("instance_id" = String, Path, description = "Instance ID")),
    responses(
        (status = 200, description = "SSE event stream for instance"),
        (status = 404, description = "Instance not found")
    ),
    tag = "A2A Instances"
)]
pub async fn stream_instance_events(
    State(state): State<Arc<AppState>>,
    Path(instance_id): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>>, StatusCode> {
    let mut rx = state
        .agent_pool
        .subscribe(&instance_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let (event_type, data) = match &event {
                        PoolEvent::Message { text } => {
                            ("message", serde_json::json!({ "text": text }).to_string())
                        }
                        PoolEvent::TurnComplete { turn } => {
                            ("turn", serde_json::json!({ "turn": turn }).to_string())
                        }
                        PoolEvent::Completed { output } => {
                            let data = serde_json::json!({ "output": output }).to_string();
                            yield Ok(Event::default().event("completed").data(data));
                            break;
                        }
                        PoolEvent::Failed { error } => {
                            let data = serde_json::json!({ "error": error }).to_string();
                            yield Ok(Event::default().event("failed").data(data));
                            break;
                        }
                        PoolEvent::Cancelled => {
                            yield Ok(Event::default().event("cancelled").data("{}"));
                            break;
                        }
                    };
                    yield Ok(Event::default().event(event_type).data(data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    let data = serde_json::json!({ "skipped": n }).to_string();
                    yield Ok(Event::default().event("lagged").data(data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream))
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Card builders
// ─────────────────────────────────────────────────────────────────────────────

/// Summary for the /a2a/agents listing endpoint.
#[derive(Serialize, utoipa::ToSchema)]
pub struct PersonaSummary {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub skills_count: usize,
}

/// List all available personas.
#[utoipa::path(
    get,
    path = "/a2a/agents",
    responses(
        (status = 200, description = "List of available personas", body = Vec<PersonaSummary>)
    ),
    tag = "A2A Instances"
)]
pub async fn list_personas() -> Json<Vec<PersonaSummary>> {
    let cards = build_all_persona_cards();
    let summary: Vec<PersonaSummary> = cards
        .into_iter()
        .map(|(slug, card)| PersonaSummary {
            slug,
            name: card.name,
            description: card.description,
            skills_count: card.skills.len(),
        })
        .collect();
    Json(summary)
}

/// Build the main (aggregated) A2A AgentCard with skills from ALL personas.
fn build_a2a_agent_card() -> AgentCard {
    let router = IntentRouter::new();
    let skills: Vec<AgentSkill> = router
        .slots()
        .iter()
        .flat_map(|slot| {
            let agent_name = slot.name.clone();
            slot.modes
                .iter()
                .filter(|mode| !mode.is_internal)
                .map(move |mode| AgentSkill {
                    id: format!("{}.{}", slugify(&agent_name), mode.slug),
                    name: format!("{} — {}", agent_name, mode.name),
                    description: mode.description.clone(),
                    tags: mode
                        .tool_groups
                        .iter()
                        .map(|tg| format!("{tg:?}"))
                        .collect(),
                    ..Default::default()
                })
        })
        .collect();

    build_card_base(
        "Goose",
        "An open-source AI agent by Block with multi-persona routing. \
                    Supports software development, DevOps, QA, and general-purpose tasks.",
        skills,
    )
}

/// Build a per-persona AgentCard from a single IntentRouter slot.
fn build_persona_card(slot: &goose::agents::intent_router::AgentSlot) -> AgentCard {
    let skills: Vec<AgentSkill> = slot
        .modes
        .iter()
        .filter(|mode| !mode.is_internal)
        .map(|mode| AgentSkill {
            id: format!("{}.{}", slugify(&slot.name), mode.slug),
            name: mode.name.clone(),
            description: mode.description.clone(),
            tags: mode
                .tool_groups
                .iter()
                .map(|tg| format!("{tg:?}"))
                .collect(),
            ..Default::default()
        })
        .collect();

    build_card_base(&slot.name, &slot.description, skills)
}

/// Build all persona cards keyed by slug.
fn build_all_persona_cards() -> Vec<(String, AgentCard)> {
    let router = IntentRouter::new();
    router
        .slots()
        .iter()
        .filter(|slot| slot.enabled)
        .map(|slot| {
            let slug = slugify(&slot.name);
            let card = build_persona_card(slot);
            (slug, card)
        })
        .collect()
}

/// Common AgentCard builder with shared metadata.
fn build_card_base(name: &str, description: &str, skills: Vec<AgentSkill>) -> AgentCard {
    AgentCard {
        name: name.to_string(),
        description: description.to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        protocol_version: Some("1.0".to_string()),
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        supported_interfaces: vec![AgentInterface {
            url: String::new(),
            protocol_binding: Some("JSONRPC".to_string()),
            protocol_version: Some("1.0".to_string()),
            ..Default::default()
        }],
        skills,
        capabilities: Some(AgentCapabilities {
            streaming: true,
            push_notifications: false,
            extensions: false,
            extended_agent_card: false,
        }),
        provider: Some(AgentProvider {
            organization: "Block, Inc.".to_string(),
            url: Some("https://github.com/block/goose".to_string()),
        }),
        documentation_url: Some("https://block.github.io/goose/".to_string()),
        ..Default::default()
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Main card tests ---

    #[test]
    fn test_a2a_agent_card_has_skills() {
        let card = build_a2a_agent_card();
        assert_eq!(card.name, "Goose");
        assert!(!card.skills.is_empty());
        assert_eq!(card.protocol_version, Some("1.0".to_string()));
    }

    #[test]
    fn test_a2a_agent_card_capabilities() {
        let card = build_a2a_agent_card();
        let caps = card.capabilities.unwrap();
        assert!(caps.streaming);
        assert!(!caps.push_notifications);
    }

    #[test]
    fn test_a2a_agent_card_serializes_camelcase() {
        let card = build_a2a_agent_card();
        let json = serde_json::to_string_pretty(&card).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("defaultInputModes"));
        assert!(json.contains("supportedInterfaces"));
        assert!(json.contains("\"Goose\""));
    }

    #[test]
    fn test_a2a_agent_card_no_internal_skills() {
        let card = build_a2a_agent_card();
        let skill_ids: Vec<&str> = card.skills.iter().map(|s| s.id.as_str()).collect();
        assert!(
            !skill_ids.iter().any(|id| id.contains("judge")),
            "Internal mode 'judge' should not appear"
        );
        assert!(
            !skill_ids.iter().any(|id| id.contains("planner")),
            "Internal mode 'planner' should not appear"
        );
    }

    // --- Per-persona card tests ---

    #[test]
    fn test_persona_cards_exist() {
        let cards = build_all_persona_cards();
        assert!(
            cards.len() >= 2,
            "Expected at least Goose + Developer personas"
        );

        let slugs: Vec<&str> = cards.iter().map(|(s, _)| s.as_str()).collect();
        assert!(
            slugs.contains(&"goose-agent"),
            "Missing goose-agent persona"
        );
        assert!(
            slugs.contains(&"developer-agent"),
            "Missing developer-agent persona"
        );
    }

    #[test]
    fn test_persona_cards_have_distinct_skills() {
        let cards = build_all_persona_cards();
        let goose_card = cards.iter().find(|(s, _)| s == "goose-agent").unwrap();
        let dev_card = cards.iter().find(|(s, _)| s == "developer-agent").unwrap();

        // Each persona should have its own skills
        assert!(!goose_card.1.skills.is_empty());
        assert!(!dev_card.1.skills.is_empty());

        // Skills should be different (different IDs)
        let goose_ids: Vec<&str> = goose_card.1.skills.iter().map(|s| s.id.as_str()).collect();
        let dev_ids: Vec<&str> = dev_card.1.skills.iter().map(|s| s.id.as_str()).collect();
        assert_ne!(goose_ids, dev_ids, "Persona skills should differ");
    }

    #[test]
    fn test_persona_cards_no_internal_modes() {
        let cards = build_all_persona_cards();
        for (slug, card) in &cards {
            for skill in &card.skills {
                assert!(
                    !skill.id.contains("judge")
                        && !skill.id.contains("planner")
                        && !skill.id.contains("recipe_maker"),
                    "Persona {slug} exposes internal mode: {}",
                    skill.id
                );
            }
        }
    }

    #[test]
    fn test_persona_card_metadata() {
        let cards = build_all_persona_cards();
        for (_slug, card) in &cards {
            assert_eq!(card.protocol_version, Some("1.0".to_string()));
            assert!(card.capabilities.is_some());
            assert!(card.provider.is_some());
            assert!(!card.name.is_empty());
            assert!(!card.description.is_empty());
        }
    }

    #[test]
    fn test_persona_card_serializes_camelcase() {
        let cards = build_all_persona_cards();
        let (_slug, card) = &cards[0];
        let json = serde_json::to_string_pretty(card).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("defaultInputModes"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Goose Agent"), "goose-agent");
        assert_eq!(slugify("Developer Agent"), "developer-agent");
        assert_eq!(slugify("QA Agent"), "qa-agent");
    }
}
