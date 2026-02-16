//! QA Agent — a multi-mode agent for quality assurance, testing, and code review.
//!
//! Each mode represents a specialized QA activity. Modes can be switched dynamically
//! via ACP `set_session_mode` to adapt the agent's behavior to the current task.
//!
//! # Modes
//!
//! | Mode | Role | Tool Groups |
//! |------|------|-------------|
//! | `analyze` | QA Analyst — code quality, anti-patterns, complexity | developer, read |
//! | `test-design` | Test Designer — test strategies, plans, case generation | developer, read, memory |
//! | `coverage-audit` | Coverage Auditor — test gap analysis, reliability | developer, read, command |
//! | `review` | Code Reviewer — correctness, reliability, maintainability | developer, read |

use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};
use serde::Serialize;
use std::collections::HashMap;

/// A QA agent mode representing a specialized quality activity.
#[derive(Debug, Clone)]
pub struct QaMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub template_name: String,
    pub tool_groups: Vec<ToolGroupAccess>,
    pub when_to_use: String,
    pub recommended_extensions: Vec<String>,
}

/// The QA Agent with quality-specialized modes.
pub struct QaAgent {
    modes: HashMap<String, QaMode>,
    default_mode: String,
}

impl Default for QaAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl QaAgent {
    pub fn new() -> Self {
        let modes = vec![
            QaMode {
                slug: "analyze".into(),
                name: "🔍 QA Analyst".into(),
                description:
                    "Code quality analysis, anti-patterns, complexity, and actionable findings"
                        .into(),
                template_name: "qa_agent/analyze.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use:
                    "When analyzing code quality, finding anti-patterns, or reviewing complexity"
                        .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "knowledgegraph".into(),
                ],
            },
            QaMode {
                slug: "test-design".into(),
                name: "📝 Test Designer".into(),
                description:
                    "Test strategies, test plans, test case generation, and fixture design".into(),
                template_name: "qa_agent/test_design.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use:
                    "When designing test strategies, writing test plans, or generating test cases"
                        .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "knowledgegraph".into(),
                ],
            },
            QaMode {
                slug: "coverage-audit".into(),
                name: "📊 Coverage Auditor".into(),
                description:
                    "Test coverage gap analysis, audit of existing tests, reliability assessment"
                        .into(),
                template_name: "qa_agent/coverage_audit.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("command".into()),
                ],
                when_to_use:
                    "When auditing test coverage, finding coverage gaps, or assessing test quality"
                        .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "code_execution".into(),
                ],
            },
            QaMode {
                slug: "review".into(),
                name: "👁️ Code Reviewer".into(),
                description:
                    "Code review for correctness, reliability, concurrency safety, and maintainability"
                        .into(),
                template_name: "qa_agent/review.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("read".into()),
                ],
                when_to_use:
                    "When reviewing code for bugs, correctness, reliability, or maintainability"
                        .into(),
                recommended_extensions: vec!["developer".into()],
            },
        ];

        let default_mode = "analyze".to_string();
        let modes_map: HashMap<String, QaMode> =
            modes.into_iter().map(|m| (m.slug.clone(), m)).collect();

        Self {
            modes: modes_map,
            default_mode,
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&QaMode> {
        self.modes.get(slug)
    }

    pub fn modes(&self) -> Vec<&QaMode> {
        let order = ["analyze", "test-design", "coverage-audit", "review"];
        order
            .iter()
            .filter_map(|slug| self.modes.get(*slug))
            .collect()
    }

    pub fn default_mode_slug(&self) -> &str {
        &self.default_mode
    }

    pub fn render_mode<T: Serialize>(
        &self,
        slug: &str,
        context: &T,
    ) -> Result<String, minijinja::Error> {
        let mode = self.mode(slug).ok_or_else(|| {
            minijinja::Error::new(
                minijinja::ErrorKind::TemplateNotFound,
                format!("QA mode '{}' not found", slug),
            )
        })?;
        prompt_template::render_template(&mode.template_name, context)
    }

    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        self.modes()
            .iter()
            .map(|m| AgentMode {
                slug: m.slug.clone(),
                name: m.name.clone(),
                description: m.description.clone(),
                instructions: None,
                instructions_file: Some(m.template_name.clone()),
                tool_groups: m.tool_groups.clone(),
                when_to_use: Some(m.when_to_use.clone()),
                is_internal: false,
                deprecated: None,
            })
            .collect()
    }

    pub fn recommended_extensions(&self, slug: &str) -> Vec<String> {
        self.mode(slug)
            .map(|m| m.recommended_extensions.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_has_four_modes() {
        let qa = QaAgent::new();
        assert_eq!(qa.modes().len(), 4);
    }

    #[test]
    fn test_default_mode_is_analyze() {
        let qa = QaAgent::new();
        assert_eq!(qa.default_mode_slug(), "analyze");
    }

    #[test]
    fn test_mode_lookup() {
        let qa = QaAgent::new();
        let review = qa.mode("review").unwrap();
        assert_eq!(review.name, "👁️ Code Reviewer");
        assert!(review.when_to_use.contains("reviewing code"));
    }

    #[test]
    fn test_mode_order() {
        let qa = QaAgent::new();
        let slugs: Vec<&str> = qa.modes().iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec!["analyze", "test-design", "coverage-audit", "review"]
        );
    }

    #[test]
    fn test_to_agent_modes() {
        let qa = QaAgent::new();
        let modes = qa.to_agent_modes();
        assert_eq!(modes.len(), 4);
        assert!(modes.iter().all(|m| m.instructions_file.is_some()));
        assert!(modes.iter().all(|m| m.when_to_use.is_some()));
        assert!(modes.iter().all(|m| !m.is_internal));
    }

    #[test]
    fn test_tool_groups_per_mode() {
        let qa = QaAgent::new();

        // Analyze is read-only + memory
        let analyze = qa.mode("analyze").unwrap();
        assert!(analyze
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "developer")));
        assert!(analyze
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "memory")));

        // Coverage audit can run commands (for test runners)
        let coverage = qa.mode("coverage-audit").unwrap();
        assert!(coverage
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "command")));

        // Review is minimal — developer + read only
        let review = qa.mode("review").unwrap();
        assert!(!review
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "command")));
    }

    #[test]
    fn test_recommended_extensions() {
        let qa = QaAgent::new();
        let recs = qa.recommended_extensions("coverage-audit");
        assert!(recs.contains(&"developer".to_string()));
        assert!(recs.contains(&"code_execution".to_string()));
    }

    #[test]
    fn test_unknown_mode_returns_none() {
        let qa = QaAgent::new();
        assert!(qa.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let qa = QaAgent::new();
        let result = qa.render_mode("analyze", &HashMap::<String, String>::new());
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Quality Assurance"));
    }
}
