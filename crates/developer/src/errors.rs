use mcp_core::handler::{ResourceError, ToolError};
use mcp_server::RouterError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum AgentError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("The parameters to the tool call were invalid: {0}")]
    InvalidParameters(String),

    #[error("The tool failed during execution with the following output: \n{0}")]
    ExecutionError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Invalid tool name: {0}")]
    InvalidToolName(String),
}

pub type AgentResult<T> = Result<T, AgentError>;

impl From<AgentError> for ToolError {
    fn from(err: AgentError) -> Self {
        match err {
            AgentError::InvalidParameters(msg) => ToolError::InvalidParameters(msg),
            AgentError::InvalidToolName(msg) => ToolError::InvalidParameters(msg),
            AgentError::ToolNotFound(msg) => ToolError::NotFound(msg),
            AgentError::ExecutionError(msg) => ToolError::ExecutionError(msg),
            AgentError::Internal(msg) => ToolError::ExecutionError(msg),
        }
    }
}

impl From<AgentError> for ResourceError {
    fn from(err: AgentError) -> Self {
        match err {
            AgentError::InvalidParameters(msg) => ResourceError::NotFound(msg),
            _ => ResourceError::NotFound(err.to_string()),
        }
    }
}

impl From<AgentError> for RouterError {
    fn from(err: AgentError) -> Self {
        match err {
            AgentError::ToolNotFound(msg) => RouterError::ToolNotFound(msg),
            AgentError::InvalidParameters(msg) => RouterError::InvalidParams(msg),
            AgentError::ExecutionError(msg) => RouterError::Internal(msg),
            AgentError::Internal(msg) => RouterError::Internal(msg),
            AgentError::InvalidToolName(msg) => RouterError::ToolNotFound(msg),
        }
    }
}

impl From<ResourceError> for AgentError {
    fn from(err: ResourceError) -> Self {
        match err {
            ResourceError::NotFound(msg) => {
                AgentError::InvalidParameters(format!("Resource not found: {}", msg))
            }
            ResourceError::ExecutionError(msg) => AgentError::ExecutionError(msg),
        }
    }
}
