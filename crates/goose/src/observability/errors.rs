//! Observability Error Types

use thiserror::Error;

/// Observability-specific errors
#[derive(Error, Debug)]
pub enum ObservabilityError {
    /// Model pricing not found
    #[error("Model pricing not found: {model}")]
    PricingNotFound { model: String },

    /// Session not found
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    /// Export error
    #[error("Failed to export report: {reason}")]
    ExportError { reason: String },

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Metrics error
    #[error("Metrics error: {reason}")]
    MetricsError { reason: String },

    /// Configuration error
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ObservabilityError {
    /// Create a new export error
    pub fn export(reason: impl Into<String>) -> Self {
        Self::ExportError {
            reason: reason.into(),
        }
    }

    /// Create a new metrics error
    pub fn metrics(reason: impl Into<String>) -> Self {
        Self::MetricsError {
            reason: reason.into(),
        }
    }

    /// Create a new config error
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
        let err = ObservabilityError::PricingNotFound {
            model: "unknown-model".to_string(),
        };
        assert!(err.to_string().contains("unknown-model"));
    }

    #[test]
    fn test_error_constructors() {
        let export_err = ObservabilityError::export("failed to write");
        assert!(matches!(export_err, ObservabilityError::ExportError { .. }));

        let metrics_err = ObservabilityError::metrics("counter overflow");
        assert!(matches!(metrics_err, ObservabilityError::MetricsError { .. }));

        let config_err = ObservabilityError::config("invalid interval");
        assert!(matches!(config_err, ObservabilityError::ConfigError { .. }));
    }
}
