//! ACP v0.2.0 discovery and compatibility endpoints.
//!
//! Aligned with ACP / A2A protocol: 1 agent = 1 persona with N session modes.
//!   - Goose Agent: general-purpose agent (modes: assistant, specialist, recipe_maker, …)
//!   - Coding Agent: software engineering agent (modes: pm, architect, backend, …)
//!
//! Modes are switched per-session via `session/setMode`, NOT flattened into separate agents.
//!
//! Provides: GET /ping, GET /agents, GET /agents/{name}, GET /session/{id}

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use goose::acp_compat::{
    AcpSession, AgentDependency, AgentManifest, AgentMetadata, AgentModeInfo, Link, Person,
};
use goose::agents::intent_router::IntentRouter;

use crate::state::AppState;

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AgentsListResponse {
    agents: Vec<AgentManifest>,
}

/// ACP v0.2.0 GET /ping
#[utoipa::path(get, path = "/ping",
    tag = "ACP Discovery",
    responses(
        (status = 200, description = "Health check", body = serde_json::Value),
    )
)]
async fn ping() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

/// Slugify an agent name for use in URLs (RFC 1123 DNS label).
fn slugify_agent_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Resolve a mode slug to its parent agent.
/// e.g. "backend" → Some(("Coding Agent", "backend"))
pub fn resolve_mode_to_agent(mode_slug: &str) -> Option<(String, String)> {
    let router = IntentRouter::new();
    for slot in router.slots() {
        for mode in &slot.modes {
            if mode.slug == mode_slug {
                return Some((slot.name.clone(), mode.slug.clone()));
            }
        }
    }
    None
}

/// Build one AgentManifest per agent persona (NOT per mode).
///
/// Each agent lists its modes in the `modes` field, aligned with ACP SessionMode.
fn build_agent_manifests() -> Vec<AgentManifest> {
    let router = IntentRouter::new();
    let mut manifests = Vec::new();

    for slot in router.slots() {
        let slug = slugify_agent_name(&slot.name);

        let modes: Vec<AgentModeInfo> = slot
            .modes
            .iter()
            .filter(|mode| !mode.is_internal)
            .map(|mode| {
                let tool_groups: Vec<String> = mode
                    .tool_groups
                    .iter()
                    .filter_map(|tg| {
                        let name = match tg {
                            goose::registry::manifest::ToolGroupAccess::Full(n) => n,
                            goose::registry::manifest::ToolGroupAccess::Restricted {
                                group,
                                ..
                            } => group,
                        };
                        if name == "none" {
                            None
                        } else {
                            Some(name.clone())
                        }
                    })
                    .collect();

                AgentModeInfo {
                    id: mode.slug.clone(),
                    name: mode.name.clone(),
                    description: Some(mode.description.clone()),
                    tool_groups,
                }
            })
            .collect();

        let all_deps: Vec<AgentDependency> = modes
            .iter()
            .flat_map(|m| m.tool_groups.iter())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .map(|name| AgentDependency {
                dep_type: "tool".to_string(),
                name: name.clone(),
            })
            .collect();

        manifests.push(AgentManifest {
            name: slug,
            description: slot.description.clone(),
            input_content_types: vec![
                "text/plain".to_string(),
                "image/png".to_string(),
                "image/jpeg".to_string(),
                "application/json".to_string(),
            ],
            output_content_types: vec![
                "text/plain".to_string(),
                "application/json".to_string(),
                "image/*".to_string(),
            ],
            metadata: Some(AgentMetadata {
                author: Some(Person {
                    name: "Block".to_string(),
                    url: Some("https://block.xyz".to_string()),
                }),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                links: Some(vec![Link {
                    url: "https://github.com/block/goose".to_string(),
                    title: Some("GitHub".to_string()),
                }]),
                recommended_models: None,
                dependencies: if all_deps.is_empty() {
                    None
                } else {
                    Some(all_deps)
                },
                annotations: None,
            }),
            default_mode: Some(slot.default_mode.clone()),
            modes,
            status: None,
        });
    }

    manifests
}

/// ACP v0.2.0 GET /agents — one manifest per agent persona
#[utoipa::path(get, path = "/agents",
    tag = "ACP Discovery",
    responses(
        (status = 200, description = "List available agents", body = AgentsListResponse),
    )
)]
async fn list_agents() -> Json<AgentsListResponse> {
    Json(AgentsListResponse {
        agents: build_agent_manifests(),
    })
}

