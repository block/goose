//! Error types for goose2

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("Extension error: {0}")]
    Extension(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Completion error: {0}")]
    Completion(#[from] rig::completion::CompletionError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for sacp::Error {
    fn from(err: Error) -> Self {
        sacp::Error::internal_error().data(err.to_string())
    }
}
