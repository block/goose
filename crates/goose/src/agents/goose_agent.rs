//! Built-in Goose agent with specialized behavioral modes.
//!
//! Instead of separate prompt templates loaded ad-hoc by different subsystems,
//! the built-in agent formalizes all Goose behaviors as `BuiltinMode`s.
//! Each mode maps to what was previously a standalone .md prompt template.
//!
//! # Mode Categories
//!
//! 1. **Session modes** — affect the main agent's system prompt
//!    - `assistant` (system.md) — default personality
//!    - `specialist` (specialist.md) — bounded task execution
//!
//! 2. **LLM-only modes** — direct provider.complete() with specialized prompt
//!    - `judge` (permission_judge.md) — read-only detection
//!    - `compactor` — migrated to OrchestratorAgent (compaction is orchestrator-level)
//!    - `app_maker` (apps_create.md) — generate new apps
//!    - `app_iterator` (apps_iterate.md) — update existing apps
//!
//! 3. **Prompt-only modes** — just return a rendered prompt string
//!    - `recipe_maker` (recipe.md) — recipe generation prompt
//!    - `planner` (plan.md) — step-by-step planning prompt
//!
//! # Migration
//!
//! Callers currently use `prompt_template::render_template("foo.md", &ctx)` directly.
//! The migration path:
//! 1. `GooseAgent::mode("judge").render(&ctx)` — same result, but discoverable
//! 2. `GooseAgent::mode("judge").complete(provider, messages)` — encapsulates the LLM call
//! 3. Eventually, modes become ACP SessionModes advertised to clients

use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};
use serde::Serialize;
use std::collections::HashMap;

/// A built-in mode that maps to a prompt template.
#[derive(Debug, Clone)]
pub struct BuiltinMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub template_name: String,
    pub category: ModeCategory,
    pub tool_groups: Vec<ToolGroupAccess>,
    pub recommended_extensions: Vec<String>,
    /// When this mode is most useful (for routing hints).
    pub when_to_use: String,
    /// Internal modes are used by orchestration only, not exposed via ACP/A2A discovery.
    pub is_internal: bool,
}

/// How the mode is executed.
#[derive(Debug, Clone, PartialEq)]
pub enum ModeCategory {
    /// Affects the main agent's system prompt (creates Agent or overrides prompt)
    Session,
    /// Direct LLM call with specialized system prompt (provider.complete)
    LlmOnly,
    /// Just returns a rendered prompt string
    PromptOnly,
}

/// The built-in Goose agent definition.
/// All standard Goose behaviors are modes of this agent.
pub struct GooseAgent {
    modes: HashMap<String, BuiltinMode>,
    default_mode: String,
}

