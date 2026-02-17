//! A2A (Agent-to-Agent) protocol routes.
//!
//! Mounts spec-compliant A2A endpoints under `/a2a`:
//! - `GET  /a2a/.well-known/agent-card.json` — Agent Card discovery
//! - `POST /a2a`                             — JSON-RPC 2.0 (message/send, tasks/*)
//! - `POST /a2a/stream`                      — SSE streaming (message/sendStream)

use std::sync::Arc;

use a2a::server::context::RequestContext;
use a2a::server::executor::AgentExecutor;
use a2a::server::request_handler::DefaultRequestHandler;
use a2a::server::store::InMemoryTaskStore;
use a2a::server::transport::create_a2a_router;
use a2a::types::agent_card::{
    AgentCapabilities, AgentCard, AgentInterface, AgentProvider, AgentSkill,
};
use a2a::types::core::{Artifact, TaskState, TaskStatus};
use a2a::types::events::{AgentExecutionEvent, TaskArtifactUpdateEvent, TaskStatusUpdateEvent};
use axum::Router;
use futures::StreamExt;
use goose::a2a_compat::message::{a2a_message_to_goose, goose_message_to_a2a};
use goose::agents::intent_router::IntentRouter;
use goose::agents::{AgentEvent, SessionConfig};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::state::AppState;

/// Build the A2A sub-router and nest it under `/a2a`.
pub fn routes(state: Arc<AppState>) -> Router {
    let agent_card = build_a2a_agent_card();
    let task_store = InMemoryTaskStore::new();
    let executor = GooseServerExecutor {
        state: state.clone(),
    };

    let handler = DefaultRequestHandler::new(agent_card, task_store, executor);

    Router::new().nest("/a2a", create_a2a_router(handler))
}

// ─────────────────────────────────────────────────────────────────────────────
// GooseServerExecutor — bridges AppState → A2A AgentExecutor
// ─────────────────────────────────────────────────────────────────────────────

/// An AgentExecutor that creates a Goose agent per task using AppState.
///
/// Uses the A2A context_id as the Goose session_id so that multi-turn
/// conversations within the same A2A context share history.
#[derive(Clone)]
struct GooseServerExecutor {
    state: Arc<AppState>,
}

#[async_trait::async_trait]
impl AgentExecutor for GooseServerExecutor {
    async fn execute(
        &self,
        context: RequestContext,
        event_tx: mpsc::Sender<AgentExecutionEvent>,
    ) -> Result<(), a2a::A2AError> {
        // Use context_id as session_id for multi-turn continuity
        let session_id = context.context_id.clone();

        let agent = self
            .state
            .get_agent(session_id)
            .await
            .map_err(|e| a2a::A2AError::internal_error(e.to_string()))?;

        let goose_msg = a2a_message_to_goose(&context.user_message);

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
                            message: None,
                            timestamp: None,
                        },
                        metadata: None,
                    }))
                    .await;
                return Err(a2a::A2AError::internal_error(e.to_string()));
            }
        };

        let mut artifact_index: u32 = 0;

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(AgentEvent::Message(msg)) => {
                    let a2a_msg = goose_message_to_a2a(&msg);
                    if a2a_msg.parts.is_empty() {
                        continue;
                    }

                    if msg.role == rmcp::model::Role::Assistant {
                        let _ = event_tx
                            .send(AgentExecutionEvent::ArtifactUpdate(
                                TaskArtifactUpdateEvent {
                                    task_id: context.task_id.clone(),
                                    context_id: context.context_id.clone(),
                                    artifact: Artifact {
                                        artifact_id: format!("artifact-{artifact_index}"),
                                        name: None,
                                        description: None,
                                        parts: a2a_msg.parts,
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
                Ok(_) => {
                    debug!("Skipping non-message AgentEvent in A2A executor");
                }
                Err(e) => {
                    warn!("Error in agent stream: {e}");
                    let _ = event_tx
                        .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                            task_id: context.task_id.clone(),
                            context_id: context.context_id.clone(),
                            status: TaskStatus {
                                state: TaskState::Failed,
                                message: None,
                                timestamp: None,
                            },
                            metadata: None,
                        }))
                        .await;
                    return Err(a2a::A2AError::internal_error(e.to_string()));
                }
            }
        }

        // Signal completion
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
        // TODO: Wire up per-task cancellation tokens
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Agent Card builder
// ─────────────────────────────────────────────────────────────────────────────

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

    AgentCard {
        name: "Goose".to_string(),
        description: "An open-source AI agent by Block with multi-persona routing. \
                      Supports software development, DevOps, QA, and general-purpose tasks."
            .to_string(),
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
}
