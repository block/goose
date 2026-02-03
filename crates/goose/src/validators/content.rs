//! Content Validators - file exists, file contains, no TODOs

use super::{
    IssueSeverity, ValidationContext, ValidationDetails, ValidationIssue, ValidationResult,
    Validator,
};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Validator that checks if specified files exist
pub struct FileExistsValidator {
    required_files: Vec<PathBuf>,
}

impl FileExistsValidator {
    pub fn new() -> Self {
        Self {
            required_files: Vec::new(),
        }
    }

    pub fn with_required_files(mut self, files: Vec<PathBuf>) -> Self {
        self.required_files = files;
        self
    }
}

impl Default for FileExistsValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for FileExistsValidator {
    fn name(&self) -> &str {
        "file_exists"
    }

    fn description(&self) -> &str {
        "Validates that required files exist"
    }

    fn should_run(&self, _context: &ValidationContext) -> bool {
        !self.required_files.is_empty()
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut missing = Vec::new();
        let mut details = ValidationDetails::default();

        for file in &self.required_files {
            let full_path = context.working_dir.join(file);
            if !full_path.exists() {
                missing.push(file.clone());
                details.errors.push(ValidationIssue {
                    file: Some(file.clone()),
                    line: None,
                    column: None,
                    message: format!("Required file not found: {}", file.display()),
                    code: Some("FILE_NOT_FOUND".to_string()),
                    severity: IssueSeverity::Error,
                });
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        if missing.is_empty() {
            Ok(ValidationResult::success(self.name())
                .with_duration(duration)
                .with_action(format!(
                    "Verified {} required files exist",
                    self.required_files.len()
                )))
        } else {
            Ok(ValidationResult::failure(
                self.name(),
                format!("{} required file(s) not found", missing.len()),
            )
            .with_duration(duration)
            .with_details(details)
            .with_recommendation("Create the missing files"))
        }
    }
}

/// Validator that checks if files contain required content
pub struct FileContainsValidator {
    checks: Vec<(PathBuf, Vec<String>)>,
}

impl FileContainsValidator {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn require_content(mut self, file: PathBuf, patterns: Vec<String>) -> Self {
        self.checks.push((file, patterns));
        self
    }
}

impl Default for FileContainsValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for FileContainsValidator {
    fn name(&self) -> &str {
        "file_contains"
    }

    fn description(&self) -> &str {
        "Validates that files contain required content"
    }

    fn should_run(&self, _context: &ValidationContext) -> bool {
        !self.checks.is_empty()
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut details = ValidationDetails::default();
        let mut all_passed = true;

        for (file, patterns) in &self.checks {
            let full_path = context.working_dir.join(file);

            if !full_path.exists() {
                all_passed = false;
                details.errors.push(ValidationIssue {
                    file: Some(file.clone()),
                    line: None,
                    column: None,
                    message: format!("File not found: {}", file.display()),
                    code: Some("FILE_NOT_FOUND".to_string()),
                    severity: IssueSeverity::Error,
                });
                continue;
            }

            let content = tokio::fs::read_to_string(&full_path).await?;

            for pattern in patterns {
                if !content.contains(pattern) {
                    all_passed = false;
                    details.errors.push(ValidationIssue {
                        file: Some(file.clone()),
                        line: None,
                        column: None,
                        message: format!("Required content not found: '{}'", pattern),
                        code: Some("CONTENT_MISSING".to_string()),
                        severity: IssueSeverity::Error,
                    });
                }
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        if all_passed {
            Ok(ValidationResult::success(self.name())
                .with_duration(duration)
                .with_action("Verified all required content present"))
        } else {
            Ok(
                ValidationResult::failure(self.name(), "Required content missing")
                    .with_duration(duration)
                    .with_details(details),
            )
        }
    }
}

/// Validator that checks for TODO/FIXME/XXX comments in production code
pub struct NoTodosValidator {
    patterns: Vec<String>,
    exclude_paths: Vec<String>,
}

impl NoTodosValidator {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "XXX".to_string(),
                "HACK".to_string(),
            ],
            exclude_paths: vec![
                "test".to_string(),
                "tests".to_string(),
                "spec".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
            ],
        }
    }

    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    pub fn with_exclude_paths(mut self, paths: Vec<String>) -> Self {
        self.exclude_paths = paths;
        self
    }

    async fn run_ripgrep(
        &self,
        pattern: &str,
        working_dir: &std::path::Path,
    ) -> Result<Vec<String>> {
        let exclude_args: Vec<String> = self
            .exclude_paths
            .iter()
            .flat_map(|p| vec!["-g".to_string(), format!("!{}/**", p)])
            .collect();

        let mut cmd = Command::new("rg");
        cmd.arg("-n")
            .arg("--no-heading")
            .arg(pattern)
            .args(&exclude_args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        Ok(stdout.lines().map(String::from).collect())
    }
}

impl Default for NoTodosValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validator for NoTodosValidator {
    fn name(&self) -> &str {
        "no_todos"
    }

    fn description(&self) -> &str {
        "Validates that production code has no TODO/FIXME/XXX comments"
    }

    fn should_run(&self, _context: &ValidationContext) -> bool {
        true
    }

    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult> {
        let start = std::time::Instant::now();
        let mut details = ValidationDetails::default();
        let mut found_todos = Vec::new();

        for pattern in &self.patterns {
            match self.run_ripgrep(pattern, &context.working_dir).await {
                Ok(matches) => {
                    for match_line in matches {
                        found_todos.push(match_line.clone());
                        details.warnings.push(ValidationIssue {
                            file: None,
                            line: None,
                            column: None,
                            message: match_line,
                            code: Some(pattern.clone()),
                            severity: IssueSeverity::Warning,
                        });
                    }
                }
                Err(_) => {
                    details
                        .info
                        .push("ripgrep not available, skipping TODO scan".to_string());
                }
            }
        }

        let duration = start.elapsed().as_millis() as u64;

        if found_todos.is_empty() {
            Ok(ValidationResult::success(self.name())
                .with_duration(duration)
                .with_action("Scanned for TODO/FIXME/XXX comments"))
        } else {
            Ok(ValidationResult::failure(
                self.name(),
                format!("Found {} TODO/FIXME/XXX comments", found_todos.len()),
            )
            .with_duration(duration)
            .with_details(details)
            .with_recommendation("Address or remove TODO comments before completion"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_exists_validator() {
        let validator =
            FileExistsValidator::new().with_required_files(vec![PathBuf::from("README.md")]);

        assert_eq!(validator.name(), "file_exists");
        assert!(validator.should_run(&ValidationContext::new("/project")));
    }

    #[test]
    fn test_no_todos_validator() {
        let validator = NoTodosValidator::new();
        assert_eq!(validator.name(), "no_todos");
        assert!(validator.patterns.contains(&"TODO".to_string()));
    }

    #[test]
    fn test_file_contains_validator() {
        let validator = FileContainsValidator::new()
            .require_content(PathBuf::from("README.md"), vec!["# Title".to_string()]);

        assert_eq!(validator.name(), "file_contains");
    }
}
