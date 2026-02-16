//! Coding Agent — focused on writing, debugging, and deploying code.
//!
//! Each mode represents a behavioral stance (not a persona), with curated instructions
//! and recommended tool groups. Modes can be switched dynamically via ACP
//! `set_session_mode` to adapt the agent's behavior to the current task.
//!
//! # Modes
//!
//! | Mode | Slug | Focus | Key Tool Groups |
//! |------|------|-------|-----------------|
//! | 💻 Code | `code` | Implementation, APIs, logic | developer, edit, command |
//! | 🏗️ Architect | `architect` | System design, ADRs, diagrams | developer, read |
//! | 🎨 Frontend | `frontend` | UI, client-side, accessibility | developer, edit, browser |
//! | 🔍 Debug | `debug` | Diagnostics, profiling, fixes | developer, command |
//! | 🚀 DevOps | `devops` | CI/CD, IaC, containers, monitoring | developer, command |
//!
//! ## Tool Group Mapping
//!
//! Tool groups are abstract categories mapped to concrete MCP tools at runtime:
//!
//! | Group | Maps to |
//! |-------|---------|
//! | `developer` | developer__analyze, developer__shell, developer__text_editor |
//! | `edit` | developer__text_editor (write, str_replace, insert) |
//! | `command` | developer__shell |
//! | `read` | developer__text_editor (view), developer__analyze |
//! | `browser` | developer__screen_capture, developer__list_windows |
//! | `memory` | knowledgegraphmemory__* |
//! | `fetch` | fetch__fetch, context7__* |
//! | `mcp` | All MCP extension tools |

use std::collections::HashMap;

use crate::prompt_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

#[derive(Debug, Clone)]
pub struct CodingMode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub template_name: String,
    pub tool_groups: Vec<ToolGroupAccess>,
    pub when_to_use: String,
    pub recommended_extensions: Vec<String>,
}

#[derive(Debug)]
pub struct CodingAgent {
    modes: HashMap<String, CodingMode>,
    mode_order: Vec<String>,
    default_mode: String,
}

