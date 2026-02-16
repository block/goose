use std::collections::HashMap;

use serde::Serialize;

use crate::prompt_template::render_template;
use crate::registry::manifest::{AgentMode, ToolGroupAccess};

/// A mode definition for the Security Agent.
pub struct SecurityMode {
    slug: &'static str,
    name: &'static str,
    description: &'static str,
    template_name: &'static str,
    tool_groups: Vec<ToolGroupAccess>,
    when_to_use: &'static str,
    recommended_extensions: Vec<&'static str>,
}

/// Security Agent — identifies vulnerabilities, models threats, audits
/// compliance, and plans security testing.
///
/// Four specialized modes:
/// - **threat-model**: STRIDE analysis, attack surface mapping, risk assessment
/// - **vulnerability**: SAST-style code review, injection analysis, CVE scanning
/// - **compliance**: OWASP ASVS, PCI-DSS, SOC 2, HIPAA/GDPR auditing
/// - **pentest**: Security test plan design, attack scenario creation
pub struct SecurityAgent {
    modes: HashMap<&'static str, SecurityMode>,
    default_mode: &'static str,
}

impl Default for SecurityAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityAgent {
    pub fn new() -> Self {
        let mut modes = HashMap::new();

        modes.insert(
            "threat-model",
            SecurityMode {
                slug: "threat-model",
                name: "🛡️ Threat Modeler",
                description: "STRIDE analysis, attack surface mapping, and risk assessment",
                template_name: "security_agent/threat_model.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "threat model STRIDE DREAD attack surface trust boundary \
                    risk assessment threat actor data flow security architecture",
                recommended_extensions: vec!["developer", "memory"],
            },
        );

        modes.insert(
            "vulnerability",
            SecurityMode {
                slug: "vulnerability",
                name: "🔍 Vulnerability Analyst",
                description: "SAST-style code review, injection analysis, and CVE scanning",
                template_name: "security_agent/vulnerability.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "vulnerability SAST injection XSS SQL OWASP CWE CVE \
                    security review code audit taint analysis secrets hardcoded",
                recommended_extensions: vec!["developer", "memory"],
            },
        );

        modes.insert(
            "compliance",
            SecurityMode {
                slug: "compliance",
                name: "📜 Compliance Auditor",
                description: "Audit against OWASP ASVS, PCI-DSS, SOC 2, HIPAA, GDPR",
                template_name: "security_agent/compliance.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "compliance audit PCI HIPAA GDPR SOC ASVS regulation \
                    standard certification control framework policy",
                recommended_extensions: vec!["developer", "memory"],
            },
        );

        modes.insert(
            "pentest",
            SecurityMode {
                slug: "pentest",
                name: "⚔️ Penetration Test Planner",
                description: "Design security test plans and attack scenarios",
                template_name: "security_agent/pentest.md",
                tool_groups: vec![
                    ToolGroupAccess::Full("developer".into()),
                    ToolGroupAccess::Full("memory".into()),
                ],
                when_to_use: "penetration test pentest security testing attack scenario \
                    fuzzing exploit proof of concept red team assessment",
                recommended_extensions: vec!["developer", "memory"],
            },
        );

        Self {
            modes,
            default_mode: "vulnerability",
        }
    }

    pub fn mode(&self, slug: &str) -> Option<&SecurityMode> {
        self.modes.get(slug)
    }

    pub fn modes(&self) -> Vec<&SecurityMode> {
        let order = ["threat-model", "vulnerability", "compliance", "pentest"];
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
                format!("Security mode '{}' not found", slug),
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
                deprecated: None,
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
        let agent = SecurityAgent::new();
        assert_eq!(agent.default_mode_slug(), "vulnerability");
    }

    #[test]
    fn test_mode_lookup() {
        let agent = SecurityAgent::new();
        let mode = agent.mode("threat-model").unwrap();
        assert_eq!(mode.name, "🛡️ Threat Modeler");
    }

    #[test]
    fn test_mode_order() {
        let agent = SecurityAgent::new();
        let slugs: Vec<&str> = agent.modes().iter().map(|m| m.slug).collect();
        assert_eq!(
            slugs,
            vec!["threat-model", "vulnerability", "compliance", "pentest"]
        );
    }

    #[test]
    fn test_four_modes() {
        let agent = SecurityAgent::new();
        assert_eq!(agent.modes().len(), 4);
    }

    #[test]
    fn test_agent_modes_conversion() {
        let agent = SecurityAgent::new();
        let modes = agent.to_agent_modes();
        assert_eq!(modes.len(), 4);
        for mode in &modes {
            assert!(mode.instructions_file.is_some());
            assert!(mode.when_to_use.is_some());
            assert!(!mode.is_internal);
        }
    }

    #[test]
    fn test_tool_groups_read_only() {
        let agent = SecurityAgent::new();
        let vuln = agent.mode("vulnerability").unwrap();
        assert!(!vuln.tool_groups.is_empty());
    }

    #[test]
    fn test_recommended_extensions() {
        let agent = SecurityAgent::new();
        let exts = agent.recommended_extensions("vulnerability");
        assert!(exts.contains(&"developer".to_string()));
        assert!(exts.contains(&"memory".to_string()));
    }

    #[test]
    fn test_unknown_mode() {
        let agent = SecurityAgent::new();
        assert!(agent.mode("nonexistent").is_none());
    }

    #[test]
    fn test_render_mode() {
        let agent = SecurityAgent::new();
        let ctx = std::collections::HashMap::<String, String>::new();
        let rendered = agent.render_mode("vulnerability", &ctx);
        assert!(rendered.is_ok());
        assert!(rendered.unwrap().contains("Vulnerability Analyst"));
    }
}