/// ACP v0.2.0 GET /agents/{name}
#[utoipa::path(get, path = "/agents/{name}",
    tag = "ACP Discovery",
    params(("name" = String, Path, description = "Agent slug (e.g. goose-agent, coding-agent)")),
    responses(
        (status = 200, description = "Agent manifest", body = AgentManifest),
        (status = 404, description = "Agent not found"),
    )
)]
async fn get_agent(
    Path(name): Path<String>,
) -> Result<Json<AgentManifest>, axum::http::StatusCode> {
    build_agent_manifests()
        .into_iter()
        .find(|a| a.name == name)
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

/// ACP-compatible GET /session/{session_id} — returns ACP Session schema.
#[utoipa::path(get, path = "/session/{session_id}",
    tag = "ACP Sessions",
    params(("session_id" = String, Path, description = "Session ID")),
    responses(
        (status = 200, description = "ACP session view", body = AcpSession),
        (status = 404, description = "Session not found"),
    )
)]
async fn get_acp_session(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Result<Json<AcpSession>, axum::http::StatusCode> {
    let session = state
        .session_manager()
        .get_session(&session_id, true)
        .await
        .map_err(|_| axum::http::StatusCode::NOT_FOUND)?;

    let history: Vec<String> = state
        .run_store()
        .list(1000, 0)
        .await
        .into_iter()
        .filter(|r| r.session_id.as_deref() == Some(&session_id))
        .map(|r| format!("/runs/{}/events", r.run_id))
        .collect();

    Ok(Json(AcpSession {
        id: session.id.clone(),
        history,
        state: None,
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/agents", get(list_agents))
        .route("/agents/{name}", get(get_agent))
        .route("/session/{session_id}", get(get_acp_session))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_agent_name() {
        assert_eq!(slugify_agent_name("Goose Agent"), "goose-agent");
        assert_eq!(slugify_agent_name("Coding Agent"), "coding-agent");
        assert_eq!(slugify_agent_name("My Custom Agent"), "my-custom-agent");
    }

    #[test]
    fn test_build_agent_manifests_returns_agents_not_modes() {
        let manifests = build_agent_manifests();
        // Should have 6 agents (Goose, Coding, QA, PM, Security, Research)
        assert!(
            manifests.len() >= 6,
            "Expected >= 6 agent personas, got {}",
            manifests.len()
        );

        let names: Vec<_> = manifests.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"goose-agent"), "Missing goose-agent");
        assert!(names.contains(&"coding-agent"), "Missing coding-agent");
    }

    #[test]
    fn test_goose_agent_has_modes() {
        let manifests = build_agent_manifests();
        let goose = manifests.iter().find(|m| m.name == "goose-agent").unwrap();

        // Goose Agent: 7 total modes, 3 internal (judge, planner, recipe_maker) → 4 public
        assert!(
            goose.modes.len() >= 4,
            "Expected >= 4 public modes for Goose Agent, got {}",
            goose.modes.len()
        );

        // Internal modes must NOT appear in ACP discovery
        let mode_ids: Vec<_> = goose.modes.iter().map(|m| m.id.as_str()).collect();
        assert!(
            !mode_ids.contains(&"judge"),
            "Internal mode 'judge' should not be exposed"
        );
        assert!(
            !mode_ids.contains(&"planner"),
            "Internal mode 'planner' should not be exposed"
        );
        assert!(
            !mode_ids.contains(&"recipe_maker"),
            "Internal mode 'recipe_maker' should not be exposed"
        );

        // Public modes must be present
        assert!(mode_ids.contains(&"assistant"), "Missing assistant mode");
        assert!(mode_ids.contains(&"specialist"), "Missing specialist mode");
        assert_eq!(
            goose.default_mode.as_deref(),
            Some("assistant"),
            "Default mode should be assistant"
        );
    }

    #[test]
    fn test_coding_agent_has_modes() {
        let manifests = build_agent_manifests();
        let coding = manifests.iter().find(|m| m.name == "coding-agent").unwrap();

        // Coding Agent now has 5 focused modes (code, architect, frontend, debug, devops)
        assert!(
            coding.modes.len() >= 5,
            "Expected >= 5 modes for Coding Agent, got {}",
            coding.modes.len()
        );

        let mode_ids: Vec<_> = coding.modes.iter().map(|m| m.id.as_str()).collect();
        assert!(mode_ids.contains(&"code"), "Missing code mode");
        assert!(mode_ids.contains(&"frontend"), "Missing frontend mode");
        assert!(mode_ids.contains(&"architect"), "Missing architect mode");
        assert!(mode_ids.contains(&"debug"), "Missing debug mode");
        assert!(mode_ids.contains(&"devops"), "Missing devops mode");
    }

    #[test]
    fn test_modes_have_tool_groups() {
        let manifests = build_agent_manifests();
        let coding = manifests.iter().find(|m| m.name == "coding-agent").unwrap();

        let code = coding.modes.iter().find(|m| m.id == "code").unwrap();
        assert!(
            !code.tool_groups.is_empty(),
            "Code mode should have tool groups"
        );
    }

    #[test]
    fn test_resolve_mode_to_agent() {
        let result = resolve_mode_to_agent("code");
        assert!(result.is_some());
        let (slot, mode) = result.unwrap();
        assert_eq!(slot, "Coding Agent");
        assert_eq!(mode, "code");

        let result = resolve_mode_to_agent("assistant");
        assert!(result.is_some());
        let (slot, mode) = result.unwrap();
        assert_eq!(slot, "Goose Agent");
        assert_eq!(mode, "assistant");

        assert!(resolve_mode_to_agent("nonexistent").is_none());
    }
}
