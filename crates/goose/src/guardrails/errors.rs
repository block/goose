//! Guardrails error types

use thiserror::Error;

/// Errors that can occur during guardrails operations
#[derive(Error, Debug)]
pub enum GuardrailsError {
    /// Detector execution timed out
    #[error("Guardrails scan timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// One or more detectors failed
    #[error("Detector(s) failed: {errors}")]
    DetectorFailed { errors: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    /// Pattern compilation error
    #[error("Pattern compilation error: {pattern}")]
    PatternError { pattern: String },

    /// Invalid input
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl From<regex::Error> for GuardrailsError {
    fn from(err: regex::Error) -> Self {
        GuardrailsError::PatternError {
            pattern: err.to_string(),
        }
    }
}

impl From<anyhow::Error> for GuardrailsError {
    fn from(err: anyhow::Error) -> Self {
        GuardrailsError::Internal {
            message: err.to_string(),
        }
    }
}
