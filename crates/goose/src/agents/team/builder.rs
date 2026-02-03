//! Builder Agent - Full tool access for implementing features

use super::{BuildResult, TeamAgent, TeamCapabilities, TeamRole, TeamTask};
use crate::validators::ValidationResult;
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Instant;

/// Builder agent with full write/edit capabilities
pub struct BuilderAgent {
    id: String,
    name: String,
    capabilities: TeamCapabilities,
    working_dir: PathBuf,
    auto_validate: bool,
    validators: Vec<String>,
}

impl BuilderAgent {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        working_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            capabilities: TeamCapabilities::builder(),
            working_dir: working_dir.into(),
            auto_validate: true,
            validators: vec!["rust".to_string(), "no_todos".to_string()],
        }
    }

    pub fn with_validators(mut self, validators: Vec<String>) -> Self {
        self.validators = validators;
        self
    }

    pub fn disable_auto_validate(mut self) -> Self {
        self.auto_validate = false;
        self
    }

    async fn run_auto_validators(&self, changed_files: &[String]) -> Result<Vec<ValidationResult>> {
        use crate::validators::{ValidationContext, ValidatorRegistry};

        let context = ValidationContext::new(&self.working_dir)
            .with_changed_files(changed_files.iter().map(PathBuf::from).collect());

        let registry = ValidatorRegistry::with_defaults();
        let results = registry.validate_all(&context).await;

        Ok(results)
    }
}

#[async_trait]
impl TeamAgent for BuilderAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn role(&self) -> TeamRole {
        TeamRole::Builder
    }

    fn capabilities(&self) -> &TeamCapabilities {
        &self.capabilities
    }

    async fn execute_task(&self, task: &TeamTask) -> Result<BuildResult> {
        let start = Instant::now();

        // In a real implementation, this would:
        // 1. Parse the task description
        // 2. Generate code using the LLM
        // 3. Write files
        // 4. Run auto-validators (ruff, clippy, etc.)

        let changed_files = Vec::new(); // Would be populated by actual file operations
        let mut output = format!(
            "Builder {} executing task: {}\n",
            self.name, task.description
        );

        // Run auto-validators if enabled
        if self.auto_validate && !changed_files.is_empty() {
            let validation_results = self.run_auto_validators(&changed_files).await?;
            let all_passed = validation_results.iter().all(|r| r.ok);

            if !all_passed {
                let failures: Vec<_> = validation_results
                    .iter()
                    .filter(|r| !r.ok)
                    .map(|r| r.fail_reason.clone().unwrap_or_default())
                    .collect();
                output.push_str(&format!("Auto-validation failed: {:?}\n", failures));
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        Ok(BuildResult {
            success: true,
            output,
            artifacts: Vec::new(),
            changed_files,
            duration_ms: duration,
        })
    }

    async fn validate_work(
        &self,
        _task: &TeamTask,
        _build_result: &BuildResult,
    ) -> Result<ValidationResult> {
        // Builders don't validate - this is for the Validator agent
        Ok(ValidationResult::success("builder").with_action("Builder does not perform validation"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_agent_creation() {
        let builder = BuilderAgent::new("b1", "Test Builder", "/project");
        assert_eq!(builder.id(), "b1");
        assert_eq!(builder.name(), "Test Builder");
        assert_eq!(builder.role(), TeamRole::Builder);
        assert!(builder.capabilities().can_write);
    }

    #[test]
    fn test_builder_capabilities() {
        let builder = BuilderAgent::new("b1", "Test", "/project");
        let caps = builder.capabilities();

        assert!(caps.can_write);
        assert!(caps.can_edit);
        assert!(caps.can_execute);
        assert!(caps.can_read);
        assert!(caps.can_spawn_subagents);
    }
}
