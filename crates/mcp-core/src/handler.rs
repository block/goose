use async_trait::async_trait;
use schemars::JsonSchema;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Schema error: {0}")]
    SchemaError(String),
    #[error("Tool not found: {0}")]
    NotFound(String),
}

#[derive(Error, Debug)]
pub enum ResourceError {
    #[error("Execution failed: {0}")]
    ExecutionError(String),
    #[error("Resource not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, ToolError>;

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
    async fn call(&self, params: Value) -> Result<Value>;
}

/// Trait for implementing MCP resources
#[async_trait]
pub trait ResourceTemplateHandler: Send + Sync + 'static {
    /// The URL template for this resource
    fn template() -> &'static str;

    /// JSON schema describing the resource parameters
    fn schema() -> Value;

    /// Get the resource value
    async fn get(&self, params: Value) -> Result<String>;
}

/// Helper function to generate JSON schema for a type
pub fn generate_schema<T: JsonSchema>() -> Result<Value> {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(schema).map_err(|e| ToolError::SchemaError(e.to_string()))
}
