//! Per-project agent configuration via `.goose/agents.yaml`.
//!
//! Allows projects to customize agent behavior without code changes:
//! - Enable/disable specific agents
//! - Override agent descriptions (affects semantic routing)
//! - Bind extra extensions to agents
//! - Define custom modes with instructions and tool groups
//! - Set default agent/mode for the project

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const AGENT_CONFIG_FILENAME: &str = "agents.yaml";
const GOOSE_DIR: &str = ".goose";

/// Root configuration loaded from `.goose/agents.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProjectAgentConfig {
    /// Default agent for this project (e.g., "Developer Agent").
    pub default_agent: Option<String>,
    /// Default mode slug for the default agent (e.g., "write").
    pub default_mode: Option<String>,
    /// Per-agent overrides keyed by agent name.
    pub agents: HashMap<String, AgentOverride>,
    /// Custom modes that can be added to any agent.
    pub custom_modes: Vec<CustomModeConfig>,
    /// Routing feedback: learned priors from user corrections.
    #[serde(skip)]
    pub routing_feedback: Vec<RoutingFeedbackEntry>,
}

/// Override configuration for a specific agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentOverride {
    /// Whether this agent is enabled. Default: true.
    pub enabled: Option<bool>,
    /// Override the agent's description (affects semantic routing).
    pub description: Option<String>,
    /// Extra keywords to add to routing (merged with existing).
    pub extra_keywords: Vec<String>,
    /// Additional extensions to bind to this agent.
    pub extra_extensions: Vec<String>,
    /// Override default mode for this agent.
    pub default_mode: Option<String>,
    /// Per-mode overrides.
    pub modes: HashMap<String, ModeOverride>,
}

/// Override configuration for a specific mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModeOverride {
    /// Whether this mode is enabled. Default: true.
    pub enabled: Option<bool>,
    /// Override description.
    pub description: Option<String>,
    /// Override when_to_use hints.
    pub when_to_use: Option<String>,
    /// Additional extensions for this mode.
    pub extra_extensions: Vec<String>,
}

/// User-defined custom mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModeConfig {
    /// Slug identifier (e.g., "data-analysis").
    pub slug: String,
    /// Display name (e.g., "Data Analysis").
    pub name: String,
    /// Description shown in routing catalog.
    pub description: String,
    /// When to use this mode (routing hints).
    pub when_to_use: String,
    /// Which agent(s) this mode applies to. Empty = all agents.
    #[serde(default)]
    pub agents: Vec<String>,
    /// Instructions file path (relative to .goose/) or inline text.
    pub instructions: Option<String>,
    /// Tool groups this mode grants access to.
    #[serde(default)]
    pub tool_groups: Vec<String>,
    /// Extensions recommended for this mode.
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// A routing feedback entry: records when a user corrected a routing decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingFeedbackEntry {
    /// The user message that was routed.
    pub message: String,
    /// The agent that was originally selected.
    pub original_agent: String,
    /// The mode that was originally selected.
    pub original_mode: String,
    /// The agent the user corrected to.
    pub corrected_agent: String,
    /// The mode the user corrected to.
    pub corrected_mode: String,
    /// Timestamp of the correction.
    pub timestamp: String,
}

/// Find the `.goose/` directory by walking up from cwd to git root.
fn find_goose_dir(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        let goose_dir = current.join(GOOSE_DIR);
        if goose_dir.is_dir() {
            return Some(goose_dir);
        }
        // Stop at git root
        if current.join(".git").exists() {
            return None;
        }
        current = current.parent()?;
    }
}

/// Load project agent config from `.goose/agents.yaml`.
/// Returns `None` if the file doesn't exist.
pub fn load_project_agent_config(cwd: &Path) -> Option<ProjectAgentConfig> {
    let goose_dir = find_goose_dir(cwd)?;
    let config_path = goose_dir.join(AGENT_CONFIG_FILENAME);

    if !config_path.is_file() {
        return None;
    }

    let content = std::fs::read_to_string(&config_path).ok()?;
    match serde_yaml::from_str::<ProjectAgentConfig>(&content) {
        Ok(mut config) => {
            // Also load routing feedback if it exists
            let feedback_path = goose_dir.join("routing_feedback.json");
            if feedback_path.is_file() {
                if let Ok(feedback_content) = std::fs::read_to_string(&feedback_path) {
                    if let Ok(feedback) =
                        serde_json::from_str::<Vec<RoutingFeedbackEntry>>(&feedback_content)
                    {
                        config.routing_feedback = feedback;
                    }
                }
            }
            Some(config)
        }
        Err(e) => {
            tracing::warn!("Failed to parse {}: {}", config_path.display(), e);
            None
        }
    }
}

