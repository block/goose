//! Memory system errors
//!
//! This module defines error types for the memory subsystem.

use thiserror::Error;

/// Errors that can occur in the memory system
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Memory entry not found
    #[error("Memory entry not found: {0}")]
    NotFound(String),

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Embedding generation error
    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Capacity exceeded
    #[error("Memory capacity exceeded: {message}")]
    CapacityExceeded { message: String },

    /// Invalid memory type
    #[error("Invalid memory type: {0}")]
    InvalidMemoryType(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Consolidation error
    #[error("Consolidation error: {0}")]
    ConsolidationError(String),

    /// Retrieval error
    #[error("Retrieval error: {0}")]
    RetrievalError(String),

    /// Vector operation error
    #[error("Vector operation error: {0}")]
    VectorError(String),

    /// Backend not available
    #[error("Backend not available: {0}")]
    BackendUnavailable(String),

    /// Invalid query
    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Lock error
    #[error("Lock error: {0}")]
    LockError(String),
}

impl MemoryError {
    /// Create a not found error
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound(id.into())
    }

    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::StorageError(msg.into())
    }

    /// Create an embedding error
    pub fn embedding(msg: impl Into<String>) -> Self {
        Self::EmbeddingError(msg.into())
    }

    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// Create a capacity exceeded error
    pub fn capacity_exceeded(msg: impl Into<String>) -> Self {
        Self::CapacityExceeded {
            message: msg.into(),
        }
    }

    /// Create a consolidation error
    pub fn consolidation(msg: impl Into<String>) -> Self {
        Self::ConsolidationError(msg.into())
    }

    /// Create a retrieval error
    pub fn retrieval(msg: impl Into<String>) -> Self {
        Self::RetrievalError(msg.into())
    }

    /// Create a vector error
    pub fn vector(msg: impl Into<String>) -> Self {
        Self::VectorError(msg.into())
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_) | Self::LockError(_) | Self::BackendUnavailable(_)
        )
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

/// Result type for memory operations
pub type MemoryResult<T> = Result<T, MemoryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = MemoryError::not_found("test-id");
        assert!(matches!(err, MemoryError::NotFound(_)));
        assert_eq!(err.to_string(), "Memory entry not found: test-id");
    }

    #[test]
    fn test_storage_error() {
        let err = MemoryError::storage("disk full");
        assert!(matches!(err, MemoryError::StorageError(_)));
        assert_eq!(err.to_string(), "Storage error: disk full");
    }

    #[test]
    fn test_embedding_error() {
        let err = MemoryError::embedding("API unavailable");
        assert!(matches!(err, MemoryError::EmbeddingError(_)));
    }

    #[test]
    fn test_config_error() {
        let err = MemoryError::config("invalid backend");
        assert!(matches!(err, MemoryError::ConfigError(_)));
    }

    #[test]
    fn test_capacity_exceeded() {
        let err = MemoryError::capacity_exceeded("max 1000 entries");
        assert!(matches!(err, MemoryError::CapacityExceeded { .. }));
    }

    #[test]
    fn test_is_recoverable() {
        assert!(MemoryError::Timeout("timed out".to_string()).is_recoverable());
        assert!(MemoryError::LockError("lock held".to_string()).is_recoverable());
        assert!(MemoryError::BackendUnavailable("db down".to_string()).is_recoverable());
        assert!(!MemoryError::NotFound("id".to_string()).is_recoverable());
        assert!(!MemoryError::ConfigError("bad config".to_string()).is_recoverable());
    }

    #[test]
    fn test_from_serde_error() {
        let json_err = serde_json::from_str::<String>("invalid").unwrap_err();
        let mem_err: MemoryError = json_err.into();
        assert!(matches!(mem_err, MemoryError::SerializationError(_)));
    }

    #[test]
    fn test_consolidation_error() {
        let err = MemoryError::consolidation("failed to merge");
        assert!(matches!(err, MemoryError::ConsolidationError(_)));
    }

    #[test]
    fn test_retrieval_error() {
        let err = MemoryError::retrieval("search failed");
        assert!(matches!(err, MemoryError::RetrievalError(_)));
    }

    #[test]
    fn test_vector_error() {
        let err = MemoryError::vector("dimension mismatch");
        assert!(matches!(err, MemoryError::VectorError(_)));
    }
}
