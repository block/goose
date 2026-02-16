//! PM Agent — product management persona with universal behavioral modes.
//!
//! The PM Agent is a specialist in product strategy, requirements, and roadmaps.
//! It uses the universal mode set (ask/plan/write/review) to adapt its behavior.
//! The persona stays constant; the mode changes HOW it works.

use std::collections::HashMap;

use serde::Serialize;

use crate::agents::universal_mode::UniversalMode;
use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

const PM_EXTRA_TOOLS: &[&str] = &["memory"];
const PM_EXTENSIONS: &[&str] = &["memory", "fetch"];

#[derive(Debug, Clone, Serialize)]
pub struct PmAgent {
    modes: HashMap<String, UniversalMode>,
    default_mode: String,
}

impl Default for PmAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl PmAgent {
    pub fn new() -> Self {
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
            let template_name = format!("pm/{slug}.md");
            Ok(prompt_template::render_template(&template_name, context)?)
        } else {
            anyhow::bail!("Unknown PM mode: {slug}")
        }
    }

    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        self.modes()
            .iter()
            .map(|m| {
                let mut tool_groups = m.base_tool_groups();
                for tool in PM_EXTRA_TOOLS {
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
                    instructions_file: Some(format!("pm/{}.md", m.slug())),
                    tool_groups,
                    when_to_use: Some(m.when_to_use().to_string()),
                    is_internal: false,
                    deprecated: None,
                }
            })
            .collect()
    }

    pub fn recommended_extensions(&self, _slug: &str) -> Vec<String> {
        PM_EXTENSIONS.iter().map(|s| s.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        let pm = PmAgent::new();
        assert_eq!(pm.default_mode(), "ask");
    }

    #[test]
    fn test_mode_lookup() {
        let pm = PmAgent::new();
        assert!(pm.mode("ask").is_some());
        assert!(pm.mode("plan").is_some());
        assert!(pm.mode("write").is_some());
        assert!(pm.mode("review").is_some());
        assert!(pm.mode("requirements").is_none()); // old slug
    }

    #[test]
    fn test_mode_count_and_order() {
        let pm = PmAgent::new();
        let modes = pm.modes();
        assert_eq!(modes.len(), 4);
        let slugs: Vec<&str> = modes.iter().map(|m| m.slug()).collect();
        assert_eq!(slugs, vec!["ask", "plan", "write", "review"]);
    }

    #[test]
    fn test_to_agent_modes() {
        let pm = PmAgent::new();
        let agent_modes = pm.to_agent_modes();
        assert_eq!(agent_modes.len(), 4);
        for am in &agent_modes {
            assert!(!am.name.is_empty());
            assert!(am.when_to_use.is_some());
        }
    }

    #[test]
    fn test_recommended_extensions() {
        let pm = PmAgent::new();
        let exts = pm.recommended_extensions("ask");
        assert!(exts.contains(&"memory".to_string()));
        assert!(exts.contains(&"fetch".to_string()));
    }

    #[test]
    fn test_unknown_mode_returns_none() {
        let pm = PmAgent::new();
        assert!(pm.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let pm = PmAgent::new();
        let ctx = HashMap::new();
        let result = pm.render_mode("ask", &ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("PM Agent"));
    }
}
