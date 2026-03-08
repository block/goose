use std::sync::Arc;

use axum::{extract::Path, http::StatusCode, routing::get, Json, Router};
use goose::acp_compat::manifest::{
    AgentManifest, AgentMetadata, AgentModeInfo, AgentStatus, Link, Person,
};

use crate::state::AppState;

/// Build the default Goose agent manifest.
fn goose_agent_manifest() -> AgentManifest {
    AgentManifest {
        name: "Goose".to_string(),
        description: "General-purpose AI agent by Block".to_string(),
        input_content_types: vec!["text/plain".to_string(), "application/json".to_string()],
        output_content_types: vec!["text/plain".to_string(), "application/json".to_string()],
        metadata: Some(AgentMetadata {
            author: Some(Person {
                name: "Block, Inc.".to_string(),
                url: Some("https://block.xyz".to_string()),
            }),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            links: Some(vec![
                Link {
                    url: "https://github.com/block/goose".to_string(),
                    title: Some("GitHub".to_string()),
                },
                Link {
                    url: "https://block.github.io/goose/".to_string(),
                    title: Some("Documentation".to_string()),
                },
            ]),
            recommended_models: Some(vec![
                "claude-sonnet-4-20250514".to_string(),
                "gpt-4.1".to_string(),
            ]),
            dependencies: None,
            annotations: None,
        }),
        status: Some(AgentStatus {
            avg_run_tokens: None,
            avg_run_time_seconds: None,
            success_rate: None,
        }),
        modes: vec![AgentModeInfo {
            id: "default".to_string(),
            name: "Default".to_string(),
            description: Some("General-purpose assistant mode".to_string()),
            tool_groups: vec![],
        }],
        default_mode: Some("default".to_string()),
    }
}

/// GET /acp/ping — health check
async fn ping() -> &'static str {
    "pong"
}

/// GET /acp/agents — list available agents
async fn list_agents() -> Json<Vec<AgentManifest>> {
    Json(vec![goose_agent_manifest()])
}

/// GET /acp/agents/:name — get a specific agent
async fn get_agent(Path(name): Path<String>) -> Result<Json<AgentManifest>, StatusCode> {
    let manifest = goose_agent_manifest();
    if manifest.name.to_lowercase() == name.to_lowercase() {
        Ok(Json(manifest))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    let _state = state;
    Router::new()
        .route("/acp/ping", get(ping))
        .route("/acp/agents", get(list_agents))
        .route("/acp/agents/{name}", get(get_agent))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goose_manifest_has_required_fields() {
        let manifest = goose_agent_manifest();
        assert_eq!(manifest.name, "Goose");
        assert!(!manifest.description.is_empty());
        assert!(manifest.metadata.is_some());
        let meta = manifest.metadata.unwrap();
        assert!(meta.version.is_some());
        assert!(meta.author.is_some());
        assert!(!manifest.modes.is_empty());
    }
}
