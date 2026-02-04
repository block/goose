//! Provider routing errors

use thiserror::Error;

/// Errors that can occur in provider routing
#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Endpoint not found: {0}")]
    EndpointNotFound(String),

    #[error("Model not found: {model} for provider {provider}")]
    ModelNotFound { provider: String, model: String },

    #[error("Capability mismatch: required {required}, available {available}")]
    CapabilityMismatch { required: String, available: String },

    #[error("Project policy violation: {0}")]
    PolicyViolation(String),

    #[error("Provider switch failed: {reason}")]
    SwitchFailed { reason: String },

    #[error("Authentication failed for endpoint {endpoint}: {message}")]
    AuthenticationFailed { endpoint: String, message: String },

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Export/import error: {0}")]
    ExportImportError(String),

    #[error("Handoff generation failed: {0}")]
    HandoffError(String),

    #[error("Project is pinned to provider {provider}")]
    ProjectPinned { provider: String },

    #[error("No fallback providers available")]
    NoFallbackAvailable,

    #[error("All providers exhausted")]
    AllProvidersFailed,

    #[error("Health check failed for endpoint {endpoint}: {message}")]
    HealthCheckFailed { endpoint: String, message: String },

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl RoutingError {
    pub fn provider_not_found(name: impl Into<String>) -> Self {
        Self::ProviderNotFound(name.into())
    }

    pub fn endpoint_not_found(id: impl Into<String>) -> Self {
        Self::EndpointNotFound(id.into())
    }

    pub fn model_not_found(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self::ModelNotFound {
            provider: provider.into(),
            model: model.into(),
        }
    }

    pub fn policy_violation(message: impl Into<String>) -> Self {
        Self::PolicyViolation(message.into())
    }

    pub fn switch_failed(reason: impl Into<String>) -> Self {
        Self::SwitchFailed {
            reason: reason.into(),
        }
    }

    pub fn auth_failed(endpoint: impl Into<String>, message: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            endpoint: endpoint.into(),
            message: message.into(),
        }
    }

    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError(message.into())
    }

    pub fn project_pinned(provider: impl Into<String>) -> Self {
        Self::ProjectPinned {
            provider: provider.into(),
        }
    }

    pub fn health_check_failed(endpoint: impl Into<String>, message: impl Into<String>) -> Self {
        Self::HealthCheckFailed {
            endpoint: endpoint.into(),
            message: message.into(),
        }
    }

    /// Whether this error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::NetworkError(_)
                | Self::HealthCheckFailed { .. }
                | Self::SwitchFailed { .. }
                | Self::NoFallbackAvailable
        )
    }

    /// Whether this error indicates a configuration issue
    pub fn is_config_issue(&self) -> bool {
        matches!(
            self,
            Self::ProviderNotFound(_)
                | Self::EndpointNotFound(_)
                | Self::ModelNotFound { .. }
                | Self::ConfigError(_)
                | Self::AuthenticationFailed { .. }
                | Self::PolicyViolation(_)
        )
    }
}

pub type RoutingResult<T> = Result<T, RoutingError>;
