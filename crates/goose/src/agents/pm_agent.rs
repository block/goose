use std::collections::HashMap;

use serde::Serialize;

use crate::prompt_template::render_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

/// A mode definition for the PM Agent.
pub struct PmMode {
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    template_name: &'static str,
    tool_groups: Vec<ToolGroupAccess>,
    when_to_use: &'static str,
    recommended_extensions: Vec<&'static str>,
}

/// Product Management Agent — translates business needs into structured
/// requirements, roadmaps, priorities, and stakeholder analysis.
///
/// Four specialized modes:
/// - **requirements**: User stories, acceptance criteria, PRDs
/// - **prioritize**: RICE/MoSCoW scoring and backlog ordering
/// - **roadmap**: Milestone planning, phased rollout, risk registers
/// - **stakeholder**: Personas, competitive analysis, KPIs
pub struct PmAgent {
    modes: HashMap<&'static str, PmMode>,
    default_mode: &'static str,
}

impl Default for PmAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl PmAgent {
    pub fn new() -> Self {
        let mut modes = HashMap::new();

        modes.insert(
            "requirements",
            PmMode {
                slug: "requirements",
                name: "📋 Requirements Analyst",
                description: "Translate business needs into user stories and acceptance criteria",
                template_name: "pm_agent/requirements.md",
                tool_groups: vec![ToolGroupAccess::Full("memory".into())],
                when_to_use: "user stories requirements PRD acceptance criteria \
                    specifications functional non-functional edge cases",
                recommended_extensions: vec!["memory"],
            },
        );

        modes.insert(
            "prioritize",
            PmMode {
                slug: "prioritize",
                name: "🎯 Prioritization Strategist",
                description: "Apply RICE, MoSCoW, and other frameworks to order work",
                template_name: "pm_agent/prioritize.md",
                tool_groups: vec![ToolGroupAccess::Full("memory".into())],
                when_to_use: "prioritize backlog RICE MoSCoW scoring ranking \
                    trade-offs quick wins tech debt priority",
                recommended_extensions: vec!["memory"],
            },
        );

        modes.insert(
            "roadmap",
            PmMode {
                slug: "roadmap",
                name: "🗺️ Roadmap Planner",
                description: "Create milestone plans, phased rollouts, and risk registers",
                template_name: "pm_agent/roadmap.md",
                tool_groups: vec![ToolGroupAccess::Full("memory".into())],
                when_to_use: "roadmap milestones timeline release plan rollout \
                    phases alpha beta GA schedule capacity",
                recommended_extensions: vec!["memory"],
            },
        );

        modes.insert(
            "stakeholder",
            PmMode {
                slug: "stakeholder",
                name: "👥 Stakeholder Analyst",
                description: "Analyze personas, competitive landscape, and success metrics",
                template_name: "pm_agent/stakeholder.md",
                tool_groups: vec![ToolGroupAccess::Full("memory".into())],
                when_to_use: "persona stakeholder competitive analysis KPI metrics \
                    market positioning user research feedback",
                recommended_extensions: vec!["memory"],
            },
        );

        Self {
            modes,
            default_mode: "requirements",
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&PmMode> {
        self.modes.get(slug)
    }

    pub fn modes(&self) -> Vec<&PmMode> {
        let order = ["requirements", "prioritize", "roadmap", "stakeholder"];
        order.iter().filter_map(|s| self.modes.get(s)).collect()
    }

    pub fn default_mode_slug(&self) -> &str {
        self.default_mode
    }

    pub fn render_mode<T: Serialize>(
        &self,
        slug: &str,
        context: &T,
    ) -> Result<String, minijinja::Error> {
        let mode = self.mode(slug).ok_or_else(|| {
            minijinja::Error::new(
                minijinja::ErrorKind::TemplateNotFound,
                format!("PM mode '{}' not found", slug),
            )
        })?;
        render_template(mode.template_name, context)
    }

    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        self.modes()
            .iter()
            .map(|m| AgentMode {
                slug: m.slug.to_string(),
                name: m.name.to_string(),
                description: m.description.to_string(),
                instructions: None,
                instructions_file: Some(m.template_name.to_string()),
                tool_groups: m.tool_groups.clone(),
                when_to_use: Some(m.when_to_use.to_string()),
                is_internal: false,
            })
            .collect()
    }

    pub fn recommended_extensions(&self, slug: &str) -> Vec<String> {
        self.mode(slug)
            .map(|m| {
                m.recommended_extensions
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mode() {
        let agent = PmAgent::new();
        assert_eq!(agent.default_mode_slug(), "requirements");
    }

    #[test]
    fn test_mode_lookup() {
        let agent = PmAgent::new();
        let mode = agent.mode("prioritize").unwrap();
        assert_eq!(mode.name, "🎯 Prioritization Strategist");
    }

    #[test]
    fn test_mode_order() {
        let agent = PmAgent::new();
        let slugs: Vec<&str> = agent.modes().iter().map(|m| m.slug).collect();
        assert_eq!(
            slugs,
            vec!["requirements", "prioritize", "roadmap", "stakeholder"]
        );
    }

    #[test]
    fn test_four_modes() {
        let agent = PmAgent::new();
        assert_eq!(agent.modes().len(), 4);
    }

    #[test]
    fn test_agent_modes_conversion() {
        let agent = PmAgent::new();
        let modes = agent.to_agent_modes();
        assert_eq!(modes.len(), 4);
        for mode in &modes {
            assert!(mode.instructions_file.is_some());
            assert!(mode.when_to_use.is_some());
            assert!(!mode.is_internal);
        }
    }

    #[test]
    fn test_tool_groups() {
        let agent = PmAgent::new();
        let reqs = agent.mode("requirements").unwrap();
        assert!(reqs
            .tool_groups
            .iter()
            .any(|t| matches!(t, ToolGroupAccess::Full(g) if g == "memory")));
    }

    #[test]
    fn test_recommended_extensions() {
        let agent = PmAgent::new();
        let exts = agent.recommended_extensions("requirements");
        assert!(exts.contains(&"memory".to_string()));
    }

    #[test]
    fn test_unknown_mode() {
        let agent = PmAgent::new();
        assert!(agent.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let agent = PmAgent::new();
        let ctx = std::collections::HashMap::<String, String>::new();
        let rendered = agent.render_mode("requirements", &ctx);
        assert!(rendered.is_ok());
        assert!(rendered.unwrap().contains("Requirements Analyst"));
    }
}
