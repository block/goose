/// Universal behavioral modes for all agents.
///
/// Modes are orthogonal to agents (personas). Every agent supports the same
/// set of behavioral stances; the agent's *persona* determines domain
/// expertise while the *mode* determines how it behaves.
///
/// | Mode   | Stance           | Tools          | Output        |
/// |--------|------------------|----------------|---------------|
/// | ask    | Read-only Q&A    | read, fetch    | Answers       |
/// | plan   | Design / reason  | read, fetch    | Plans, ADRs   |
/// | write  | Produce artifacts| full access    | Code, configs |
/// | review | Evaluate work    | read           | Feedback      |
/// | debug  | Diagnose issues  | full access    | Root cause    |
use serde::Serialize;

use crate::registry::manifest::{AgentMode, ToolGroupAccess};

/// The five universal behavioral modes every agent can operate in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum UniversalMode {
    Ask,
    Plan,
    Write,
    Review,
    Debug,
}

impl UniversalMode {
    /// All modes in display order.
    pub fn all() -> &'static [UniversalMode] {
        &[
            UniversalMode::Ask,
            UniversalMode::Plan,
            UniversalMode::Write,
            UniversalMode::Review,
            UniversalMode::Debug,
        ]
    }

    pub fn slug(&self) -> &'static str {
        match self {
            UniversalMode::Ask => "ask",
            UniversalMode::Plan => "plan",
            UniversalMode::Write => "write",
            UniversalMode::Review => "review",
            UniversalMode::Debug => "debug",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            UniversalMode::Ask => "❓ Ask",
            UniversalMode::Plan => "📋 Plan",
            UniversalMode::Write => "✏️ Write",
            UniversalMode::Review => "🔍 Review",
            UniversalMode::Debug => "🐛 Debug",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            UniversalMode::Ask => {
                "Read-only exploration: search files, query docs, answer questions. \
                 No file modifications."
            }
            UniversalMode::Plan => {
                "Design and reason about approach: produce plans, architecture docs, \
                 ADRs, roadmaps. Read-only with optional markdown/temp output."
            }
            UniversalMode::Write => {
                "Produce artifacts: write code, configs, docs, tests. Full tool access \
                 with verification."
            }
            UniversalMode::Review => {
                "Evaluate existing work: code review, audit, assess quality. Read-only \
                 analysis with structured feedback."
            }
            UniversalMode::Debug => {
                "Diagnose and fix issues: reproduce, isolate, hypothesize, fix, verify. \
                 Full tool access focused on root-cause analysis."
            }
        }
    }

    pub fn when_to_use(&self) -> &'static str {
        match self {
            UniversalMode::Ask => {
                "questions answers explain what why how search find show describe \
                 understand explore lookup documentation help"
            }
            UniversalMode::Plan => {
                "plan design architect propose strategy roadmap approach outline \
                 structure organize decide evaluate options tradeoffs ADR RFC"
            }
            UniversalMode::Write => {
                "write code implement create build add fix update change develop \
                 configure deploy scaffold generate test setup install"
            }
            UniversalMode::Review => {
                "review audit check assess evaluate quality feedback analyze inspect \
                 critique examine verify validate compliance"
            }
            UniversalMode::Debug => {
                "debug diagnose troubleshoot fix error bug crash failure broken \
                 investigate reproduce isolate root cause stack trace log"
            }
        }
    }

    /// Base tool groups for this mode (agent may add domain-specific ones).
    pub fn base_tool_groups(&self) -> Vec<ToolGroupAccess> {
        match self {
            UniversalMode::Ask => vec![
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
            UniversalMode::Plan => vec![
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
            UniversalMode::Write => vec![
                ToolGroupAccess::Full("developer".into()),
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("edit".into()),
                ToolGroupAccess::Full("command".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
            UniversalMode::Review => vec![
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
            UniversalMode::Debug => vec![
                ToolGroupAccess::Full("developer".into()),
                ToolGroupAccess::Full("read".into()),
                ToolGroupAccess::Full("edit".into()),
                ToolGroupAccess::Full("command".into()),
                ToolGroupAccess::Full("fetch".into()),
                ToolGroupAccess::Full("memory".into()),
            ],
        }
    }

    /// Convert to an `AgentMode` for ACP/A2A discovery.
    ///
    /// `agent_slug` is used to namespace the template file path
    /// (e.g., "developer" → "developer/write.md").
    pub fn to_agent_mode(&self, agent_slug: &str) -> AgentMode {
        AgentMode {
            slug: self.slug().to_string(),
            name: self.display_name().to_string(),
            description: self.description().to_string(),
            instructions: None,
            instructions_file: Some(format!("{}/{}.md", agent_slug, self.slug())),
            tool_groups: self.base_tool_groups(),
            when_to_use: Some(self.when_to_use().to_string()),
            is_internal: false,
            deprecated: None,
        }
    }

    /// Parse a slug string into a UniversalMode.
    pub fn from_slug(slug: &str) -> Option<UniversalMode> {
        match slug {
            "ask" => Some(UniversalMode::Ask),
            "plan" => Some(UniversalMode::Plan),
            "write" => Some(UniversalMode::Write),
            "review" => Some(UniversalMode::Review),
            "debug" => Some(UniversalMode::Debug),
            _ => None,
        }
    }
}

impl std::fmt::Display for UniversalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.slug())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modes_ordered() {
        let modes = UniversalMode::all();
        assert_eq!(modes.len(), 5);
        assert_eq!(modes[0], UniversalMode::Ask);
        assert_eq!(modes[4], UniversalMode::Debug);
    }

    #[test]
    fn test_slug_roundtrip() {
        for mode in UniversalMode::all() {
            let slug = mode.slug();
            let parsed = UniversalMode::from_slug(slug).unwrap();
            assert_eq!(*mode, parsed);
        }
    }

    #[test]
    fn test_unknown_slug_returns_none() {
        assert!(UniversalMode::from_slug("nonexistent").is_none());
        assert!(UniversalMode::from_slug("backend").is_none());
    }

    #[test]
    fn test_to_agent_mode() {
        let mode = UniversalMode::Write.to_agent_mode("developer");
        assert_eq!(mode.slug, "write");
        assert_eq!(mode.instructions_file, Some("developer/write.md".into()));
        assert!(!mode.is_internal);
        assert!(!mode.tool_groups.is_empty());
    }

    #[test]
    fn test_ask_is_readonly() {
        let groups = UniversalMode::Ask.base_tool_groups();
        for tg in &groups {
            let name = format!("{:?}", tg);
            assert!(
                !name.contains("edit") && !name.contains("command"),
                "Ask mode should not have edit/command access: {name}"
            );
        }
    }

    #[test]
    fn test_review_is_readonly() {
        let groups = UniversalMode::Review.base_tool_groups();
        for tg in &groups {
            let name = format!("{:?}", tg);
            assert!(
                !name.contains("edit") && !name.contains("command"),
                "Review mode should not have edit/command access: {name}"
            );
        }
    }

    #[test]
    fn test_write_has_full_access() {
        let groups = UniversalMode::Write.base_tool_groups();
        let names: Vec<String> = groups.iter().map(|tg| format!("{:?}", tg)).collect();
        let joined = names.join(" ");
        assert!(joined.contains("edit"), "Write mode needs edit access");
        assert!(
            joined.contains("command"),
            "Write mode needs command access"
        );
    }

    #[test]
    fn test_debug_has_full_access() {
        let groups = UniversalMode::Debug.base_tool_groups();
        let names: Vec<String> = groups.iter().map(|tg| format!("{:?}", tg)).collect();
        let joined = names.join(" ");
        assert!(joined.contains("edit"), "Debug mode needs edit access");
        assert!(
            joined.contains("command"),
            "Debug mode needs command access"
        );
    }

    #[test]
    fn test_display_names_have_emoji() {
        for mode in UniversalMode::all() {
            let name = mode.display_name();
            assert!(
                name.len() > mode.slug().len(),
                "Display name should have emoji prefix: {name}"
            );
        }
    }
}
