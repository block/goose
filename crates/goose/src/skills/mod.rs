//! Skills Pack Module - Installable enforcement modules
//!
//! Provides Claude Code-style skills capabilities:
//! - Skills as markdown files with YAML frontmatter
//! - Prompt templates for orchestration
//! - Runnable validators
//! - Default gates (what must pass before task completion)
//! - Auto-loading from .goose/skills/

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Skill pack definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPack {
    pub name: String,
    pub description: String,
    pub version: String,
    pub prompts: Vec<PromptTemplate>,
    pub validators: Vec<ValidatorConfig>,
    pub gates: GateConfig,
    pub hooks: Vec<HookConfig>,
    pub metadata: HashMap<String, String>,
}

impl SkillPack {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: "1.0.0".to_string(),
            prompts: Vec::new(),
            validators: Vec::new(),
            gates: GateConfig::default(),
            hooks: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_prompts(mut self, prompts: Vec<PromptTemplate>) -> Self {
        self.prompts = prompts;
        self
    }

    pub fn with_validators(mut self, validators: Vec<ValidatorConfig>) -> Self {
        self.validators = validators;
        self
    }

    pub fn with_gates(mut self, gates: GateConfig) -> Self {
        self.gates = gates;
        self
    }
}

/// Prompt template for orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<TemplateVariable>,
    pub hooks: Vec<String>,
}

impl PromptTemplate {
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            template: template.into(),
            variables: Vec::new(),
            hooks: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_variables(mut self, variables: Vec<TemplateVariable>) -> Self {
        self.variables = variables;
        self
    }

    /// Render the template with provided values
    pub fn render(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        for (key, value) in values {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }
}

/// Variable in a prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub name: String,
    pub command: String,
    pub trigger: ValidatorTrigger,
    pub blocking: bool,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorTrigger {
    #[default]
    PostToolUse,
    PreComplete,
    OnFileChange,
    Manual,
}

/// Gate configuration - what must pass before completion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateConfig {
    pub pre_complete: Vec<String>,
    pub post_tool_use: Vec<String>,
    pub pre_commit: Vec<String>,
}

impl GateConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pre_complete(mut self, commands: Vec<String>) -> Self {
        self.pre_complete = commands;
        self
    }

    pub fn with_post_tool_use(mut self, validators: Vec<String>) -> Self {
        self.post_tool_use = validators;
        self
    }
}

/// Hook configuration for skills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub event: String,
    pub command: String,
    pub async_execution: bool,
}