impl Default for GooseAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl GooseAgent {
    pub fn new() -> Self {
        let modes = vec![
            BuiltinMode {
                slug: "assistant".into(),
                name: "🦆 Assistant".into(),
                description: "General-purpose assistant — the default Goose personality".into(),
                template_name: "system.md".into(),
                category: ModeCategory::Session,
                tool_groups: vec![ToolGroupAccess::Full("mcp".into())],
                recommended_extensions: vec!["developer".into(), "memory".into(), "todo".into()],
                when_to_use: "General conversation, Q&A, brainstorming, or any request that doesn't fit a specialized mode".into(),
                is_internal: false,
            },
            BuiltinMode {
                slug: "specialist".into(),
                name: "🔧 Specialist".into(),
                description: "Focused task execution with bounded turns".into(),
                template_name: "specialist.md".into(),
                category: ModeCategory::Session,
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("memory".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("fetch".into()),
                ],
                recommended_extensions: vec!["developer".into(), "memory".into()],
                when_to_use: "Delegated sub-tasks requiring focused execution within bounded scope".into(),
                is_internal: false,
            },
            BuiltinMode {
                slug: "recipe_maker".into(),
                name: "📋 Recipe Maker".into(),
                description: "Generate recipe files from conversations".into(),
                template_name: "recipe.md".into(),
                category: ModeCategory::PromptOnly,
                tool_groups: vec![ToolGroupAccess::Full("none".into())],
                recommended_extensions: vec![],
                when_to_use: "Generating reusable recipe YAML from a conversation".into(),
                is_internal: true,
            },
            BuiltinMode {
                slug: "app_maker".into(),
                name: "🎨 App Creator".into(),
                description: "Create new Goose apps from user instructions".into(),
                template_name: "apps_create.md".into(),
                category: ModeCategory::LlmOnly,
                tool_groups: vec![ToolGroupAccess::Full("apps".into())],
                recommended_extensions: vec!["apps".into()],
                when_to_use: "User asks to create a new standalone HTML/CSS/JS app".into(),
                is_internal: false,
            },
            BuiltinMode {
                slug: "app_iterator".into(),
                name: "🔄 App Iterator".into(),
                description: "Update existing Goose apps based on feedback".into(),
                template_name: "apps_iterate.md".into(),
                category: ModeCategory::LlmOnly,
                tool_groups: vec![ToolGroupAccess::Full("apps".into())],
                recommended_extensions: vec!["apps".into()],
                when_to_use: "User asks to modify or improve an existing Goose app".into(),
                is_internal: false,
            },
            BuiltinMode {
                slug: "judge".into(),
                name: "⚖️ Permission Judge".into(),
                description: "Analyze tool operations for read-only detection".into(),
                template_name: "permission_judge.md".into(),
                category: ModeCategory::LlmOnly,
                tool_groups: vec![ToolGroupAccess::Full("none".into())],
                recommended_extensions: vec![],
                when_to_use: "Internal: classify tool calls as read-only or write for permission gating".into(),
                is_internal: true,
            },
            BuiltinMode {
                slug: "planner".into(),
                name: "🗺️ Planner".into(),
                description: "Create step-by-step execution plans".into(),
                template_name: "plan.md".into(),
                category: ModeCategory::PromptOnly,
                tool_groups: vec![ToolGroupAccess::Full("none".into())],
                recommended_extensions: vec![],
                when_to_use: "Internal: generate step-by-step plans for complex multi-step tasks".into(),
                is_internal: true,
            },
        ];

        let mode_map = modes.into_iter().map(|m| (m.slug.clone(), m)).collect();

        Self {
            modes: mode_map,
            default_mode: "assistant".into(),
        }
    }

    /// Get a mode by slug.
    pub fn mode(&self, slug: &str) -> Option<&BuiltinMode> {
        self.modes.get(slug)
    }

    /// Get the default mode.
    pub fn default_mode(&self) -> &BuiltinMode {
        self.modes
            .get(&self.default_mode)
            .expect("default mode must exist")
    }

    /// List all available modes (including internal).
    pub fn list_modes(&self) -> Vec<&BuiltinMode> {
        let mut modes: Vec<_> = self.modes.values().collect();
        modes.sort_by_key(|m| &m.slug);
        modes
    }

    /// List only user-facing modes (excludes internal orchestration modes).
    pub fn list_public_modes(&self) -> Vec<&BuiltinMode> {
        self.list_modes()
            .into_iter()
            .filter(|m| !m.is_internal)
            .collect()
    }

    /// Convert built-in modes to registry AgentMode format.
    /// This allows built-in modes to be advertised via ACP SessionModeState.
    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        self.list_modes()
            .into_iter()
            .map(|m| AgentMode {
                slug: m.slug.clone(),
                name: m.name.clone(),
                description: m.description.clone(),
                instructions: None,
                instructions_file: Some(m.template_name.clone()),
                tool_groups: m.tool_groups.clone(),
                when_to_use: Some(m.when_to_use.clone()),
                is_internal: m.is_internal,
            })
            .collect()
    }

    /// Convert only user-facing modes to registry AgentMode format.
    /// Internal modes (judge, planner, recipe_maker) are excluded from ACP/A2A discovery.
    pub fn to_public_agent_modes(&self) -> Vec<AgentMode> {
        self.list_public_modes()
            .into_iter()
            .map(|m| AgentMode {
                slug: m.slug.clone(),
                name: m.name.clone(),
                description: m.description.clone(),
                instructions: None,
                instructions_file: Some(m.template_name.clone()),
                tool_groups: m.tool_groups.clone(),
                when_to_use: Some(m.when_to_use.clone()),
                is_internal: false,
            })
            .collect()
    }

    /// Get the default mode slug.
    pub fn default_mode_slug(&self) -> &str {
        &self.default_mode
    }
}

