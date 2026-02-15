use axum::{routing::get, Json, Router};
use goose::agents::intent_router::IntentRouter;
use goose::registry::formats::{
    A2aAgentCapabilities, A2aAgentCard, A2aAgentInterface, A2aAgentProvider, A2aAgentSkill,
};

pub fn routes() -> Router {
    Router::new()
        .route("/.well-known/agent.json", get(agent_card))
        .route("/.well-known/agent-card.json", get(agent_card))
}

#[utoipa::path(
    get,
    path = "/.well-known/agent-card.json",
    responses(
        (status = 200, description = "A2A Agent Card generated from registered agent personas"),
    ),
    tag = "Discovery"
)]
pub async fn agent_card() -> Json<A2aAgentCard> {
    let card = build_dynamic_agent_card();
    Json(card)
}

fn build_dynamic_agent_card() -> A2aAgentCard {
    let router = IntentRouter::new();

    let skills: Vec<A2aAgentSkill> = router
        .slots()
        .iter()
        .flat_map(|slot| {
            let agent_name = slot.name.clone();
            slot.modes.iter().map(move |mode| A2aAgentSkill {
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
    }
}

fn slugify(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_agent_card_has_skills_from_all_agents() {
        let card = build_dynamic_agent_card();
        assert_eq!(card.name, "Goose");

        // Should have skills from both Goose Agent (7 modes) and Coding Agent (8 modes)
        assert!(
            card.skills.len() >= 15,
            "Expected >= 15 skills, got {}",
            card.skills.len()
        );

        // Check specific skills exist
        let skill_ids: Vec<&str> = card.skills.iter().map(|s| s.id.as_str()).collect();
        assert!(
            skill_ids.contains(&"goose-agent.assistant"),
            "Missing assistant skill"
        );
        assert!(
            skill_ids.contains(&"coding-agent.backend"),
            "Missing backend skill"
        );
        assert!(skill_ids.contains(&"coding-agent.qa"), "Missing qa skill");
    }

    #[test]
    fn test_dynamic_agent_card_has_streaming() {
        let card = build_dynamic_agent_card();
        assert_eq!(card.capabilities.streaming, Some(true));
    }

    #[test]
    fn test_dynamic_agent_card_version() {
        let card = build_dynamic_agent_card();
        assert!(!card.version.is_empty());
    }

    #[test]
    fn test_dynamic_agent_card_serializes() {
        let card = build_dynamic_agent_card();
        let json = serde_json::to_string_pretty(&card).unwrap();
        assert!(json.contains("Goose"));
        assert!(json.contains("acp-rest"));
    }
}