impl Default for CodingAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl CodingAgent {
    pub fn new() -> Self {
        let modes_vec = vec![
            CodingMode {
                slug: "code".into(),
                name: "💻 Code".into(),
                description: "Implementation: APIs, business logic, data models, and server-side code".into(),
                template_name: "coding_agent/code.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("mcp".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "When implementing backend code, APIs, database schemas, server logic, or writing new features".into(),
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
                slug: "architect".into(),
                name: "🏗️ Architect".into(),
                description: "System design, component boundaries, C4 diagrams, and ADRs".into(),
                template_name: "coding_agent/architect.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("fetch".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "When designing system architecture, creating diagrams, or writing ADRs".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "knowledgegraph".into(),
                    "fetch".into(),
                    "context7".into(),
                ],
            },
            CodingMode {
                slug: "frontend".into(),
                name: "🎨 Frontend".into(),
                description: "UI components, client-side logic, state management, and accessibility".into(),
                template_name: "coding_agent/frontend.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("browser".into()),
                    ToolGroupAccess::Full("mcp".into()),
                ],
                when_to_use: "When implementing UI components, styling, responsive design, or client-side features".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "computercontroller".into(),
                    "chrome_devtools".into(),
                    "context7".into(),
                ],
            },
            CodingMode {
                slug: "debug".into(),
                name: "🔍 Debug".into(),
                description: "Systematic diagnosis: reproduce, isolate, fix, and verify bugs".into(),
                template_name: "coding_agent/debug.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "When debugging errors, diagnosing failures, fixing bugs, or profiling performance issues".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "code_execution".into(),
                    "chrome_devtools".into(),
                ],
            },
            CodingMode {
                slug: "devops".into(),
                name: "🚀 DevOps".into(),
                description: "CI/CD pipelines, infrastructure as code, containers, and monitoring".into(),
                template_name: "coding_agent/devops.md".into(),
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("command".into()),
                    ToolGroupAccess::Full("edit".into()),
                    ToolGroupAccess::Full("read".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "When setting up CI/CD pipelines, configuring deployments, writing Dockerfiles, Kubernetes manifests, or monitoring".into(),
                recommended_extensions: vec![
                    "developer".into(),
                    "github".into(),
                    "fetch".into(),
                ],
            },
        ];

        let mode_order: Vec<String> = modes_vec.iter().map(|m| m.slug.clone()).collect();
        let default_mode = "code".into();
        let modes: HashMap<String, CodingMode> =
            modes_vec.into_iter().map(|m| (m.slug.clone(), m)).collect();

        Self {
            modes,
            mode_order,
            default_mode,
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&CodingMode> {
        self.modes.get(slug)
    }

    pub fn modes(&self) -> Vec<&CodingMode> {
        self.mode_order
            .iter()
            .filter_map(|slug| self.modes.get(slug))
            .collect()
    }

    pub fn default_mode_slug(&self) -> &str {
        &self.default_mode
    }

    pub fn render_mode(
        &self,
        slug: &str,
        extra_context: &HashMap<String, String>,
    ) -> anyhow::Result<String> {
        let mode = self
            .mode(slug)
            .ok_or_else(|| anyhow::anyhow!("Unknown coding mode: {}", slug))?;
        let mut context = extra_context.clone();
        context.insert("mode_name".into(), mode.name.clone());
        context.insert("mode_description".into(), mode.description.clone());
        Ok(prompt_template::render_template(
            &mode.template_name,
            &context,
        )?)
    }

    pub fn to_agent_modes(&self) -> Vec<AgentMode> {
        self.modes()
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

    #[test]
    fn test_default_mode_is_code() {
        let ca = CodingAgent::new();
        assert_eq!(ca.default_mode_slug(), "code");
    }

    #[test]
    fn test_mode_lookup() {
        let ca = CodingAgent::new();
        let mode = ca.mode("architect").unwrap();
        assert_eq!(mode.slug, "architect");
        assert!(mode.name.contains("Architect"));
    }

    #[test]
    fn test_mode_count_and_order() {
        let ca = CodingAgent::new();
        let modes = ca.modes();
        assert_eq!(modes.len(), 5);
        let slugs: Vec<&str> = modes.iter().map(|m| m.slug.as_str()).collect();
        assert_eq!(
            slugs,
            vec!["code", "architect", "frontend", "debug", "devops"]
        );
    }

    #[test]
    fn test_to_agent_modes() {
        let ca = CodingAgent::new();
        let agent_modes = ca.to_agent_modes();
        assert_eq!(agent_modes.len(), 5);
        assert!(agent_modes.iter().all(|m| m.when_to_use.is_some()));
    }

    #[test]
    fn test_tool_groups_per_mode() {
        let ca = CodingAgent::new();

        // Code mode has full developer + edit + command access
        let code = ca.mode("code").unwrap();
        let code_groups: Vec<String> = code
            .tool_groups
            .iter()
            .map(|tg| format!("{:?}", tg))
            .collect();
        assert!(code_groups.iter().any(|g| g.contains("developer")));
        assert!(code_groups.iter().any(|g| g.contains("edit")));
        assert!(code_groups.iter().any(|g| g.contains("command")));

        // Architect is read-heavy
        let arch = ca.mode("architect").unwrap();
        let arch_groups: Vec<String> = arch
            .tool_groups
            .iter()
            .map(|tg| format!("{:?}", tg))
            .collect();
        assert!(arch_groups.iter().any(|g| g.contains("read")));
    }

    #[test]
    fn test_recommended_extensions() {
        let ca = CodingAgent::new();
        let exts = ca.recommended_extensions("code");
        assert!(exts.contains(&"developer".to_string()));
        assert!(exts.contains(&"github".to_string()));
    }

    #[test]
    fn test_unknown_mode() {
        let ca = CodingAgent::new();
        assert!(ca.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let ca = CodingAgent::new();
        let result = ca.render_mode("code", &HashMap::<String, String>::new());
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Code"));
    }
}
