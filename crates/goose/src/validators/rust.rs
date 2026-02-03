//! Rust Validator - cargo build, cargo test, cargo clippy, cargo fmt

use super::{
    IssueSeverity, ValidationContext, ValidationDetails, ValidationIssue, ValidationResult,
    Validator,
};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct RustValidator {
    run_build: bool,
    run_test: bool,
    run_clippy: bool,
    run_fmt_check: bool,
}

impl RustValidator {
    pub fn new() -> Self {
        Self {
            run_build: true,
            run_test: true,
            run_clippy: true,
            run_fmt_check: true,
        }
    }

    pub fn build_only() -> Self {
        Self {
            run_build: true,
            run_test: false,
            run_clippy: false,
            run_fmt_check: false,
        }
    }

    async fn run_cargo_command(
        &self,
        args: &[&str],
        working_dir: &std::path::Path,
    ) -> Result<(bool, String, String)> {
        let output = Command::new("cargo")
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

    fn parse_cargo_errors(&self, output: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if line.contains("error[E") || line.starts_with("error:") {
                issues.push(ValidationIssue {
                    file: None,
                    line: None,
                    column: None,
                    message: line.to_string(),
                    code: None,
                    severity: IssueSeverity::Error,
                });
            } else if line.contains("warning:") {
                issues.push(ValidationIssue {
                    file: None,
                    line: None,
                    column: None,
                    message: line.to_string(),
                    code: None,
                    severity: IssueSeverity::Warning,
                });
            }
        }

        issues
    }
}

impl Default for RustValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for RustValidator {
    fn name(&self) -> &str {
        "rust"
    }

    fn description(&self) -> &str {
        "Validates Rust code with cargo build, test, clippy, and fmt"
    }

    fn should_run(&self, context: &ValidationContext) -> bool {
        context.has_extension("rs") || context.working_dir.join("Cargo.toml").exists()
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut details = ValidationDetails::default();
        let mut actions = Vec::new();
        let mut all_passed = true;
        let mut fail_reasons = Vec::new();

        // cargo build
        if self.run_build {
            actions.push("Running cargo build".to_string());
            let (success, _stdout, stderr) = self
                .run_cargo_command(&["build", "--all-targets"], &context.working_dir)
                .await?;

            if !success {
                all_passed = false;
                fail_reasons.push("cargo build failed".to_string());
                details.errors.extend(self.parse_cargo_errors(&stderr));
            }
        }

        // cargo test
        if self.run_test && all_passed {
            actions.push("Running cargo test".to_string());
            let (success, _stdout, stderr) = self
                .run_cargo_command(&["test", "--no-fail-fast"], &context.working_dir)
                .await?;

            if !success {
                all_passed = false;
                fail_reasons.push("cargo test failed".to_string());
                details.errors.extend(self.parse_cargo_errors(&stderr));
            }
        }

        // cargo clippy
        if self.run_clippy && all_passed {
            actions.push("Running cargo clippy".to_string());
            let (success, _stdout, stderr) = self
                .run_cargo_command(
                    &["clippy", "--all-targets", "--", "-D", "warnings"],
                    &context.working_dir,
                )
                .await?;

            if !success {
                all_passed = false;
                fail_reasons.push("cargo clippy found issues".to_string());
                details.warnings.extend(self.parse_cargo_errors(&stderr));
            }
        }

        // cargo fmt --check
        if self.run_fmt_check && all_passed {
            actions.push("Running cargo fmt --check".to_string());
            let (success, _stdout, _stderr) = self
                .run_cargo_command(&["fmt", "--", "--check"], &context.working_dir)
                .await?;

            if !success {
                all_passed = false;
                fail_reasons.push("cargo fmt found formatting issues".to_string());
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
                    .with_recommendation("Fix the reported errors and run validation again"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rust_validator_should_run() {
        let validator = RustValidator::new();

        let context = ValidationContext::new("/project")
            .with_changed_files(vec![PathBuf::from("src/main.rs")]);
        assert!(validator.should_run(&context));

        let context =
            ValidationContext::new("/project").with_changed_files(vec![PathBuf::from("script.py")]);
        assert!(!validator.should_run(&context));
    }

    #[test]
    fn test_parse_cargo_errors() {
        let validator = RustValidator::new();
        let output = r#"
error[E0425]: cannot find value `x` in this scope
warning: unused variable: `y`
        "#;

        let issues = validator.parse_cargo_errors(output);
        assert_eq!(issues.len(), 2);
        assert!(matches!(issues[0].severity, IssueSeverity::Error));
        assert!(matches!(issues[1].severity, IssueSeverity::Warning));
    }
}
