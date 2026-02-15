//! Coding Assistant — a multi-mode agent for the full software development lifecycle.
//!
//! Each mode represents a specialized role in the SDLC, with curated instructions
//! and recommended tool groups. Modes can be switched dynamically via ACP
//! `set_session_mode` to adapt the agent's behavior to the current task.
//!
//! # Modes
//!
//! | Mode | Role | Tool Groups |
//! |------|------|-------------|
//! | `pm` | Product Manager — requirements, user stories, prioritization | memory, fetch |
//! | `architect` | Software Architect — C4 diagrams, ADRs, API contracts | developer, memory, fetch |
//! | `backend` | Backend Engineer — APIs, data models, business logic | developer, command, mcp, memory |
//! | `frontend` | Frontend Engineer — UI components, state, accessibility | developer, command, browser, mcp |
//! | `qa` | Quality Assurance — test plans, automated testing, bug reports | developer, command, browser |
//! | `security` | Security Champion — OWASP, threat modeling, code review | developer, fetch, memory |
//! | `sre` | Site Reliability Engineer — SLOs, monitoring, incident response | developer, command, fetch |
//! | `devsecops` | DevSecOps — CI/CD security, IaC, container security | developer, command, mcp |
//!
//! # Tool Groups
//!
//! Tool groups are abstract capability categories. Actual tool availability depends
//! on which MCP extensions the user has configured:
//!
//! | Group | Maps to |
//! |-------|---------|
//! | `developer` | builtin developer extension (text_editor, shell) |
//! | `command` | shell execution, terminal management |
//! | `read` | file reading (subset of developer) |
//! | `edit` | file writing (subset of developer) |
//! | `mcp` | all user-configured MCP extensions (github, context7, etc.) |
//! | `browser` | chrome dev tools, computer controller |
//! | `memory` | knowledge graph, beads (project tracking) |
//! | `fetch` | web fetching for research |
//! | `code_execution` | Code Mode — batch tool calls into single scripts, save tokens |

use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};
use serde::Serialize;
use std::collections::HashMap;

/// A coding assistant mode representing a specialized SDLC role.
#[derive(Debug, Clone)]
pub struct CodingMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub template_name: String,
    pub tool_groups: Vec<ToolGroupAccess>,
    pub when_to_use: String,
    /// Recommended MCP extensions for this mode (informational).
    pub recommended_extensions: Vec<String>,
}

/// The Coding Assistant agent with SDLC-specialized modes.
pub struct CodingAgent {
    modes: HashMap<String, CodingMode>,
    default_mode: String,
}

