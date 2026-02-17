//! Build an A2A AgentCard from Goose agent metadata.

use a2a::types::agent_card::{
    AgentCapabilities, AgentCard, AgentInterface, AgentProvider, AgentSkill,
};

/// Build an A2A AgentCard for a Goose agent.
pub fn build_agent_card(
    name: &str,
    description: &str,
    base_url: &str,
    skills: Vec<AgentSkill>,
) -> AgentCard {
    AgentCard {
        name: name.to_string(),
        description: description.to_string(),
        supported_interfaces: vec![AgentInterface {
            url: base_url.to_string(),
            protocol_binding: Some("JSONRPC".to_string()),
            tenant: None,
            protocol_version: Some("1.0".to_string()),
        }],
        provider: Some(AgentProvider {
            organization: "Goose".to_string(),
            url: Some("https://github.com/block/goose".to_string()),
        }),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        protocol_version: Some("1.0".to_string()),
        capabilities: Some(AgentCapabilities {
            streaming: true,
            push_notifications: false,
            extensions: false,
            extended_agent_card: false,
        }),
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string()],
        skills,
        ..Default::default()
    }
}

/// Build an AgentSkill from a name/description pair.
pub fn skill(name: &str, description: &str, tags: Vec<String>) -> AgentSkill {
    AgentSkill {
        id: name.to_lowercase().replace(' ', "-"),
        name: name.to_string(),
        description: description.to_string(),
        tags,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_agent_card() {
        let card = build_agent_card(
            "Goose Agent",
            "A general-purpose AI agent",
            "http://localhost:3000",
            vec![skill("coding", "Write and edit code", vec!["code".into()])],
        );
        assert_eq!(card.name, "Goose Agent");
        assert_eq!(card.description, "A general-purpose AI agent");
        assert_eq!(card.supported_interfaces.len(), 1);
        assert_eq!(card.supported_interfaces[0].url, "http://localhost:3000");
        assert!(card.capabilities.unwrap().streaming);
        assert_eq!(card.skills.len(), 1);
        assert_eq!(card.skills[0].name, "coding");
    }

    #[test]
    fn test_skill_builder() {
        let s = skill("Code Review", "Review code changes", vec!["review".into()]);
        assert_eq!(s.id, "code-review");
        assert_eq!(s.name, "Code Review");
        assert_eq!(s.tags.len(), 1);
    }
}
