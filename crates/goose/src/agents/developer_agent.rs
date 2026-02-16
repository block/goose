/// Developer Agent — software engineering persona.
///
/// The Developer Agent is responsible for writing, debugging, reviewing, and
/// planning code. It uses the universal mode set (ask/plan/write/review/debug)
/// where each mode controls behavioral stance and tool access.
///
/// This agent replaces the former `CodingAgent` which conflated personas
/// (PM, QA, Security) with behavioral modes.
use std::collections::HashMap;

use crate::agents::universal_mode::UniversalMode;
use crate::prompt_template;
use crate::registry::manifest::AgentMode;

/// Additional tool groups the Developer persona adds on top of UniversalMode defaults.
fn developer_extra_tools(mode: &UniversalMode) -> Vec<crate::registry::manifest::ToolGroupAccess> {
    use crate::registry::manifest::ToolGroupAccess;
    match mode {
        // Ask mode: developer adds code_execution for REPL exploration
        UniversalMode::Ask => vec![ToolGroupAccess::Full("code_execution".into())],
        // Plan mode: developer can sketch in temp files
        UniversalMode::Plan => vec![ToolGroupAccess::Full("code_execution".into())],
        // Write mode: developer adds MCP, browser, code_execution
        UniversalMode::Write => vec![
            ToolGroupAccess::Full("mcp".into()),
            ToolGroupAccess::Full("code_execution".into()),
        ],
        // Review mode: developer adds code_execution for running checks
        UniversalMode::Review => vec![
            ToolGroupAccess::Full("command".into()),
            ToolGroupAccess::Full("code_execution".into()),
        ],
        // Debug mode: developer adds MCP, code_execution
        UniversalMode::Debug => vec![
            ToolGroupAccess::Full("mcp".into()),
            ToolGroupAccess::Full("code_execution".into()),
        ],
    }
}

/// Recommended MCP extensions per mode.
fn recommended_extensions(mode: &UniversalMode) -> Vec<&'static str> {
    match mode {
        UniversalMode::Ask => vec!["developer", "context7", "memory"],
        UniversalMode::Plan => vec!["developer", "context7", "memory", "fetch"],
        UniversalMode::Write => vec![
            "developer",
            "github",
            "context7",
            "memory",
            "code_execution",
        ],
        UniversalMode::Review => vec!["developer", "github", "memory"],
        UniversalMode::Debug => vec![
            "developer",
            "github",
            "context7",
            "memory",
            "code_execution",
        ],
    }
}

pub struct DeveloperAgent {
    modes: HashMap<String, DeveloperMode>,
    default_mode: String,
}

struct DeveloperMode {
    mode: UniversalMode,
    extra_tools: Vec<crate::registry::manifest::ToolGroupAccess>,
    recommended_extensions: Vec<&'static str>,
}

