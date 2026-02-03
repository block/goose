//! MCP Gateway Error Types
//!
//! Error types specific to MCP Gateway operations.

use thiserror::Error;

/// Errors that can occur in MCP Gateway operations
#[derive(Debug, Error)]
pub enum GatewayError {
    /// Tool not found in registry
    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },

    /// Server not available
    #[error("Server not available: {server_id}")]
    ServerNotAvailable { server_id: String },

    /// Server not found
    #[error("Server not found: {0}")]
    ServerNotFound(String),

    /// Server unavailable for execution
    #[error("Server unavailable: {0}")]
    ServerUnavailable(String),

    /// Server connection failed
    #[error("Server connection failed: {server_id} - {reason}")]
    ServerConnectionFailed { server_id: String, reason: String },

    /// Permission denied
    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    /// Credential not found
    #[error("Credential not found for server: {server_id}")]
    CredentialNotFound { server_id: String },

    /// Credential expired
    #[error("Credential expired for server: {server_id}")]
    CredentialExpired { server_id: String },

    /// Invalid credential
    #[error("Invalid credential for server: {server_id}")]
    InvalidCredential { server_id: String },

    /// Tool execution failed
    #[error("Tool execution failed: {tool_name} - {reason}")]
    ExecutionFailed { tool_name: String, reason: String },

    /// Tool execution timeout
    #[error("Tool execution timeout: {tool_name} after {timeout_ms}ms")]
    ExecutionTimeout { tool_name: String, timeout_ms: u64 },

    /// Invalid arguments
    #[error("Invalid arguments for tool: {tool_name} - {reason}")]
    InvalidArguments { tool_name: String, reason: String },

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Policy violation
    #[error("Policy violation: {policy_id} - {reason}")]
    PolicyViolation { policy_id: String, reason: String },

    /// Audit logging failed
    #[error("Audit logging failed: {0}")]
    AuditError(String),

    /// Bundle not found
    #[error("Bundle not found: {bundle_id}")]
    BundleNotFound { bundle_id: String },

    /// Allow list expired
    #[error("Allow list expired for bundle: {bundle_id}")]
    AllowListExpired { bundle_id: String },

    /// Internal error
    #[error("Internal gateway error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for GatewayError {
    fn from(err: serde_json::Error) -> Self {
        GatewayError::SerializationError(err.to_string())
    }
}

impl From<std::io::Error> for GatewayError {
    fn from(err: std::io::Error) -> Self {
        GatewayError::Internal(format!("IO error: {}", err))
    }
}
