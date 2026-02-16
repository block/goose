use std::collections::HashMap;

use serde::Serialize;

use crate::prompt_template::render_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

/// A mode definition for the Research Agent.
pub struct ResearchMode {
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    template_name: &'static str,
    tool_groups: Vec<ToolGroupAccess>,
    when_to_use: &'static str,
    recommended_extensions: Vec<&'static str>,
}

/// Research Agent — investigates topics, compares technologies, summarizes
/// documents, and guides learning through structured explanations.
///
/// Four specialized modes:
/// - **investigate**: Deep-dive research on technical topics
/// - **compare**: Structured technology/tool comparisons
/// - **summarize**: Document and discussion summarization
/// - **learn**: Concept explanations and learning paths
pub struct ResearchAgent {
    modes: HashMap<&'static str, ResearchMode>,
    default_mode: &'static str,
}

impl Default for ResearchAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl ResearchAgent {
    pub fn new() -> Self {
        let mut modes = HashMap::new();

        modes.insert(
            "investigate",
            ResearchMode {
                slug: "investigate",
                name: "🔬 Research Investigator",
                description: "Deep-dive into technical topics, libraries, APIs, and technologies",
                template_name: "research_agent/investigate.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "research investigate how does what is library framework \
                    API documentation explore unfamiliar technology tool",
                recommended_extensions: vec!["fetch", "memory"],
            },
        );

        modes.insert(
            "compare",
            ResearchMode {
                slug: "compare",
                name: "⚖️ Technology Comparator",
                description: "Structured comparison of tools, frameworks, and approaches",
                template_name: "research_agent/compare.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "compare versus vs alternative which is better pros cons \
                    trade-offs benchmark decision matrix evaluation",
                recommended_extensions: vec!["fetch", "memory"],
            },
        );

        modes.insert(
            "summarize",
            ResearchMode {
                slug: "summarize",
                name: "📝 Document Summarizer",
                description:
                    "Distill documents, discussions, and threads into structured summaries",
                template_name: "research_agent/summarize.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "summarize TLDR summary digest key points extract \
                    document RFC design doc discussion thread changelog",
                recommended_extensions: vec!["fetch", "memory"],
            },
        );

        modes.insert(
            "learn",
            ResearchMode {
                slug: "learn",
                name: "🎓 Learning Guide",
                description: "Explain concepts, create learning paths, and guide understanding",
                template_name: "research_agent/learn.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "explain teach learn tutorial how does why concept \
                    understand beginner guide walkthrough example",
                recommended_extensions: vec!["fetch", "memory"],
            },
        );

        Self {
            modes,
            default_mode: "investigate",
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&ResearchMode> {
        self.modes.get(slug)
    }

    pub fn modes(&self) -> Vec<&ResearchMode> {
        let order = ["investigate", "compare", "summarize", "learn"];
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
                format!("Research mode '{}' not found", slug),
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
        let agent = ResearchAgent::new();
        assert_eq!(agent.default_mode_slug(), "investigate");
    }

    #[test]
    fn test_mode_lookup() {
        let agent = ResearchAgent::new();
        let mode = agent.mode("compare").unwrap();
        assert_eq!(mode.name, "⚖️ Technology Comparator");
    }

    #[test]
    fn test_mode_order() {
        let agent = ResearchAgent::new();
        let slugs: Vec<&str> = agent.modes().iter().map(|m| m.slug).collect();
        assert_eq!(slugs, vec!["investigate", "compare", "summarize", "learn"]);
    }

    #[test]
    fn test_four_modes() {
        let agent = ResearchAgent::new();
        assert_eq!(agent.modes().len(), 4);
    }

    #[test]
    fn test_agent_modes_conversion() {
        let agent = ResearchAgent::new();
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
        let agent = ResearchAgent::new();
        let inv = agent.mode("investigate").unwrap();
        assert!(inv
            .tool_groups
            .iter()
            .any(|t| matches!(t, ToolGroupAccess::Full(g) if g == "fetch")));
    }

    #[test]
    fn test_summarize_has_read_access() {
        let agent = ResearchAgent::new();
        let sum = agent.mode("summarize").unwrap();
        assert!(sum
            .tool_groups
            .iter()
            .any(|t| matches!(t, ToolGroupAccess::Full(g) if g == "read")));
    }

    #[test]
    fn test_recommended_extensions() {
        let agent = ResearchAgent::new();
        let exts = agent.recommended_extensions("investigate");
        assert!(exts.contains(&"fetch".to_string()));
        assert!(exts.contains(&"memory".to_string()));
    }

    #[test]
    fn test_unknown_mode() {
        let agent = ResearchAgent::new();
        assert!(agent.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let agent = ResearchAgent::new();
        let ctx = std::collections::HashMap::<String, String>::new();
        let rendered = agent.render_mode("investigate", &ctx);
        assert!(rendered.is_ok());
        assert!(rendered.unwrap().contains("Research Investigator"));
    }
}