impl Default for DeveloperAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DeveloperAgent {
    pub fn new() -> Self {
        let mut modes = HashMap::new();

        for um in UniversalMode::all() {
            modes.insert(
                um.slug().to_string(),
                DeveloperMode {
                    mode: *um,
                    extra_tools: developer_extra_tools(um),
                    recommended_extensions: recommended_extensions(um),
                },
            );
        }

        Self {
            modes,
            default_mode: "write".to_string(),
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&UniversalMode> {
        self.modes.get(slug).map(|dm| &dm.mode)
    }

    pub fn default_mode(&self) -> &str {
        &self.default_mode
    }

    pub fn modes(&self) -> Vec<&str> {
        let mut slugs: Vec<&str> = self.modes.keys().map(|s| s.as_str()).collect();
        // Stable ordering: ask, plan, write, review, debug
        slugs.sort_by_key(|s| {
            UniversalMode::all()
                .iter()
                .position(|m| m.slug() == *s)
                .unwrap_or(99)
        });
        slugs
    }

    pub fn render_mode(
        &self,
        slug: &str,
        context: &HashMap<String, String>,
    ) -> anyhow::Result<String> {
        let dm = self
            .modes
            .get(slug)
            .ok_or_else(|| anyhow::anyhow!("Unknown Developer Agent mode: {slug}"))?;
        let template_name = format!("developer/{}.md", dm.mode.slug());
        Ok(prompt_template::render_template(&template_name, context)?)
    }

    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        let mut result: Vec<AgentMode> = self
            .modes
            .values()
            .map(|dm| {
                let mut tool_groups = dm.mode.base_tool_groups();
                tool_groups.extend(dm.extra_tools.clone());
                AgentMode {
                    slug: dm.mode.slug().to_string(),
                    name: dm.mode.display_name().to_string(),
                    description: dm.mode.description().to_string(),
                    instructions: None,
                    instructions_file: Some(format!("developer/{}.md", dm.mode.slug())),
                    tool_groups,
                    when_to_use: Some(dm.mode.when_to_use().to_string()),
                    is_internal: false,
                    deprecated: None,
                }
            })
            .collect();
        // Stable ordering
        result.sort_by_key(|m| {
            UniversalMode::all()
                .iter()
                .position(|um| um.slug() == m.slug)
                .unwrap_or(99)
        });
        result
    }

    pub fn recommended_extensions(&self, slug: &str) -> Vec<String> {
        self.modes
            .get(slug)
            .map(|dm| {
                dm.recommended_extensions
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn tool_groups_for(&self, slug: &str) -> Vec<crate::registry::manifest::ToolGroupAccess> {
        self.modes
            .get(slug)
            .map(|dm| {
                let mut tg = dm.mode.base_tool_groups();
                tg.extend(dm.extra_tools.clone());
                tg
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode_is_write() {
        let agent = DeveloperAgent::new();
        assert_eq!(agent.default_mode(), "write");
    }

    #[test]
    fn test_has_five_universal_modes() {
        let agent = DeveloperAgent::new();
        let modes = agent.modes();
        assert_eq!(modes.len(), 5);
        assert_eq!(modes, vec!["ask", "plan", "write", "review", "debug"]);
    }

    #[test]
    fn test_mode_lookup() {
        let agent = DeveloperAgent::new();
        assert!(agent.mode("write").is_some());
        assert!(agent.mode("ask").is_some());
        assert!(agent.mode("plan").is_some());
        assert!(agent.mode("review").is_some());
        assert!(agent.mode("debug").is_some());
        assert!(agent.mode("backend").is_none());
        assert!(agent.mode("pm").is_none());
    }

    #[test]
    fn test_to_agent_modes_ordered() {
        let agent = DeveloperAgent::new();
        let modes = agent.to_agent_modes();
        assert_eq!(modes.len(), 5);
        let slugs: Vec<&str> = modes.iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(slugs, vec!["ask", "plan", "write", "review", "debug"]);
    }

    #[test]
    fn test_all_modes_have_when_to_use() {
        let agent = DeveloperAgent::new();
        for mode in agent.to_agent_modes() {
            assert!(
                mode.when_to_use.is_some(),
                "Mode {} missing when_to_use",
                mode.slug
            );
        }
    }

    #[test]
    fn test_write_has_edit_and_command() {
        let agent = DeveloperAgent::new();
        let modes = agent.to_agent_modes();
        let write_mode = modes.iter().find(|m| m.slug == "write").unwrap();
        let tg_str = format!("{:?}", write_mode.tool_groups);
        assert!(tg_str.contains("edit"), "Write mode needs edit: {tg_str}");
        assert!(
            tg_str.contains("command"),
            "Write mode needs command: {tg_str}"
        );
    }

    #[test]
    fn test_ask_is_readonly() {
        let agent = DeveloperAgent::new();
        let modes = agent.to_agent_modes();
        let ask_mode = modes.iter().find(|m| m.slug == "ask").unwrap();
        let tg_str = format!("{:?}", ask_mode.tool_groups);
        assert!(
            !tg_str.contains("edit"),
            "Ask mode should not have edit: {tg_str}"
        );
    }

    #[test]
    fn test_review_is_readonly_but_can_run_checks() {
        let agent = DeveloperAgent::new();
        let modes = agent.to_agent_modes();
        let review_mode = modes.iter().find(|m| m.slug == "review").unwrap();
        let tg_str = format!("{:?}", review_mode.tool_groups);
        assert!(
            !tg_str.contains("edit"),
            "Review mode should not have edit: {tg_str}"
        );
        assert!(
            tg_str.contains("command"),
            "Review mode needs command for running checks: {tg_str}"
        );
    }

    #[test]
    fn test_recommended_extensions() {
        let agent = DeveloperAgent::new();
        let exts = agent.recommended_extensions("write");
        assert!(exts.contains(&"developer".to_string()));
        assert!(exts.contains(&"github".to_string()));
    }

    #[test]
    fn test_unknown_mode_returns_none() {
        let agent = DeveloperAgent::new();
        assert!(agent.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let agent = DeveloperAgent::new();
        let ctx = HashMap::new();
        let result = agent.render_mode("write", &ctx);
        assert!(result.is_ok(), "render_mode failed: {:?}", result.err());
    }
}
