//! Prompt Errors
//!
//! Error types for the prompts module.

use thiserror::Error;

/// Errors that can occur in the prompts module
#[derive(Debug, Error)]
pub enum PromptError {
    /// Pattern not found in registry
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),

    /// Template not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Variable not provided for template
    #[error("Missing variable: {0}")]
    MissingVariable(String),

    /// Invalid variable value
    #[error("Invalid variable value for '{name}': {reason}")]
    InvalidVariable { name: String, reason: String },

    /// Prompt too long
    #[error("Prompt too long: {length} characters (max: {max})")]
    PromptTooLong { length: usize, max: usize },

    /// Template parsing error
    #[error("Template parsing error: {0}")]
    TemplateParse(String),

    /// Pattern validation error
    #[error("Pattern validation error: {0}")]
    PatternValidation(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlParse(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PromptError {
    /// Create a missing variable error
    pub fn missing_variable(name: impl Into<String>) -> Self {
        Self::MissingVariable(name.into())
    }

    /// Create an invalid variable error
    pub fn invalid_variable(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidVariable {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Create a template parse error
    pub fn template_parse(msg: impl Into<String>) -> Self {
        Self::TemplateParse(msg.into())
    }

    /// Create a pattern validation error
    pub fn pattern_validation(msg: impl Into<String>) -> Self {
        Self::PatternValidation(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PromptError::PatternNotFound("test".to_string());
        assert!(err.to_string().contains("test"));

        let err = PromptError::MissingVariable("name".to_string());
        assert!(err.to_string().contains("name"));

        let err = PromptError::PromptTooLong {
            length: 100,
            max: 50,
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("50"));
    }

    #[test]
    fn test_error_constructors() {
        let err = PromptError::missing_variable("var");
        assert!(matches!(err, PromptError::MissingVariable(_)));

        let err = PromptError::invalid_variable("var", "reason");
        assert!(matches!(err, PromptError::InvalidVariable { .. }));

        let err = PromptError::internal("test");
        assert!(matches!(err, PromptError::Internal(_)));
    }
}
