use rmcp::model::{ErrorCode, ErrorData};
use thiserror::Error;

pub type ToolResult<T> = std::result::Result<T, ErrorData>;

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

/// Helper function to require a string, returning an ErrorData
pub fn require_str_parameter<'a>(
    v: &'a serde_json::Value,
    name: &str,
) -> Result<&'a str, ErrorData> {
    let v = v.get(name).ok_or_else(|| {
        ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!("The parameter {name} is required"),
            None,
        )
    })?;
    match v.as_str() {
        Some(r) => Ok(r),
        None => Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!("The parameter {name} must be a string"),
            None,
        )),
    }
}

/// Helper function to require a u64, returning an ErrorData
pub fn require_u64_parameter(v: &serde_json::Value, name: &str) -> Result<u64, ErrorData> {
    let v = v.get(name).ok_or_else(|| {
        ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!("The parameter {name} is required"),
            None,
        )
    })?;
    match v.as_u64() {
        Some(r) => Ok(r),
        None => Err(ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            format!("The parameter {name} is required"),
            None,
        )),
    }
}
