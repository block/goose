//! Validator Agent - Read-only verification of builder work

use super::{BuildResult, TeamAgent, TeamCapabilities, TeamRole, TeamTask};
use crate::validators::{ValidationContext, ValidationResult, ValidatorRegistry};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Instant;

/// Validator agent with read-only access for verification
pub struct ValidatorAgent {
    id: String,
    name: String,
    capabilities: TeamCapabilities,
    working_dir: PathBuf,
    acceptance_criteria: Vec<String>,
    strict_mode: bool,
}

impl ValidatorAgent {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        working_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            capabilities: TeamCapabilities::validator(),
            working_dir: working_dir.into(),
            acceptance_criteria: Vec::new(),
            strict_mode: true,
        }
    }

    pub fn with_acceptance_criteria(mut self, criteria: Vec<String>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    pub fn lenient_mode(mut self) -> Self {
        self.strict_mode = false;
        self
    }

    async fn check_acceptance_criteria(&self, _build_result: &BuildResult) -> Vec<(String, bool)> {
        let mut results = Vec::new();

        for criterion in &self.acceptance_criteria {
            // In a real implementation, this would use LLM to verify each criterion
            let passed = true; // Placeholder
            results.push((criterion.clone(), passed));
        }

        results
    }

    async fn run_validators(&self, changed_files: &[String]) -> Vec<ValidationResult> {
        let context = ValidationContext::new(&self.working_dir)
            .with_changed_files(changed_files.iter().map(PathBuf::from).collect());

        let registry = ValidatorRegistry::with_defaults();
        registry.validate_all(&context).await
    }

    async fn verify_no_regressions(&self, _build_result: &BuildResult) -> Result<bool> {
        // In a real implementation, this would:
        // 1. Run the test suite
        // 2. Compare with baseline
        // 3. Check for performance regressions
        Ok(true)
    }
}

#[async_trait]
impl TeamAgent for ValidatorAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn role(&self) -> TeamRole {
        TeamRole::Validator
    }

    fn capabilities(&self) -> &TeamCapabilities {
        &self.capabilities
    }

    async fn execute_task(&self, _task: &TeamTask) -> Result<BuildResult> {
        // Validators don't execute tasks - they only validate
        Ok(BuildResult {
            success: false,
            output: "Validators do not execute tasks".to_string(),
            artifacts: Vec::new(),
            changed_files: Vec::new(),
            duration_ms: 0,
        })
    }

    async fn validate_work(
        &self,
        task: &TeamTask,
        build_result: &BuildResult,
    ) -> Result<ValidationResult> {
        let start = Instant::now();
        let mut all_passed = true;
        let mut fail_reasons = Vec::new();
        let mut actions = Vec::new();

        // 1. Run automated validators
        actions.push("Running automated validators".to_string());
        let validation_results = self.run_validators(&build_result.changed_files).await;

        for result in &validation_results {
            if !result.ok {
                all_passed = false;
                if let Some(reason) = &result.fail_reason {
                    fail_reasons.push(format!("{}: {}", result.validator_name, reason));
                }
            }
        }

        // 2. Check acceptance criteria
        if !self.acceptance_criteria.is_empty() {
            actions.push("Checking acceptance criteria".to_string());
            let criteria_results = self.check_acceptance_criteria(build_result).await;

            for (criterion, passed) in criteria_results {
                if !passed {
                    all_passed = false;
                    fail_reasons.push(format!("Acceptance criterion failed: {}", criterion));
                }
            }
        }

        // 3. Verify no regressions
        actions.push("Verifying no regressions".to_string());
        if !self.verify_no_regressions(build_result).await? {
            all_passed = false;
            fail_reasons.push("Regression detected".to_string());
        }

        let duration = start.elapsed().as_millis() as u64;

        if all_passed {
            Ok(ValidationResult::success(&self.name)
                .with_duration(duration)
                .with_action(format!("Validated task: {}", task.description)))
        } else {
            Ok(
                ValidationResult::failure(&self.name, fail_reasons.join("; "))
                    .with_duration(duration)
                    .with_recommendation("Fix the validation errors and rebuild"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_agent_creation() {
        let validator = ValidatorAgent::new("v1", "Test Validator", "/project");
        assert_eq!(validator.id(), "v1");
        assert_eq!(validator.name(), "Test Validator");
        assert_eq!(validator.role(), TeamRole::Validator);
        assert!(!validator.capabilities().can_write);
    }

    #[test]
    fn test_validator_capabilities() {
        let validator = ValidatorAgent::new("v1", "Test", "/project");
        let caps = validator.capabilities();

        assert!(!caps.can_write);
        assert!(!caps.can_edit);
        assert!(caps.can_execute); // Can run tests
        assert!(caps.can_read);
        assert!(!caps.can_spawn_subagents);
    }

    #[test]
    fn test_validator_with_criteria() {
        let validator =
            ValidatorAgent::new("v1", "Test", "/project").with_acceptance_criteria(vec![
                "All tests pass".to_string(),
                "No TODOs in production code".to_string(),
            ]);

        assert_eq!(validator.acceptance_criteria.len(), 2);
    }
}
