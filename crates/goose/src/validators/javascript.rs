//! JavaScript/TypeScript Validator - eslint, tsc

use super::{
    IssueSeverity, ValidationContext, ValidationDetails, ValidationIssue, ValidationResult,
    Validator,
};
use anyhow::Result;
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct JavaScriptValidator {
    run_eslint: bool,
    run_tsc: bool,
}

impl JavaScriptValidator {
    pub fn new() -> Self {
        Self {
            run_eslint: true,
            run_tsc: true,
        }
    }

    async fn run_npx(
        &self,
        args: &[&str],
        working_dir: &std::path::Path,
    ) -> Result<(bool, String, String)> {
        let output = Command::new("npx")
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

    fn parse_eslint_output(&self, output: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for line in output.lines() {
            if line.contains("error") {
                issues.push(ValidationIssue {
                    file: None,
                    line: None,
                    column: None,
                    message: line.to_string(),
                    code: None,
                    severity: IssueSeverity::Error,
                });
            } else if line.contains("warning") {
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

impl Default for JavaScriptValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for JavaScriptValidator {
    fn name(&self) -> &str {
        "javascript"
    }

    fn description(&self) -> &str {
        "Validates JavaScript/TypeScript code with eslint and tsc"
    }

    fn should_run(&self, context: &ValidationContext) -> bool {
        context.has_extension("js")
            || context.has_extension("ts")
            || context.has_extension("jsx")
            || context.has_extension("tsx")
            || context.working_dir.join("package.json").exists()
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut details = ValidationDetails::default();
        let mut actions = Vec::new();
        let mut all_passed = true;
        let mut fail_reasons = Vec::new();

        // ESLint
        if self.run_eslint {
            actions.push("Running eslint".to_string());
            match self
                .run_npx(
                    &["eslint", ".", "--ext", ".js,.jsx,.ts,.tsx"],
                    &context.working_dir,
                )
                .await
            {
                Ok((success, stdout, _stderr)) => {
                    if !success {
                        all_passed = false;
                        fail_reasons.push("eslint found issues".to_string());
                        details.errors.extend(self.parse_eslint_output(&stdout));
                    }
                }
                Err(_) => {
                    details
                        .info
                        .push("eslint not available, skipping".to_string());
                }
            }
        }

        // TypeScript compiler check
        if self.run_tsc && context.working_dir.join("tsconfig.json").exists() {
            actions.push("Running tsc --noEmit".to_string());
            match self
                .run_npx(&["tsc", "--noEmit"], &context.working_dir)
                .await
            {
                Ok((success, stdout, stderr)) => {
                    if !success {
                        all_passed = false;
                        fail_reasons.push("TypeScript compilation errors".to_string());
                        details.errors.extend(self.parse_eslint_output(&stdout));
                        details.errors.extend(self.parse_eslint_output(&stderr));
                    }
                }
                Err(_) => {
                    details
                        .info
                        .push("tsc not available, skipping type check".to_string());
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
                    .with_recommendation("Fix the reported JavaScript/TypeScript errors"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_js_validator_should_run() {
        let validator = JavaScriptValidator::new();

        let context =
            ValidationContext::new("/project").with_changed_files(vec![PathBuf::from("app.ts")]);
        assert!(validator.should_run(&context));

        let context =
            ValidationContext::new("/project").with_changed_files(vec![PathBuf::from("main.rs")]);
        assert!(!validator.should_run(&context));
    }
}
