//! QA Agent — quality assurance persona with universal behavioral modes.
//!
//! The QA Agent is a specialist in testing, code review, and quality analysis.
//! It uses the universal mode set (ask/plan/write/review) to adapt its behavior
//! to the current task. The persona stays constant; the mode changes HOW it works.

use std::collections::HashMap;

use serde::Serialize;

use crate::agents::universal_mode::UniversalMode;
use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

/// Extra tools the QA persona adds on top of universal mode base tools.
const QA_EXTRA_TOOLS: &[&str] = &["memory"];

/// Recommended MCP extensions for the QA Agent.
const QA_EXTENSIONS: &[&str] = &["developer", "memory"];

#[derive(Debug, Clone, Serialize)]
pub struct QaAgent {
    modes: HashMap<String, UniversalMode>,
    default_mode: String,
}

impl Default for QaAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl QaAgent {
    pub fn new() -> Self {
        // QA uses 4 universal modes (no debug — that's for developers)
        let mode_list = vec![
            UniversalMode::Ask,
            UniversalMode::Plan,
            UniversalMode::Write,
            UniversalMode::Review,
        ];

        let modes = mode_list
            .into_iter()
            .map(|m| (m.slug().to_string(), m))
            .collect();

        Self {
            modes,
            default_mode: "ask".into(),
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&UniversalMode> {
        self.modes.get(slug)
    }

    pub fn default_mode(&self) -> &str {
        &self.default_mode
    }

    pub fn modes(&self) -> Vec<&UniversalMode> {
        let order = ["ask", "plan", "write", "review"];
        order.iter().filter_map(|s| self.modes.get(*s)).collect()
    }

    pub fn render_mode(
        &self,
        slug: &str,
        context: &HashMap<String, String>,
    ) -> anyhow::Result<String> {
        if self.modes.contains_key(slug) {
            let template_name = format!("qa/{slug}.md");
            Ok(prompt_template::render_template(&template_name, context)?)
        } else {
            anyhow::bail!("Unknown QA mode: {slug}")
        }
    }

    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        self.modes()
            .iter()
            .map(|m| {
                let mut tool_groups = m.base_tool_groups();
                // Add QA-specific tools
                for tool in QA_EXTRA_TOOLS {
                    let tg = ToolGroupAccess::Full(tool.to_string());
                    if !tool_groups.iter().any(|t| format!("{t:?}").contains(tool)) {
                        tool_groups.push(tg);
                    }
                }

                AgentMode {
                    slug: m.slug().to_string(),
                    name: m.display_name().to_string(),
                    description: m.description().to_string(),
                    instructions: None,
                    instructions_file: Some(format!("qa/{}.md", m.slug())),
                    tool_groups,
                    when_to_use: Some(m.when_to_use().to_string()),
                    is_internal: false,
                    deprecated: None,
                }
            })
            .collect()
    }

    pub fn recommended_extensions(&self, _slug: &str) -> Vec<String> {
        QA_EXTENSIONS.iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        let qa = QaAgent::new();
        assert_eq!(qa.default_mode(), "ask");
    }

    #[test]
    fn test_mode_lookup() {
        let qa = QaAgent::new();
        assert!(qa.mode("ask").is_some());
        assert!(qa.mode("plan").is_some());
        assert!(qa.mode("write").is_some());
        assert!(qa.mode("review").is_some());
        assert!(qa.mode("debug").is_none()); // QA has no debug mode
        assert!(qa.mode("analyze").is_none()); // old slug
    }

    #[test]
    fn test_mode_count_and_order() {
        let qa = QaAgent::new();
        let modes = qa.modes();
        assert_eq!(modes.len(), 4);
        let slugs: Vec<&str> = modes.iter().map(|m| m.slug()).collect();
        assert_eq!(slugs, vec!["ask", "plan", "write", "review"]);
    }

    #[test]
    fn test_to_agent_modes() {
        let qa = QaAgent::new();
        let agent_modes = qa.to_agent_modes();
        assert_eq!(agent_modes.len(), 4);
        for am in &agent_modes {
            assert!(!am.name.is_empty());
            assert!(!am.description.is_empty());
            assert!(am.when_to_use.is_some());
            assert!(!am.is_internal);
        }
    }

    #[test]
    fn test_tool_groups_include_memory() {
        let qa = QaAgent::new();
        let agent_modes = qa.to_agent_modes();
        for am in &agent_modes {
            let tool_str = format!("{:?}", am.tool_groups);
            assert!(
                tool_str.contains("memory"),
                "Mode {} should have memory tool",
                am.slug
            );
        }
    }

    #[test]
    fn test_recommended_extensions() {
        let qa = QaAgent::new();
        let exts = qa.recommended_extensions("ask");
        assert!(exts.contains(&"developer".to_string()));
        assert!(exts.contains(&"memory".to_string()));
    }

    #[test]
    fn test_unknown_mode_returns_none() {
        let qa = QaAgent::new();
        assert!(qa.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let qa = QaAgent::new();
        let ctx = HashMap::new();
        let result = qa.render_mode("ask", &ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("QA Agent"));
    }
}
