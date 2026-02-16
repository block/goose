//! ACP Agent Manifest — describes agent capabilities for discovery.
//!
//! Aligned with ACP v0.2.0 / A2A protocol:
//!   - 1 agent = 1 persona (e.g. "Goose Agent", "Developer Agent")
//!   - Each agent advertises N session modes (e.g. ask, architect, code)
//!   - Modes are switched per-session via `session/setMode`

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// ACP agent manifest per v0.2.0 spec.
///
/// Each manifest represents one agent persona. Modes are listed in `modes`
/// following the ACP SessionMode pattern (not flattened into separate agents).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentManifest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_content_types: Vec<String>,
    #[serde(default)]
    pub output_content_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AgentMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentStatus>,
    /// Session modes this agent supports (ACP SessionMode pattern).
    /// Each mode represents a different behavior/persona the agent can adopt
    /// within a session (e.g. "assistant", "architect", "backend").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<AgentModeInfo>,
    /// The default mode ID when no explicit mode is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<String>,
}

/// A mode an agent can operate in (maps to ACP SessionMode).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentModeInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tool groups this mode has access to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_groups: Vec<String>,
}

/// Runtime status metrics for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_run_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_run_time_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_rate: Option<f64>,
}

/// ACP AgentDependency (experimental) — a tool, agent, or model required by this agent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentDependency {
    #[serde(rename = "type")]
    pub dep_type: String,
    pub name: String,
}

/// Metadata about an agent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Person>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Vec<Link>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<AgentDependency>>,
    /// ACP-REST Option B: discoverable annotations for roles and behavior modes.
    /// Keys follow the convention "goose.<dimension>" to avoid collisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// A person reference.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Person {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A link reference.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Link {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Build the default goose agent manifest.
pub fn goose_agent_manifest() -> AgentManifest {
    AgentManifest {
        name: "goose".to_string(),
        description: "General-purpose AI agent with tool use, powered by Block's Goose framework"
            .to_string(),
        input_content_types: vec!["text/plain".to_string(), "application/json".to_string()],
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
            dependencies: None,
            annotations: None,
        }),
        modes: vec![],
        default_mode: None,
        status: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = goose_agent_manifest();
        let json = serde_json::to_value(&manifest).unwrap();

        assert_eq!(json["name"], "goose");
        assert!(json["description"].as_str().unwrap().contains("AI agent"));
        assert!(json["input_content_types"].is_array());
        assert!(json["output_content_types"].is_array());
        assert_eq!(json["metadata"]["author"]["name"], "Block");
        // status should not be serialized when None
        assert!(json.get("status").is_none());
    }

    #[test]
    fn test_manifest_deserialization() {
        let json = serde_json::json!({
            "name": "custom-agent",
            "description": "A custom agent",
            "input_content_types": ["text/plain"],
            "output_content_types": ["text/plain"]
        });

        let manifest: AgentManifest = serde_json::from_value(json).unwrap();
        assert_eq!(manifest.name, "custom-agent");
        assert!(manifest.metadata.is_none());
        assert!(manifest.status.is_none());
    }

    #[test]
    fn test_manifest_with_status() {
        let json = serde_json::json!({
            "name": "agent",
            "description": "test",
            "status": {
                "avg_run_tokens": 1500.0,
                "success_rate": 0.95
            }
        });

        let manifest: AgentManifest = serde_json::from_value(json).unwrap();
        let status = manifest.status.unwrap();
        assert_eq!(status.avg_run_tokens.unwrap(), 1500.0);
        assert_eq!(status.success_rate.unwrap(), 0.95);
        assert!(status.avg_run_time_seconds.is_none());
    }
}
