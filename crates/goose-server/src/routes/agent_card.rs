use axum::{extract::State, routing::get, Json, Router};
use goose::agents::intent_router::IntentRouter;
use goose::registry::formats::{
    A2aAgentCapabilities, A2aAgentCard, A2aAgentExtension, A2aAgentInterface, A2aAgentProvider,
    A2aAgentSkill,
};
use std::sync::Arc;

use crate::state::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/.well-known/agent.json", get(agent_card))
        .route("/.well-known/agent-card.json", get(agent_card))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/.well-known/agent-card.json",
    responses(
        (status = 200, description = "A2A Agent Card generated from registered agent personas"),
    ),
    tag = "Discovery"
)]
pub async fn agent_card(State(state): State<Arc<AppState>>) -> Json<A2aAgentCard> {
    let card = build_dynamic_agent_card(&state).await;
    Json(card)
}

async fn build_dynamic_agent_card(state: &AppState) -> A2aAgentCard {
    let router = IntentRouter::new();

    let skills: Vec<A2aAgentSkill> = router
        .slots()
        .iter()
        .flat_map(|slot| {
            let agent_name = slot.name.clone();
            slot.modes
                .iter()
                .filter(|mode| !mode.is_internal)
                .map(move |mode| A2aAgentSkill {
                    id: format!("{}.{}", slugify(&agent_name), mode.slug),
                    name: format!("{} — {}", agent_name, mode.name),
                    description: mode.description.clone(),
                    tags: mode
                        .tool_groups
                        .iter()
                        .map(|tg| format!("{tg:?}"))
                        .collect(),
                    examples: Vec::new(),
                })
        })
        .collect();

    // Populate extensions from the live ExtensionRegistry
    let extension_names = state.extension_registry.list_names().await;
    let extensions: Vec<A2aAgentExtension> = extension_names
        .into_iter()
        .map(|name| A2aAgentExtension {
            uri: format!("mcp://{name}"),
            description: Some(format!("MCP extension: {name}")),
            required: false,
        })
        .collect();

    A2aAgentCard {
        name: "Goose".to_string(),
        description: "An open-source AI agent by Block with multi-persona routing. \
                      Supports software development, DevOps, QA, and general-purpose tasks."
            .to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string(), "application/json".to_string()],
        supported_interfaces: vec![A2aAgentInterface {
            url: String::new(),
            protocol_binding: "acp-rest".to_string(),
            protocol_version: "0.2.0".to_string(),
        }],
        skills,
        capabilities: A2aAgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
        },
        provider: Some(A2aAgentProvider {
            organization: "Block, Inc.".to_string(),
            url: "https://github.com/block/goose".to_string(),
        }),
        documentation_url: Some("https://block.github.io/goose/".to_string()),
        icon_url: None,
        security_schemes: Default::default(),
        extensions,
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_card() -> A2aAgentCard {
        // Build card without AppState — use empty extensions
        let router = IntentRouter::new();

        let skills: Vec<A2aAgentSkill> = router
            .slots()
            .iter()
            .flat_map(|slot| {
                let agent_name = slot.name.clone();
                slot.modes
                    .iter()
                    .filter(|mode| !mode.is_internal)
                    .map(move |mode| A2aAgentSkill {
                        id: format!("{}.{}", slugify(&agent_name), mode.slug),
                        name: format!("{} — {}", agent_name, mode.name),
                        description: mode.description.clone(),
                        tags: Vec::new(),
                        examples: Vec::new(),
                    })
            })
            .collect();

        A2aAgentCard {
            name: "Goose".to_string(),
            description: "Test agent card".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            supported_interfaces: vec![A2aAgentInterface {
                url: String::new(),
                protocol_binding: "acp-rest".to_string(),
                protocol_version: "0.2.0".to_string(),
            }],
            skills,
            capabilities: A2aAgentCapabilities {
                streaming: Some(true),
                push_notifications: Some(false),
            },
            provider: Some(A2aAgentProvider {
                organization: "Block, Inc.".to_string(),
                url: "https://github.com/block/goose".to_string(),
            }),
            documentation_url: Some("https://block.github.io/goose/".to_string()),
            icon_url: None,
            security_schemes: Default::default(),
            extensions: Vec::new(),
        }
    }

    #[test]
    fn test_dynamic_agent_card_has_skills_from_all_agents() {
        let card = build_test_card();
        assert_eq!(card.name, "Goose");

        assert!(
            card.skills.len() >= 9,
            "Expected >= 9 public skills, got {}",
            card.skills.len()
        );

        let skill_ids: Vec<&str> = card.skills.iter().map(|s| s.id.as_str()).collect();
        assert!(skill_ids.contains(&"goose-agent.ask"), "Missing ask skill");
        assert!(
            skill_ids.contains(&"developer-agent.write"),
            "Missing write skill"
        );

        assert!(
            !skill_ids.contains(&"goose-agent.judge"),
            "Internal mode 'judge' should not be an A2A skill"
        );
        assert!(
            !skill_ids.contains(&"goose-agent.planner"),
            "Internal mode 'planner' should not be an A2A skill"
        );
        assert!(
            !skill_ids.contains(&"goose-agent.recipe_maker"),
            "Internal mode 'recipe_maker' should not be an A2A skill"
        );
    }

    #[test]
    fn test_dynamic_agent_card_has_streaming() {
        let card = build_test_card();
        assert_eq!(card.capabilities.streaming, Some(true));
    }

    #[test]
    fn test_dynamic_agent_card_version() {
        let card = build_test_card();
        assert!(!card.version.is_empty());
    }

    #[test]
    fn test_dynamic_agent_card_serializes() {
        let card = build_test_card();
        let json = serde_json::to_string_pretty(&card).unwrap();
        assert!(json.contains("Goose"));
        assert!(json.contains("acp-rest"));
    }

    #[test]
    fn test_dynamic_agent_card_extensions_field() {
        let card = build_test_card();
        assert!(card.extensions.is_empty());

        // Verify extensions serialize correctly when populated
        let mut card_with_ext = card;
        card_with_ext.extensions.push(A2aAgentExtension {
            uri: "mcp://developer".to_string(),
            description: Some("Developer tools".to_string()),
            required: true,
        });
        let json = serde_json::to_string_pretty(&card_with_ext).unwrap();
        assert!(json.contains("mcp://developer"));
        assert!(json.contains("\"required\": true"));
    }
}