impl Default for CodingAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl CodingAgent {
    pub fn new() -> Self {
        let modes = vec![
            CodingMode {
                slug: "pm".into(),
                name: "📋 Product Manager".into(),
                description: "Requirements, user stories, prioritization, and roadmap planning"
                    .into(),
                template_name: "coding_agent/pm.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("memory".into()),
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("read".into()),
                ],
                when_to_use:
                    "When defining requirements, writing user stories, or prioritizing features"
                        .into(),
                recommended_extensions: vec![
                    "beads".into(),
                    "knowledgegraph".into(),
                    "fetch".into(),
                ],
            },
            CodingMode {
                slug: "architect".into(),
                name: "📐 Architect".into(),
                description:
                    "System design, C4 diagrams, ADRs, API contracts, and technology decisions"
                        .into(),
                template_name: "coding_agent/architect.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("memory".into()),
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("read".into()),
                ],
                when_to_use:
                    "When designing system architecture, creating diagrams, or writing ADRs".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "knowledgegraph".into(),
                    "fetch".into(),
                    "context7".into(),
                ],
            },
            CodingMode {
                slug: "backend".into(),
                name: "⚙️ Backend Engineer".into(),
                description: "Server-side implementation, APIs, data models, and business logic"
                    .into(),
                template_name: "coding_agent/backend.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("mcp".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use:
                    "When implementing backend code, APIs, database schemas, or server logic"
                        .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "github".into(),
                    "context7".into(),
                    "beads".into(),
                    "knowledgegraph".into(),
                    "code_execution".into(),
                ],
            },
            CodingMode {
                slug: "frontend".into(),
                name: "🎨 Frontend Engineer".into(),
                description:
                    "UI components, client-side logic, state management, and accessibility".into(),
                template_name: "coding_agent/frontend.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("browser".into()),
                    ToolGroupAccess::Full("mcp".into()),
                ],
                when_to_use: "When implementing UI components, styling, or client-side features"
                    .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "computercontroller".into(),
                    "chrome_devtools".into(),
                    "context7".into(),
                ],
            },
            CodingMode {
                slug: "qa".into(),
                name: "🧪 Quality Assurance".into(),
                description:
                    "Test planning, automated testing, exploratory testing, and bug reporting"
                        .into(),
                template_name: "coding_agent/qa.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("browser".into()),
                    ToolGroupAccess::Full("read".into()),
                ],
                when_to_use: "When writing tests, creating test plans, or investigating bugs"
                    .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "computercontroller".into(),
                    "code_execution".into(),
                ],
            },
            CodingMode {
                slug: "security".into(),
                name: "🛡️ Security Champion".into(),
                description:
                    "Security code review, threat modeling, OWASP analysis, and vulnerability assessment"
                        .into(),
                template_name: "coding_agent/security.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "When reviewing code for security, performing threat modeling, or auditing dependencies".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "fetch".into(),
                    "knowledgegraph".into(),
                    "github".into(),
                ],
            },
            CodingMode {
                slug: "sre".into(),
                name: "🔧 SRE".into(),
                description:
                    "Reliability engineering, SLOs, monitoring, incident response, and observability"
                        .into(),
                template_name: "coding_agent/sre.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("read".into()),
                ],
                when_to_use: "When defining SLOs, setting up monitoring, or handling incidents"
                    .into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "fetch".into(),
                    "beads".into(),
                ],
            },
            CodingMode {
                slug: "devsecops".into(),
                name: "🔒 DevSecOps".into(),
                description:
                    "CI/CD security, infrastructure as code, container security, and supply chain"
                        .into(),
                template_name: "coding_agent/devsecops.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("mcp".into()),
                ],
                when_to_use: "When setting up CI/CD pipelines, hardening infrastructure, or implementing security automation".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "github".into(),
                    "code_execution".into(),
                ],
            },
        ];

        let default_mode = "backend".to_string();
        let modes_map: HashMap<String, CodingMode> =
            modes.into_iter().map(|m| (m.slug.clone(), m)).collect();

        Self {
            modes: modes_map,
            default_mode,
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&CodingMode> {
        self.modes.get(slug)
    }

    pub fn modes(&self) -> Vec<&CodingMode> {
        // Return in a logical SDLC order
        let order = [
            "pm",
            "architect",
            "backend",
            "frontend",
            "qa",
            "security",
            "sre",
            "devsecops",
        ];
        order
            .iter()
            .filter_map(|slug| self.modes.get(*slug))
            .collect()
    }

    pub fn default_mode_slug(&self) -> &str {
        &self.default_mode
    }

    /// Render the mode's prompt template with the given context.
    pub fn render_mode<C: Serialize>(&self, slug: &str, context: &C) -> anyhow::Result<String> {
        let mode = self
            .modes
            .get(slug)
            .ok_or_else(|| anyhow::anyhow!("Unknown coding assistant mode: {}", slug))?;
        Ok(prompt_template::render_template(
            &mode.template_name,
            context,
        )?)
    }

    /// Convert all modes to ACP-compatible `AgentMode` for protocol advertisement.
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
            })
            .collect()
    }

    /// Get recommended extensions for a mode.
    pub fn recommended_extensions(&self, slug: &str) -> Vec<String> {
        self.modes
            .get(slug)
            .map(|m| m.recommended_extensions.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_modes_present() {
        let ca = CodingAgent::new();
        assert_eq!(ca.modes().len(), 8);
    }

    #[test]
    fn test_default_mode_is_backend() {
        let ca = CodingAgent::new();
        assert_eq!(ca.default_mode_slug(), "backend");
    }

    #[test]
    fn test_mode_lookup() {
        let ca = CodingAgent::new();
        let pm = ca.mode("pm").unwrap();
        assert_eq!(pm.name, "📋 Product Manager");
        assert!(pm.when_to_use.contains("requirements"));
    }

    #[test]
    fn test_sdlc_order() {
        let ca = CodingAgent::new();
        let slugs: Vec<&str> = ca.modes().iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec![
                "pm",
                "architect",
                "backend",
                "frontend",
                "qa",
                "security",
                "sre",
                "devsecops"
            ]
        );
    }

    #[test]
    fn test_to_agent_modes() {
        let ca = CodingAgent::new();
        let modes = ca.to_agent_modes();
        assert_eq!(modes.len(), 8);
        assert!(modes.iter().all(|m| m.instructions_file.is_some()));
        assert!(modes.iter().all(|m| m.when_to_use.is_some()));
    }

    #[test]
    fn test_tool_groups_per_mode() {
        let ca = CodingAgent::new();

        // PM is read-only + memory + fetch
        let pm = ca.mode("pm").unwrap();
        assert!(pm
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "memory")));

        // Backend has developer + edit + command + mcp
        let backend = ca.mode("backend").unwrap();
        assert!(backend
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "developer")));
        assert!(backend
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "command")));

        // Security is read-only (no edit/command)
        let security = ca.mode("security").unwrap();
        assert!(!security
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "edit")));
        assert!(!security
            .tool_groups
            .iter()
            .any(|tg| matches!(tg, ToolGroupAccess::Full(g) if g == "command")));
    }

    #[test]
    fn test_recommended_extensions() {
        let ca = CodingAgent::new();
        let recs = ca.recommended_extensions("backend");
        assert!(recs.contains(&"developer".to_string()));
        assert!(recs.contains(&"github".to_string()));
    }

    #[test]
    fn test_unknown_mode_returns_none() {
        let ca = CodingAgent::new();
        assert!(ca.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let ca = CodingAgent::new();
        let result = ca.render_mode("pm", &HashMap::<String, String>::new());
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Product Manager"));
    }
}
