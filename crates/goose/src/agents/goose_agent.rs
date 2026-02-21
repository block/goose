//! Goose Agent — the general-purpose catch-all assistant.
//!
//! GooseAgent is the default persona: a helpful, honest, resourceful AI partner.
//! It handles anything not routed to a specialist agent (Developer, QA, PM,
//! Security, Research).
//!
//! ## Universal Modes (public)
//! - `ask` (default) — interactive exploration, read-only
//! - `plan` — strategic reasoning and planning, read-only
//! - `write` — active creation with full tool access
//! - `review` — evaluative analysis, read-only
//!
//! ## Internal Modes (not exposed to ACP/A2A)
//! - `judge` — permission gating (LlmOnly)
//! - `planner` — step-by-step plan generation (PromptOnly)
//! - `recipe_maker` — recipe YAML generation (PromptOnly)
//! - `app_maker` — create Goose apps (LlmOnly)
//! - `app_iterator` — iterate Goose apps (LlmOnly)

use std::collections::HashMap;

use crate::agents::universal_mode::UniversalMode;
use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

/// Category of a builtin mode — determines how it's invoked.
#[derive(Debug, Clone, PartialEq)]
pub enum ModeCategory {
    /// Affects the system prompt for the main conversation loop.
    Session,
    /// Direct LLM call outside the main loop (e.g., judge, app_maker).
    LlmOnly,
    /// Returns rendered prompt text without calling an LLM.
    PromptOnly,
}

/// A built-in mode for GooseAgent.
#[derive(Debug, Clone)]
pub struct BuiltinMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub template_name: String,
    pub category: ModeCategory,
    pub tool_groups: Vec<ToolGroupAccess>,
    pub recommended_extensions: Vec<String>,
    pub when_to_use: String,
    pub is_internal: bool,
    pub deprecated: Option<String>,
}

