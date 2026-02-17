//! Agent Card types mapped from a2a.proto: AgentCard, AgentSkill, AgentCapabilities, etc.

use serde::{Deserialize, Serialize};

/// Agent self-describing manifest (proto `AgentCard` message).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_interfaces: Vec<AgentInterface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AgentCapabilities>,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub security_schemes: serde_json::Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_input_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_output_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<AgentSkill>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<AgentCardSignature>,
}

/// Transport interface declaration (proto `AgentInterface` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInterface {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_binding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
}

/// Agent provider information (proto `AgentProvider` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    pub organization: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Agent capability flags (proto `AgentCapabilities` message).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    #[serde(default)]
    pub streaming: bool,
    #[serde(default)]
    pub push_notifications: bool,
    #[serde(default)]
    pub extensions: bool,
    #[serde(default)]
    pub extended_agent_card: bool,
}

/// Agent skill declaration (proto `AgentSkill` message).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security_requirements: Vec<serde_json::Value>,
}

/// JWS signature for agent card verification (proto `AgentCardSignature` message).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCardSignature {
    pub protected: String,
    pub signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_minimal_serde() {
        let card = AgentCard {
            name: "TestAgent".to_string(),
            description: "A test agent".to_string(),
            supported_interfaces: vec![AgentInterface {
                url: "https://example.com/a2a".to_string(),
                protocol_binding: Some("JSONRPC".to_string()),
                tenant: None,
                protocol_version: None,
            }],
            provider: None,
            version: None,
            protocol_version: Some("1.0".to_string()),
            capabilities: Some(AgentCapabilities {
                streaming: true,
                push_notifications: false,
                extensions: false,
                extended_agent_card: false,
            }),
            security_schemes: serde_json::Value::Null,
            security: vec![],
            default_input_modes: vec!["text/plain".to_string()],
            default_output_modes: vec!["text/plain".to_string()],
            skills: vec![AgentSkill {
                id: "echo".to_string(),
                name: "Echo".to_string(),
                description: "Echoes back input".to_string(),
                tags: vec!["test".to_string()],
                examples: vec!["Say hello".to_string()],
                input_modes: vec![],
                output_modes: vec![],
                security_requirements: vec![],
            }],
            documentation_url: None,
            icon_url: None,
            signatures: vec![],
        };

        let json = serde_json::to_value(&card).unwrap();
        assert_eq!(json["name"], "TestAgent");
        assert_eq!(
            json["supportedInterfaces"][0]["url"],
            "https://example.com/a2a"
        );
        assert_eq!(json["capabilities"]["streaming"], true);
        assert_eq!(json["skills"][0]["id"], "echo");

        let deserialized: AgentCard = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, "TestAgent");
        assert_eq!(deserialized.skills.len(), 1);
    }

    #[test]
    fn test_agent_card_from_spec_example() {
        let json_str = r#"{
            "name": "GeoSpatial Route Planner Agent",
            "description": "Plans optimal routes",
            "supportedInterfaces": [
                {
                    "url": "https://api.example.com/a2a/v1",
                    "protocolBinding": "JSONRPC",
                    "protocolVersion": "1.0"
                }
            ],
            "protocolVersion": "1.0",
            "capabilities": {
                "streaming": true,
                "pushNotifications": false,
                "extensions": false,
                "extendedAgentCard": false
            },
            "defaultInputModes": ["text/plain"],
            "defaultOutputModes": ["text/plain", "application/json"],
            "skills": [
                {
                    "id": "route-planning",
                    "name": "Route Planning",
                    "description": "Plans optimal routes between locations",
                    "tags": ["navigation", "routes"]
                }
            ]
        }"#;

        let card: AgentCard = serde_json::from_str(json_str).unwrap();
        assert_eq!(card.name, "GeoSpatial Route Planner Agent");
        assert!(card.capabilities.unwrap().streaming);
        assert_eq!(card.skills[0].id, "route-planning");
    }
}
