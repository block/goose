use serde::{Deserialize, Serialize};
#[allow(unused_imports)] // this is used in schema below
use serde_json::{json, Value};
use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Schema error: {0}")]
    SchemaError(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

pub type ToolResult<T> = std::result::Result<T, ToolError>;

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
}

#[derive(Error, Debug)]
pub enum PromptError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Prompt not found: {0}")]
    NotFound(String),
}

/// Helper function to require a string, returning a ToolError
pub fn require_str_parameter<'a>(
    v: &'a serde_json::Value,
    name: &str,
) -> Result<&'a str, ToolError> {
    let v = v
        .get(name)
        .ok_or_else(|| ToolError::InvalidParameters(format!("The parameter {name} is required")))?;
    match v.as_str() {
        Some(r) => Ok(r),
        None => Err(ToolError::InvalidParameters(format!(
            "The parameter {name} must be a string"
        ))),
    }
}

/// Helper function to require a u64, returning a ToolError
pub fn require_u64_parameter(v: &serde_json::Value, name: &str) -> Result<u64, ToolError> {
    let v = v
        .get(name)
        .ok_or_else(|| ToolError::InvalidParameters(format!("The parameter {name} is required")))?;
    match v.as_u64() {
        Some(r) => Ok(r),
        None => Err(ToolError::InvalidParameters(format!(
            "The parameter {name} must be a number"
        ))),
    }
}
