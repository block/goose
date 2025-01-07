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