/// GooseAgent — general-purpose assistant with universal + internal modes.
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
    #[allow(clippy::vec_init_then_push)]
    pub fn new() -> Self {
        let mut modes = Vec::new();

        // ── Universal public modes ───────────────────────────────────

        // Ask (default) — interactive exploration, read-only
        modes.push(BuiltinMode {
            slug: UniversalMode::Ask.slug().to_string(),
            name: UniversalMode::Ask.display_name().to_string(),
            description: "General-purpose Q&A, exploration, and information retrieval".into(),
            template_name: "goose/ask.md".into(),
            category: ModeCategory::Session,
            tool_groups: vec![
                ToolGroupAccess::Full("developer".into()),
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
                ToolGroupAccess::Full("mcp".into()),
            ],
            recommended_extensions: vec!["developer".into(), "memory".into(), "fetch".into()],
            when_to_use:
                "General questions, exploration, explanation, information retrieval, conversation"
                    .into(),
            is_internal: false,
            deprecated: None,
        });

        // Plan — strategic reasoning, read-only
        modes.push(BuiltinMode {
            slug: UniversalMode::Plan.slug().to_string(),
            name: UniversalMode::Plan.display_name().to_string(),
            description: "Strategic planning, problem decomposition, and decision-making".into(),
            template_name: "goose/plan.md".into(),
            category: ModeCategory::Session,
            tool_groups: vec![
                ToolGroupAccess::Full("developer".into()),
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
            recommended_extensions: vec!["developer".into(), "memory".into(), "fetch".into()],
            when_to_use: "Planning, strategy, breaking down problems, roadmaps, decision-making"
                .into(),
            is_internal: false,
            deprecated: None,
        });

        // Write — active creation, full access
        modes.push(BuiltinMode {
            slug: UniversalMode::Write.slug().to_string(),
            name: UniversalMode::Write.display_name().to_string(),
            description: "Create and modify files, documents, configurations".into(),
            template_name: "goose/write.md".into(),
            category: ModeCategory::Session,
            tool_groups: vec![
                ToolGroupAccess::Full("developer".into()),
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("edit".into()),
                ToolGroupAccess::Full("command".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
                ToolGroupAccess::Full("mcp".into()),
            ],
            recommended_extensions: vec!["developer".into(), "memory".into()],
            when_to_use:
                "Writing documents, creating files, editing configurations, generating content"
                    .into(),
            is_internal: false,
            deprecated: None,
        });

        // Review — evaluative analysis, read-only
        modes.push(BuiltinMode {
            slug: UniversalMode::Review.slug().to_string(),
            name: UniversalMode::Review.display_name().to_string(),
            description: "Review and evaluate code, documents, or configurations".into(),
            template_name: "goose/review.md".into(),
            category: ModeCategory::Session,
            tool_groups: vec![
                ToolGroupAccess::Full("developer".into()),
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("command".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
            recommended_extensions: vec!["developer".into(), "memory".into()],
            when_to_use: "Reviewing code, evaluating documents, checking quality, auditing".into(),
            is_internal: false,
            deprecated: None,
        });

        // ── Internal modes (not exposed to ACP/A2A) ─────────────────

        modes.push(BuiltinMode {
            slug: "recipe_maker".into(),
            name: "📋 Recipe Maker".into(),
            description: "Generate recipe files from conversations".into(),
            template_name: "recipe.md".into(),
            category: ModeCategory::PromptOnly,
            tool_groups: vec![ToolGroupAccess::Full("none".into())],
            recommended_extensions: vec![],
            when_to_use: "Generating reusable recipe YAML from a conversation".into(),
            is_internal: true,
            deprecated: None,
        });

        modes.push(BuiltinMode {
            slug: "app_maker".into(),
            name: "🎨 App Creator".into(),
            description: "Create new Goose apps from user instructions".into(),
            template_name: "apps_create.md".into(),
            category: ModeCategory::LlmOnly,
            tool_groups: vec![ToolGroupAccess::Full("apps".into())],
            recommended_extensions: vec!["apps".into()],
            when_to_use: "User asks to build a standalone interactive HTML/CSS/JS application, game, tool, or utility that opens in its own window".into(),
            is_internal: false,
            deprecated: None,
        });

        modes.push(BuiltinMode {
            slug: "app_iterator".into(),
            name: "🔄 App Iterator".into(),
            description: "Update existing Goose apps based on feedback".into(),
            template_name: "apps_iterate.md".into(),
            category: ModeCategory::LlmOnly,
            tool_groups: vec![ToolGroupAccess::Full("apps".into())],
            recommended_extensions: vec!["apps".into()],
            when_to_use: "User asks to modify or improve an existing Goose app".into(),
            is_internal: false,
            deprecated: None,
        });

        modes.push(BuiltinMode {
            slug: "genui".into(),
            name: "📊 Data Visualizer".into(),
            description: "Visualize data with inline charts, dashboards, and metrics".into(),
            template_name: "genui.md".into(),
            category: ModeCategory::Session,
            tool_groups: vec![
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("command".into()),
            ],
            recommended_extensions: vec!["genui".into(), "developer".into()],
            when_to_use: "User asks to visualize, chart, graph, or show data as a dashboard, overview, or summary with graphics inline in chat".into(),
            is_internal: false,
            deprecated: None,
        });

        modes.push(BuiltinMode {
            slug: "judge".into(),
            name: "⚖️ Permission Judge".into(),
            description: "Analyze tool operations for read-only detection".into(),
            template_name: "permission_judge.md".into(),
            category: ModeCategory::LlmOnly,
            tool_groups: vec![ToolGroupAccess::Full("none".into())],
            recommended_extensions: vec![],
            when_to_use:
                "Internal: classify tool calls as read-only or write for permission gating".into(),
            is_internal: true,
            deprecated: None,
        });

        modes.push(BuiltinMode {
            slug: "planner".into(),
            name: "🗺️ Planner".into(),
            description: "Create step-by-step execution plans".into(),
            template_name: "plan.md".into(),
            category: ModeCategory::PromptOnly,
            tool_groups: vec![ToolGroupAccess::Full("none".into())],
            recommended_extensions: vec![],
            when_to_use: "Internal: generate step-by-step plans for complex multi-step tasks"
                .into(),
            is_internal: true,
            deprecated: None,
        });

        let mode_map = modes.into_iter().map(|m| (m.slug.clone(), m)).collect();

        Self {
            modes: mode_map,
            default_mode: "ask".into(),
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&BuiltinMode> {
        self.modes.get(slug)
    }

    pub fn default_mode_obj(&self) -> &BuiltinMode {
        self.modes
            .get(&self.default_mode)
            .expect("default mode must exist")
    }

    pub fn default_mode_slug(&self) -> &str {
        &self.default_mode
    }

    pub fn list_modes(&self) -> Vec<&BuiltinMode> {
        let mut modes: Vec<_> = self.modes.values().collect();
        modes.sort_by_key(|m| &m.slug);
        modes
    }

    pub fn list_public_modes(&self) -> Vec<&BuiltinMode> {
        self.list_modes()
            .into_iter()
            .filter(|m| !m.is_internal)
            .collect()
    }

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
                deprecated: m.deprecated.clone(),
            })
            .collect()
    }

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
                deprecated: m.deprecated.clone(),
            })
            .collect()
    }

    pub fn render_mode(
        &self,
        slug: &str,
        context: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<String> {
        let mode = self
            .mode(slug)
            .ok_or_else(|| anyhow::anyhow!("Unknown mode: {}", slug))?;
        Ok(prompt_template::render_template(
            &mode.template_name,
            context,
        )?)
    }

    /// Categorize modes for routing.
    pub fn session_modes(&self) -> Vec<&BuiltinMode> {
        self.list_modes()
            .into_iter()
            .filter(|m| m.category == ModeCategory::Session)
            .collect()
    }

    pub fn llm_only_modes(&self) -> Vec<&BuiltinMode> {
        self.list_modes()
            .into_iter()
            .filter(|m| m.category == ModeCategory::LlmOnly)
            .collect()
    }

    pub fn prompt_only_modes(&self) -> Vec<&BuiltinMode> {
        self.list_modes()
            .into_iter()
            .filter(|m| m.category == ModeCategory::PromptOnly)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_ask() {
        let agent = GooseAgent::new();
        assert_eq!(agent.default_mode_slug(), "ask");
        assert!(agent.mode("ask").is_some());
    }

    #[test]
    fn test_universal_public_modes_present() {
        let agent = GooseAgent::new();
        assert!(agent.mode("ask").is_some(), "Missing ask mode");
        assert!(agent.mode("plan").is_some(), "Missing plan mode");
        assert!(agent.mode("write").is_some(), "Missing write mode");
        assert!(agent.mode("review").is_some(), "Missing review mode");
    }

    #[test]
    fn test_internal_modes_present() {
        let agent = GooseAgent::new();
        assert!(agent.mode("judge").is_some(), "Missing judge");
        assert!(agent.mode("planner").is_some(), "Missing planner");
        assert!(agent.mode("recipe_maker").is_some(), "Missing recipe_maker");
        assert!(agent.mode("app_maker").is_some(), "Missing app_maker");
        assert!(agent.mode("app_iterator").is_some(), "Missing app_iterator");
        assert!(agent.mode("genui").is_some(), "Missing genui");
    }

    #[test]
    fn test_public_modes_exclude_internal() {
        let agent = GooseAgent::new();
        let public = agent.list_public_modes();
        let slugs: Vec<&str> = public.iter().map(|m| m.slug.as_str()).collect();

        // Universal modes are public
        assert!(slugs.contains(&"ask"));
        assert!(slugs.contains(&"plan"));
        assert!(slugs.contains(&"write"));
        assert!(slugs.contains(&"review"));

        // App modes are public too
        assert!(slugs.contains(&"app_maker"));
        assert!(slugs.contains(&"app_iterator"));

        // Internal modes excluded
        assert!(!slugs.contains(&"judge"));
        assert!(!slugs.contains(&"planner"));
        assert!(!slugs.contains(&"recipe_maker"));
    }

    #[test]
    fn test_total_mode_count() {
        let agent = GooseAgent::new();
        // 4 universal + 5 internal/app + 1 genui = 10 total
        assert_eq!(agent.list_modes().len(), 10);
        // 4 universal + 2 app + 1 genui = 7 public
        assert_eq!(agent.list_public_modes().len(), 7);
    }

    #[test]
    fn test_ask_mode_is_read_only() {
        let agent = GooseAgent::new();
        let ask = agent.mode("ask").unwrap();
        let groups: Vec<String> = ask
            .tool_groups
            .iter()
            .map(|tg| format!("{:?}", tg))
            .collect();
        assert!(groups.iter().any(|g| g.contains("read")));
        assert!(!groups.iter().any(|g| g.contains("edit")));
        assert!(!groups.iter().any(|g| g.contains("command")));
    }

    #[test]
    fn test_write_mode_has_full_access() {
        let agent = GooseAgent::new();
        let write = agent.mode("write").unwrap();
        let groups: Vec<String> = write
            .tool_groups
            .iter()
            .map(|tg| format!("{:?}", tg))
            .collect();
        assert!(groups.iter().any(|g| g.contains("edit")));
        assert!(groups.iter().any(|g| g.contains("command")));
    }

    #[test]
    fn test_old_modes_removed() {
        let agent = GooseAgent::new();
        assert!(
            agent.mode("assistant").is_none(),
            "assistant should be removed"
        );
        assert!(
            agent.mode("specialist").is_none(),
            "specialist should be removed"
        );
    }

    #[test]
    fn test_agent_mode_conversion() {
        let agent = GooseAgent::new();
        let modes = agent.to_agent_modes();
        assert_eq!(modes.len(), 10);
        assert!(modes.iter().all(|m| m.when_to_use.is_some()));
    }

    #[test]
    fn test_public_agent_mode_conversion() {
        let agent = GooseAgent::new();
        let modes = agent.to_public_agent_modes();
        assert_eq!(modes.len(), 7);
        assert!(modes.iter().all(|m| !m.is_internal));
    }

    #[test]
    fn test_render_mode() {
        let agent = GooseAgent::new();
        let ctx = HashMap::new();
        let result = agent.render_mode("ask", &ctx);
        assert!(result.is_ok(), "Failed to render ask mode");
        let text = result.unwrap();
        assert!(text.contains("Goose"), "Ask prompt should mention Goose");
    }

    #[test]
    fn test_session_modes() {
        let agent = GooseAgent::new();
        let session = agent.session_modes();
        assert_eq!(session.len(), 5); // ask, plan, write, review, genui
        assert!(session.iter().all(|m| m.category == ModeCategory::Session));
    }

    #[test]
    fn test_llm_only_modes() {
        let agent = GooseAgent::new();
        let llm = agent.llm_only_modes();
        // app_maker, app_iterator, judge = 3
        assert_eq!(llm.len(), 3);
    }

    #[test]
    fn test_prompt_only_modes() {
        let agent = GooseAgent::new();
        let prompt = agent.prompt_only_modes();
        // recipe_maker, planner = 2
        assert_eq!(prompt.len(), 2);
    }
}