impl BuiltinMode {
    /// Render this mode's template with the given context.
    /// This is the same as calling `prompt_template::render_template` directly,
    /// but makes the mode → template mapping explicit and discoverable.
    pub fn render<T: Serialize>(&self, context: &T) -> anyhow::Result<String> {
        prompt_template::render_template(&self.template_name, context).map_err(|e| {
            anyhow::anyhow!(
                "Failed to render mode '{}' template '{}': {}",
                self.slug,
                self.template_name,
                e
            )
        })
    }

    pub fn is_session_mode(&self) -> bool {
        self.category == ModeCategory::Session
    }

    pub fn is_llm_only(&self) -> bool {
        self.category == ModeCategory::LlmOnly
    }

    pub fn is_prompt_only(&self) -> bool {
        self.category == ModeCategory::PromptOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_agent_has_all_modes() {
        let agent = GooseAgent::new();
        assert_eq!(agent.list_modes().len(), 7);
    }

    #[test]
    fn test_public_modes_excludes_internal() {
        let agent = GooseAgent::new();
        let public = agent.list_public_modes();
        assert_eq!(public.len(), 4); // assistant, specialist, app_maker, app_iterator
        assert!(public.iter().all(|m| !m.is_internal));
        let slugs: Vec<&str> = public.iter().map(|m| m.slug.as_str()).collect();
        assert!(slugs.contains(&"assistant"));
        assert!(slugs.contains(&"specialist"));
        assert!(slugs.contains(&"app_maker"));
        assert!(slugs.contains(&"app_iterator"));
        assert!(!slugs.contains(&"judge"));
        assert!(!slugs.contains(&"planner"));
        assert!(!slugs.contains(&"recipe_maker"));
    }

    #[test]
    fn test_internal_modes_flagged() {
        let agent = GooseAgent::new();
        assert!(agent.mode("judge").unwrap().is_internal);
        assert!(agent.mode("planner").unwrap().is_internal);
        assert!(agent.mode("recipe_maker").unwrap().is_internal);
        assert!(!agent.mode("assistant").unwrap().is_internal);
        assert!(!agent.mode("specialist").unwrap().is_internal);
    }

    #[test]
    fn test_when_to_use_populated() {
        let agent = GooseAgent::new();
        for mode in agent.list_modes() {
            assert!(
                !mode.when_to_use.is_empty(),
                "mode '{}' missing when_to_use",
                mode.slug
            );
        }
    }

    #[test]
    fn test_default_mode_is_assistant() {
        let agent = GooseAgent::new();
        assert_eq!(agent.default_mode_slug(), "assistant");
        assert_eq!(agent.default_mode().template_name, "system.md");
    }

    #[test]
    fn test_mode_lookup() {
        let agent = GooseAgent::new();
        let judge = agent.mode("judge").unwrap();
        assert_eq!(judge.template_name, "permission_judge.md");
        assert!(judge.is_llm_only());
    }

    #[test]
    fn test_specialist_is_session_mode() {
        let agent = GooseAgent::new();
        let specialist = agent.mode("specialist").unwrap();
        assert!(specialist.is_session_mode());
        assert_eq!(specialist.template_name, "specialist.md");
    }

    #[test]
    fn test_planner_is_prompt_only() {
        let agent = GooseAgent::new();
        let planner = agent.mode("planner").unwrap();
        assert!(planner.is_prompt_only());
        assert_eq!(planner.template_name, "plan.md");
    }

    #[test]
    fn test_to_agent_modes_includes_all() {
        let agent = GooseAgent::new();
        let agent_modes = agent.to_agent_modes();
        assert_eq!(agent_modes.len(), 7);
        let assistant = agent_modes.iter().find(|m| m.slug == "assistant").unwrap();
        assert_eq!(assistant.instructions_file.as_deref(), Some("system.md"));
    }

    #[test]
    fn test_to_public_agent_modes_excludes_internal() {
        let agent = GooseAgent::new();
        let public_modes = agent.to_public_agent_modes();
        assert_eq!(public_modes.len(), 4);
        let slugs: Vec<&str> = public_modes.iter().map(|m| m.slug.as_str()).collect();
        assert!(!slugs.contains(&"judge"));
        assert!(!slugs.contains(&"planner"));
        assert!(!slugs.contains(&"recipe_maker"));
    }

    #[test]
    fn test_when_to_use_in_agent_modes() {
        let agent = GooseAgent::new();
        let modes = agent.to_agent_modes();
        for m in &modes {
            assert!(
                m.when_to_use.is_some(),
                "mode '{}' missing when_to_use",
                m.slug
            );
            assert!(!m.when_to_use.as_ref().unwrap().is_empty());
        }
    }

    #[test]
    fn test_render_assistant_mode() {
        let agent = GooseAgent::new();
        let assistant = agent.mode("assistant").unwrap();
        let ctx: HashMap<String, String> = HashMap::new();
        let result = assistant.render(&ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("goose"));
    }

    #[test]
    fn test_nonexistent_mode() {
        let agent = GooseAgent::new();
        assert!(agent.mode("nonexistent").is_none());
    }

    #[test]
    fn test_assistant_has_full_tool_access() {
        let agent = GooseAgent::new();
        let mode = agent.mode("assistant").unwrap();
        assert_eq!(mode.tool_groups.len(), 1);
        assert!(matches!(&mode.tool_groups[0], ToolGroupAccess::Full(name) if name == "mcp"));
    }

    #[test]
    fn test_specialist_has_scoped_tool_access() {
        let agent = GooseAgent::new();
        let mode = agent.mode("specialist").unwrap();
        assert!(mode.tool_groups.len() > 1);
        let group_names: Vec<&str> = mode
            .tool_groups
            .iter()
            .map(|g| match g {
                ToolGroupAccess::Full(name) => name.as_str(),
                ToolGroupAccess::Restricted { group, .. } => group.as_str(),
            })
            .collect();
        assert!(group_names.contains(&"developer"));
        assert!(group_names.contains(&"memory"));
        assert!(group_names.contains(&"command"));
        assert!(!group_names.contains(&"mcp"));
        assert!(!group_names.contains(&"apps"));
    }

    #[test]
    fn test_judge_has_no_tool_access() {
        let agent = GooseAgent::new();
        let mode = agent.mode("judge").unwrap();
        assert_eq!(mode.tool_groups.len(), 1);
        assert!(matches!(&mode.tool_groups[0], ToolGroupAccess::Full(name) if name == "none"));
    }

    #[test]
    fn test_app_maker_only_has_apps_tools() {
        let agent = GooseAgent::new();
        let mode = agent.mode("app_maker").unwrap();
        assert_eq!(mode.tool_groups.len(), 1);
        assert!(matches!(&mode.tool_groups[0], ToolGroupAccess::Full(name) if name == "apps"));
    }

    #[test]
    fn test_planner_has_no_tool_access() {
        let agent = GooseAgent::new();
        let mode = agent.mode("planner").unwrap();
        assert_eq!(mode.tool_groups.len(), 1);
        assert!(matches!(&mode.tool_groups[0], ToolGroupAccess::Full(name) if name == "none"));
    }

    #[test]
    fn test_recipe_maker_has_no_tool_access() {
        let agent = GooseAgent::new();
        let mode = agent.mode("recipe_maker").unwrap();
        assert_eq!(mode.tool_groups.len(), 1);
        assert!(matches!(&mode.tool_groups[0], ToolGroupAccess::Full(name) if name == "none"));
    }

    #[test]
    fn test_tool_groups_exported_in_agent_modes() {
        let agent = GooseAgent::new();
        let agent_modes = agent.to_agent_modes();
        let specialist = agent_modes.iter().find(|m| m.slug == "specialist").unwrap();
        assert!(!specialist.tool_groups.is_empty());
        let judge = agent_modes.iter().find(|m| m.slug == "judge").unwrap();
        assert!(!judge.tool_groups.is_empty());
    }
}
