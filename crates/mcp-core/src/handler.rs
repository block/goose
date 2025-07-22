use serde::{Deserialize, Serialize};
#[allow(unused_imports)] // this is used in schema below
use serde_json::json;
use thiserror::Error;
use utoipa::ToSchema;

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

// Define schema manually without generics issues
#[derive(ToSchema)]
#[schema(example = json!({"success": true, "data": {}}))]
pub struct ToolResultSchema {
    #[schema(example = "Operation completed successfully")]
    pub message: Option<String>,
    #[schema(example = true)]
    pub success: bool,
    #[schema(value_type = Object)]
    pub data: Option<serde_json::Value>,
}

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