/// Save routing feedback to `.goose/routing_feedback.json`.
pub fn save_routing_feedback(cwd: &Path, feedback: &[RoutingFeedbackEntry]) -> bool {
    let goose_dir = match find_goose_dir(cwd) {
        Some(d) => d,
        None => {
            let d = cwd.join(GOOSE_DIR);
            if std::fs::create_dir_all(&d).is_err() {
                return false;
            }
            d
        }
    };

    let feedback_path = goose_dir.join("routing_feedback.json");
    match serde_json::to_string_pretty(feedback) {
        Ok(json) => std::fs::write(feedback_path, json).is_ok(),
        Err(_) => false,
    }
}

/// Apply project config overrides to an IntentRouter's agent slots.
/// Returns the modified routing feedback entries for later use.
pub fn apply_project_config(
    config: &ProjectAgentConfig,
    slots: &mut [crate::agents::intent_router::AgentSlot],
) {
    for slot in slots.iter_mut() {
        if let Some(agent_override) = config.agents.get(&slot.name) {
            // Enable/disable
            if let Some(enabled) = agent_override.enabled {
                slot.enabled = enabled;
            }

            // Override description
            if let Some(ref desc) = agent_override.description {
                slot.description = desc.clone();
            }

            // Add extra extensions
            if !agent_override.extra_extensions.is_empty() {
                slot.bound_extensions
                    .extend(agent_override.extra_extensions.iter().cloned());
            }

            // Override default mode
            if let Some(ref default_mode) = agent_override.default_mode {
                slot.default_mode = default_mode.clone();
            }

            // Apply mode overrides
            for mode in slot.modes.iter_mut() {
                if let Some(mode_override) = agent_override.modes.get(&mode.slug) {
                    if let Some(ref desc) = mode_override.description {
                        mode.description = desc.clone();
                    }
                    if let Some(ref wtu) = mode_override.when_to_use {
                        mode.when_to_use = Some(wtu.clone());
                    }
                }
            }
        }
    }

    // Add custom modes to the specified agents (or all agents)
    for custom_mode in &config.custom_modes {
        let target_agents: Vec<String> = if custom_mode.agents.is_empty() {
            slots.iter().map(|s| s.name.clone()).collect()
        } else {
            custom_mode.agents.clone()
        };

        for slot in slots.iter_mut() {
            if target_agents.iter().any(|a| a == &slot.name) {
                let mode = crate::registry::manifest::AgentMode {
                    slug: custom_mode.slug.clone(),
                    name: custom_mode.name.clone(),
                    description: custom_mode.description.clone(),
                    when_to_use: Some(custom_mode.when_to_use.clone()),
                    instructions: custom_mode.instructions.clone(),
                    instructions_file: None,
                    tool_groups: custom_mode
                        .tool_groups
                        .iter()
                        .map(|tg| crate::registry::manifest::ToolGroupAccess::Full(tg.clone()))
                        .collect(),
                    is_internal: false,
                    deprecated: None,
                };
                slot.modes.push(mode);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_yaml() -> &'static str {
        r#"
default_agent: "Developer Agent"
default_mode: "write"
agents:
  "Developer Agent":
    enabled: true
    description: "Custom developer description for this project"
    extra_keywords: ["flutter", "dart", "mobile"]
    extra_extensions: ["flutter-tools"]
    modes:
      write:
        when_to_use: "When creating Flutter widgets or Dart code"
  "Research Agent":
    enabled: false
custom_modes:
  - slug: "data-pipeline"
    name: "Data Pipeline"
    description: "Build and debug data pipelines"
    when_to_use: "When working with ETL, data transformation, or pipeline orchestration"
    agents: ["Developer Agent"]
    extensions: ["developer", "memory"]
    tool_groups: ["read", "edit", "command"]
"#
    }

    #[test]
    fn test_parse_project_config() {
        let config: ProjectAgentConfig = serde_yaml::from_str(sample_yaml()).unwrap();
        assert_eq!(config.default_agent.as_deref(), Some("Developer Agent"));
        assert_eq!(config.default_mode.as_deref(), Some("write"));
        assert!(config.agents.contains_key("Developer Agent"));
        assert!(config.agents.contains_key("Research Agent"));
        assert_eq!(config.custom_modes.len(), 1);
        assert_eq!(config.custom_modes[0].slug, "data-pipeline");
    }

    #[test]
    fn test_load_from_file() {
        let tmp = TempDir::new().unwrap();
        let goose_dir = tmp.path().join(".goose");
        std::fs::create_dir(&goose_dir).unwrap();
        std::fs::write(goose_dir.join("agents.yaml"), sample_yaml()).unwrap();

        let config = load_project_agent_config(tmp.path()).unwrap();
        assert_eq!(config.default_agent.as_deref(), Some("Developer Agent"));
        assert_eq!(config.custom_modes.len(), 1);
    }

    #[test]
    fn test_load_missing_file_returns_none() {
        let tmp = TempDir::new().unwrap();
        assert!(load_project_agent_config(tmp.path()).is_none());
    }

    #[test]
    fn test_apply_agent_override() {
        let config: ProjectAgentConfig = serde_yaml::from_str(sample_yaml()).unwrap();
        let mut slots = vec![
            crate::agents::intent_router::AgentSlot {
                name: "Developer Agent".to_string(),
                description: "Original description".to_string(),
                modes: vec![],
                default_mode: "ask".to_string(),
                enabled: true,
                bound_extensions: vec![],
            },
            crate::agents::intent_router::AgentSlot {
                name: "Research Agent".to_string(),
                description: "Research things".to_string(),
                modes: vec![],
                default_mode: "ask".to_string(),
                enabled: true,
                bound_extensions: vec![],
            },
        ];

        apply_project_config(&config, &mut slots);

        // Developer Agent overridden
        assert_eq!(
            slots[0].description,
            "Custom developer description for this project"
        );
        // default_mode remains "ask" — the top-level default_mode is for router fallback only
        assert_eq!(slots[0].default_mode, "ask");
        assert!(slots[0]
            .bound_extensions
            .contains(&"flutter-tools".to_string()));

        // Research Agent disabled
        assert!(!slots[1].enabled);

        // Custom mode added to Developer Agent
        assert!(slots[0].modes.iter().any(|m| m.slug == "data-pipeline"));
        // Custom mode NOT added to Research Agent (agents filter)
        assert!(!slots[1].modes.iter().any(|m| m.slug == "data-pipeline"));
    }

    #[test]
    fn test_save_and_load_routing_feedback() {
        let tmp = TempDir::new().unwrap();
        let goose_dir = tmp.path().join(".goose");
        std::fs::create_dir(&goose_dir).unwrap();
        std::fs::write(goose_dir.join("agents.yaml"), "{}").unwrap();

        let feedback = vec![RoutingFeedbackEntry {
            message: "Fix the login bug".to_string(),
            original_agent: "Goose Agent".to_string(),
            original_mode: "ask".to_string(),
            corrected_agent: "Developer Agent".to_string(),
            corrected_mode: "debug".to_string(),
            timestamp: "2025-03-06T01:00:00Z".to_string(),
        }];

        assert!(save_routing_feedback(tmp.path(), &feedback));

        let config = load_project_agent_config(tmp.path()).unwrap();
        assert_eq!(config.routing_feedback.len(), 1);
        assert_eq!(
            config.routing_feedback[0].corrected_agent,
            "Developer Agent"
        );
    }

    #[test]
    fn test_custom_mode_to_all_agents() {
        let yaml = r#"
custom_modes:
  - slug: "inspect"
    name: "Inspector"
    description: "Deep inspection mode"
    when_to_use: "When user wants thorough analysis"
    agents: []
    extensions: ["developer"]
    tool_groups: ["read"]
"#;
        let config: ProjectAgentConfig = serde_yaml::from_str(yaml).unwrap();
        let mut slots = vec![
            crate::agents::intent_router::AgentSlot {
                name: "Agent A".to_string(),
                description: "".to_string(),
                modes: vec![],
                default_mode: "ask".to_string(),
                enabled: true,
                bound_extensions: vec![],
            },
            crate::agents::intent_router::AgentSlot {
                name: "Agent B".to_string(),
                description: "".to_string(),
                modes: vec![],
                default_mode: "ask".to_string(),
                enabled: true,
                bound_extensions: vec![],
            },
        ];

        apply_project_config(&config, &mut slots);

        // Custom mode added to ALL agents when agents list is empty
        assert!(slots[0].modes.iter().any(|m| m.slug == "inspect"));
        assert!(slots[1].modes.iter().any(|m| m.slug == "inspect"));
    }
}