/// Manager for skill packs
pub struct SkillManager {
    skills: HashMap<String, SkillPack>,
    skill_dirs: Vec<PathBuf>,
    loaded_skills: Vec<String>,
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            skill_dirs: vec![
                PathBuf::from(".goose/skills"),
                dirs::home_dir()
                    .map(|h| h.join(".goose/skills"))
                    .unwrap_or_default(),
            ],
            loaded_skills: Vec::new(),
        }
    }

    pub fn with_skill_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.skill_dirs = dirs;
        self
    }

    /// Install a skill pack
    pub fn install(&mut self, skill: SkillPack) -> Result<()> {
        let name = skill.name.clone();
        self.skills.insert(name.clone(), skill);
        self.loaded_skills.push(name);
        Ok(())
    }

    /// Load a skill by name
    pub fn load(&self, name: &str) -> Option<&SkillPack> {
        self.skills.get(name)
    }

    /// List all installed skills
    pub fn list(&self) -> Vec<&SkillPack> {
        self.skills.values().collect()
    }

    /// Get gates for a skill
    pub fn get_gates(&self, name: &str) -> Option<&GateConfig> {
        self.skills.get(name).map(|s| &s.gates)
    }

    /// Discover skills from configured directories
    pub async fn discover(&mut self) -> Result<Vec<String>> {
        let mut discovered = Vec::new();
        let dirs: Vec<PathBuf> = self.skill_dirs.clone();

        for dir in dirs {
            if !dir.exists() {
                continue;
            }

            let mut entries = tokio::fs::read_dir(&dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    let skill_file = path.join("skill.yaml");
                    if skill_file.exists() {
                        if let Ok(skill) = Self::load_skill_from_path(&skill_file).await {
                            discovered.push(skill.name.clone());
                            self.install(skill)?;
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    async fn load_skill_from_path(path: &PathBuf) -> Result<SkillPack> {
        let content = tokio::fs::read_to_string(path).await?;
        let skill: SkillPack = serde_yaml::from_str(&content)?;
        Ok(skill)
    }

    /// Get skill count
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Create the compound-engineering skill pack
    pub fn create_compound_engineering() -> SkillPack {
        SkillPack::new(
            "compound-engineering",
            "Enterprise team-based build/validate workflow",
        )
        .with_version("1.0.0")
        .with_prompts(vec![
            PromptTemplate::new("plan_with_team", templates::PLAN_WITH_TEAM)
                .with_description("Create a plan with team orchestration"),
            PromptTemplate::new("build_with_team", templates::BUILD_WITH_TEAM)
                .with_description("Execute build with builder/validator pairing"),
        ])
        .with_validators(vec![
            ValidatorConfig {
                name: "rust_validator".to_string(),
                command: "cargo build && cargo test && cargo clippy".to_string(),
                trigger: ValidatorTrigger::PreComplete,
                blocking: true,
                timeout_secs: 300,
            },
            ValidatorConfig {
                name: "no_todos".to_string(),
                command: "! rg -i 'TODO|FIXME|XXX' --type rust".to_string(),
                trigger: ValidatorTrigger::PostToolUse,
                blocking: false,
                timeout_secs: 30,
            },
        ])
        .with_gates(GateConfig {
            pre_complete: vec![
                "cargo build --release".to_string(),
                "cargo test --no-fail-fast".to_string(),
                "cargo clippy -D warnings".to_string(),
                "cargo fmt --check".to_string(),
            ],
            post_tool_use: vec!["no_todos".to_string()],
            pre_commit: vec!["cargo fmt".to_string()],
        })
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

// Embed default templates
mod templates {
    pub const PLAN_WITH_TEAM: &str = r#"## Objective
Create a detailed implementation plan with team orchestration.

## Team Members
{{TEAM_MEMBERS}}

## Tasks
{{TASKS}}

## Validation
{{VALIDATION_COMMANDS}}
"#;

    pub const BUILD_WITH_TEAM: &str = r#"## Build Phase
Execute the implementation with builder/validator pairing.

### Builder: {{BUILDER_NAME}}
- Full tool access
- Auto-validates with configured validators

### Validator: {{VALIDATOR_NAME}}  
- Read-only access
- Verifies acceptance criteria
"#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_pack_creation() {
        let skill = SkillPack::new("test-skill", "A test skill").with_version("2.0.0");

        assert_eq!(skill.name, "test-skill");
        assert_eq!(skill.version, "2.0.0");
    }

    #[test]
    fn test_prompt_template_render() {
        let template = PromptTemplate::new("test", "Hello {{NAME}}, welcome to {{PROJECT}}!");

        let mut values = HashMap::new();
        values.insert("NAME".to_string(), "Alice".to_string());
        values.insert("PROJECT".to_string(), "Goose".to_string());

        let result = template.render(&values);
        assert_eq!(result, "Hello Alice, welcome to Goose!");
    }

    #[test]
    fn test_skill_manager() {
        let mut manager = SkillManager::new();
        let skill = SkillPack::new("test", "Test skill");

        manager.install(skill).unwrap();

        assert_eq!(manager.count(), 1);
        assert!(manager.load("test").is_some());
    }

    #[test]
    fn test_gate_config() {
        let gates = GateConfig::new()
            .with_pre_complete(vec!["cargo test".to_string()])
            .with_post_tool_use(vec!["lint".to_string()]);

        assert_eq!(gates.pre_complete.len(), 1);
        assert_eq!(gates.post_tool_use.len(), 1);
    }

    #[test]
    fn test_compound_engineering_skill() {
        let skill = SkillManager::create_compound_engineering();

        assert_eq!(skill.name, "compound-engineering");
        assert!(!skill.validators.is_empty());
        assert!(!skill.gates.pre_complete.is_empty());
    }
}
