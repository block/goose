//! Policy Error Types

use thiserror::Error;

/// Policy-specific errors
#[derive(Error, Debug)]
pub enum PolicyError {
    /// Rule set not found
    #[error("Rule set not found: {name}")]
    RuleSetNotFound { name: String },

    /// Rule not found
    #[error("Rule not found: {rule_id}")]
    RuleNotFound { rule_id: String },

    /// Invalid condition
    #[error("Invalid condition: {reason}")]
    InvalidCondition { reason: String },

    /// Invalid action
    #[error("Invalid action: {reason}")]
    InvalidAction { reason: String },

    /// YAML parse error
    #[error("YAML parse error: {0}")]
    YamlParseError(#[from] serde_yaml::Error),

    /// JSON parse error
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Evaluation error
    #[error("Evaluation error: {reason}")]
    EvaluationError { reason: String },

    /// Timeout error
    #[error("Evaluation timeout: exceeded {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Field not found
    #[error("Field not found: {field}")]
    FieldNotFound { field: String },

    /// Type mismatch
    #[error("Type mismatch for field {field}: expected {expected}, got {actual}")]
    TypeMismatch {
        field: String,
        expected: String,
        actual: String,
    },

    /// Regex error
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    /// Action execution error
    #[error("Action execution error: {reason}")]
    ActionError { reason: String },

    /// Configuration error
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PolicyError {
    /// Create an invalid condition error
    pub fn invalid_condition(reason: impl Into<String>) -> Self {
        Self::InvalidCondition {
            reason: reason.into(),
        }
    }

    /// Create an invalid action error
    pub fn invalid_action(reason: impl Into<String>) -> Self {
        Self::InvalidAction {
            reason: reason.into(),
        }
    }

    /// Create an evaluation error
    pub fn evaluation(reason: impl Into<String>) -> Self {
        Self::EvaluationError {
            reason: reason.into(),
        }
    }

    /// Create an action error
    pub fn action(reason: impl Into<String>) -> Self {
        Self::ActionError {
            reason: reason.into(),
        }
    }

    /// Create a config error
    pub fn config(reason: impl Into<String>) -> Self {
        Self::ConfigError {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PolicyError::RuleNotFound {
            rule_id: "test-rule".to_string(),
        };
        assert!(err.to_string().contains("test-rule"));
    }

    #[test]
    fn test_error_constructors() {
        let cond_err = PolicyError::invalid_condition("missing field");
        assert!(matches!(cond_err, PolicyError::InvalidCondition { .. }));

        let action_err = PolicyError::invalid_action("unknown action type");
        assert!(matches!(action_err, PolicyError::InvalidAction { .. }));

        let eval_err = PolicyError::evaluation("timeout");
        assert!(matches!(eval_err, PolicyError::EvaluationError { .. }));
    }
}
