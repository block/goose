use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)] // this is used in schema below
use serde_json::json;
use serde_json::Value;
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

/// Trait for implementing MCP tools
#[async_trait]
pub trait ToolHandler: Send + Sync + 'static {
    /// The name of the tool
    fn name(&self) -> &'static str;

    /// A description of what the tool does
    fn description(&self) -> &'static str;

    /// JSON schema describing the tool's parameters
    fn schema(&self) -> Value;

    /// Execute the tool with the given parameters
    async fn call(&self, params: Value) -> ToolResult<Value>;
}

/// Trait for implementing MCP resources
#[async_trait]
pub trait ResourceTemplateHandler: Send + Sync + 'static {
    /// The URL template for this resource
    fn template() -> &'static str;

    /// JSON schema describing the resource parameters
    fn schema() -> Value;

    /// Get the resource value
    async fn get(&self, params: Value) -> ToolResult<String>;
}

/// Helper function to generate JSON schema for a type
pub fn generate_schema<T: JsonSchema>() -> ToolResult<Value> {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(schema).map_err(|e| ToolError::SchemaError(e.to_string()))
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
