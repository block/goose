//! Python Validator - ruff, mypy/pyright

use super::{
    IssueSeverity, ValidationContext, ValidationDetails, ValidationIssue, ValidationResult,
    Validator,
};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct PythonValidator {
    run_ruff: bool,
    run_type_check: bool,
}

impl PythonValidator {
    pub fn new() -> Self {
        Self {
            run_ruff: true,
            run_type_check: true,
        }
    }

    async fn run_command(
        &self,
        cmd: &str,
        args: &[&str],
        working_dir: &std::path::Path,
    ) -> Result<(bool, String, String)> {
        let output = Command::new(cmd)
            .args(args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((success, stdout, stderr))
    }

    fn parse_ruff_output(&self, output: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if line.contains(":") && (line.contains("error") || line.contains("warning")) {
                issues.push(ValidationIssue {
                    file: None,
                    line: None,
                    column: None,
                    message: line.to_string(),
                    code: None,
                    severity: if line.contains("error") {
                        IssueSeverity::Error
                    } else {
                        IssueSeverity::Warning
                    },
                });
            }
        }

        issues
    }
}

impl Default for PythonValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for PythonValidator {
    fn name(&self) -> &str {
        "python"
    }

    fn description(&self) -> &str {
        "Validates Python code with ruff and type checking"
    }

    fn should_run(&self, context: &ValidationContext) -> bool {
        context.has_extension("py")
            || context.working_dir.join("pyproject.toml").exists()
            || context.working_dir.join("requirements.txt").exists()
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut details = ValidationDetails::default();
        let mut actions = Vec::new();
        let mut all_passed = true;
        let mut fail_reasons = Vec::new();

        // ruff check
        if self.run_ruff {
            actions.push("Running ruff check".to_string());
            match self
                .run_command("ruff", &["check", "."], &context.working_dir)
                .await
            {
                Ok((success, stdout, _stderr)) => {
                    if !success {
                        all_passed = false;
                        fail_reasons.push("ruff found linting issues".to_string());
                        details.errors.extend(self.parse_ruff_output(&stdout));
                    }
                }
                Err(_) => {
                    details
                        .info
                        .push("ruff not available, skipping lint check".to_string());
                }
            }
        }

        // Type checking (try mypy first, then pyright)
        if self.run_type_check && all_passed {
            actions.push("Running type check".to_string());

            // Try mypy first
            match self
                .run_command(
                    "mypy",
                    &[".", "--ignore-missing-imports"],
                    &context.working_dir,
                )
                .await
            {
                Ok((success, stdout, _stderr)) => {
                    if !success {
                        all_passed = false;
                        fail_reasons.push("mypy found type errors".to_string());
                        details.errors.extend(self.parse_ruff_output(&stdout));
                    }
                }
                Err(_) => {
                    // Try pyright as fallback
                    match self
                        .run_command("pyright", &["."], &context.working_dir)
                        .await
                    {
                        Ok((success, stdout, _stderr)) => {
                            if !success {
                                all_passed = false;
                                fail_reasons.push("pyright found type errors".to_string());
                                details.errors.extend(self.parse_ruff_output(&stdout));
                            }
                        }
                        Err(_) => {
                            details.info.push(
                                "No type checker available (mypy/pyright), skipping".to_string(),
                            );
                        }
                    }
                }
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        if all_passed {
            Ok(ValidationResult::success(self.name())
                .with_duration(duration)
                .with_details(details))
        } else {
            Ok(
                ValidationResult::failure(self.name(), fail_reasons.join("; "))
                    .with_duration(duration)
                    .with_details(details)
                    .with_recommendation("Fix the reported Python errors and run validation again"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_python_validator_should_run() {
        let validator = PythonValidator::new();

        // Use temp dir to avoid matching existing pyproject.toml files
        let temp_dir = std::env::temp_dir().join("goose_test_nonexistent");

        let context =
            ValidationContext::new(&temp_dir).with_changed_files(vec![PathBuf::from("script.py")]);
        assert!(validator.should_run(&context));

        let context =
            ValidationContext::new(&temp_dir).with_changed_files(vec![PathBuf::from("main.rs")]);
        assert!(!validator.should_run(&context));
    }
}
