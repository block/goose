//! Validators Module for Goose
//!
//! Provides deterministic validation with:
//! - Rust validators (cargo build, cargo test, cargo clippy, cargo fmt)
//! - Python validators (ruff, mypy)
//! - JavaScript validators (eslint, tsc)
//! - Content validators (file exists, file contains, no TODOs)
//! - Security validators (secret scanning, dangerous patterns)

mod content;
mod javascript;
mod python;
mod rust;
mod security;

pub use content::{FileContainsValidator, FileExistsValidator, NoTodosValidator};
pub use javascript::JavaScriptValidator;
pub use python::PythonValidator;
pub use rust::RustValidator;
pub use security::SecurityValidator;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Context for validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub changed_files: Vec<PathBuf>,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub working_dir: PathBuf,
    pub project_type: Option<ProjectType>,
}

impl ValidationContext {
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        Self {
            changed_files: Vec::new(),
            tool_name: String::new(),
            tool_input: serde_json::Value::Null,
            working_dir: working_dir.into(),
            project_type: None,
        }
    }

    pub fn with_changed_files(mut self, files: Vec<PathBuf>) -> Self {
        self.changed_files = files;
        self
    }

    pub fn with_tool(mut self, name: impl Into<String>, input: serde_json::Value) -> Self {
        self.tool_name = name.into();
        self.tool_input = input;
        self
    }

    pub fn with_project_type(mut self, project_type: ProjectType) -> Self {
        self.project_type = Some(project_type);
        self
    }

    pub fn has_extension(&self, ext: &str) -> bool {
        self.changed_files.iter().any(|f| {
            f.extension()
                .map(|e| e.to_string_lossy().to_lowercase() == ext.to_lowercase())
                .unwrap_or(false)
        })
    }
}

/// Project type for determining which validators to use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Mixed,
}

/// Result of validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub validator_name: String,
    pub fail_reason: Option<String>,
    pub actions_taken: Vec<String>,
    pub evidence_paths: Vec<PathBuf>,
    pub next_recommendation: Option<String>,
    pub duration_ms: u64,
    pub details: ValidationDetails,
}

impl ValidationResult {
    pub fn success(validator_name: impl Into<String>) -> Self {
        Self {
            ok: true,
            validator_name: validator_name.into(),
            fail_reason: None,
            actions_taken: Vec::new(),
            evidence_paths: Vec::new(),
            next_recommendation: None,
            duration_ms: 0,
            details: ValidationDetails::default(),
        }
    }

    pub fn failure(validator_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            ok: false,
            validator_name: validator_name.into(),
            fail_reason: Some(reason.into()),
            actions_taken: Vec::new(),
            evidence_paths: Vec::new(),
            next_recommendation: None,
            duration_ms: 0,
            details: ValidationDetails::default(),
        }
    }

    pub fn with_recommendation(mut self, recommendation: impl Into<String>) -> Self {
        self.next_recommendation = Some(recommendation.into());
        self
    }

    pub fn with_evidence(mut self, path: PathBuf) -> Self {
        self.evidence_paths.push(path);
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.actions_taken.push(action.into());
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_details(mut self, details: ValidationDetails) -> Self {
        self.details = details;
        self
    }
}

/// Detailed validation information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationDetails {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    pub info: Vec<String>,
}

/// A single validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub file: Option<PathBuf>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub code: Option<String>,
    pub severity: IssueSeverity,
}

/// Severity of a validation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Trait for validators
#[async_trait]
pub trait Validator: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn should_run(&self, context: &ValidationContext) -> bool;
    async fn validate(&self, context: &ValidationContext) -> Result<ValidationResult>;
}

/// Registry of validators
pub struct ValidatorRegistry {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(RustValidator::new()));
        registry.register(Box::new(PythonValidator::new()));
        registry.register(Box::new(JavaScriptValidator::new()));
        registry.register(Box::new(NoTodosValidator::new()));
        registry.register(Box::new(SecurityValidator::new()));
        registry
    }

    pub fn register(&mut self, validator: Box<dyn Validator>) {
        self.validators.push(validator);
    }

    pub fn get_applicable(&self, context: &ValidationContext) -> Vec<&dyn Validator> {
        self.validators
            .iter()
            .filter(|v| v.should_run(context))
            .map(|v| v.as_ref())
            .collect()
    }

    pub async fn validate_all(&self, context: &ValidationContext) -> Vec<ValidationResult> {
        let applicable = self.get_applicable(context);
        let mut results = Vec::new();

        for validator in applicable {
            match validator.validate(context).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(ValidationResult::failure(
                        validator.name(),
                        format!("Validator error: {}", e),
                    ));
                }
            }
        }

        results
    }

    pub fn all_passed(results: &[ValidationResult]) -> bool {
        results.iter().all(|r| r.ok)
    }

    pub fn get_failures(results: &[ValidationResult]) -> Vec<&ValidationResult> {
        results.iter().filter(|r| !r.ok).collect()
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_context_has_extension() {
        let context = ValidationContext::new("/project").with_changed_files(vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
        ]);

        assert!(context.has_extension("rs"));
        assert!(!context.has_extension("py"));
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success("test_validator")
            .with_recommendation("Run cargo fmt")
            .with_action("Checked formatting");

        assert!(result.ok);
        assert!(result.fail_reason.is_none());
        assert_eq!(
            result.next_recommendation,
            Some("Run cargo fmt".to_string())
        );
    }

    #[test]
    fn test_validation_result_failure() {
        let result = ValidationResult::failure("test_validator", "Tests failed");

        assert!(!result.ok);
        assert_eq!(result.fail_reason, Some("Tests failed".to_string()));
    }

    #[test]
    fn test_validator_registry() {
        let registry = ValidatorRegistry::with_defaults();
        let context = ValidationContext::new("/project")
            .with_changed_files(vec![PathBuf::from("src/main.rs")]);

        let applicable = registry.get_applicable(&context);
        assert!(!applicable.is_empty());
    }

    #[test]
    fn test_all_passed() {
        let results = vec![
            ValidationResult::success("v1"),
            ValidationResult::success("v2"),
        ];
        assert!(ValidatorRegistry::all_passed(&results));

        let results = vec![
            ValidationResult::success("v1"),
            ValidationResult::failure("v2", "Failed"),
        ];
        assert!(!ValidatorRegistry::all_passed(&results));
    }
}
