/// Helper utilities for MCP operations
use rmcp::model::{ErrorCode, ErrorData};

/// Type alias for tool results - matches the old mcp-core::ToolResult
pub type ToolResult<T> = std::result::Result<T, ErrorData>;

/// Helper function to require a string parameter, returning an ErrorData
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

/// Helper function to require a u64 parameter, returning an ErrorData
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
            format!("The parameter {name} must be a number"),
            None,
        )),
    }
}

/// Wrapper for ToolCall to match old mcp-core interface
/// Maps to rmcp's CallToolRequestParam
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    /// The name of the tool to execute
    pub name: String,
    /// The parameters for the execution
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Create a new ToolCall with the given name and parameters
    pub fn new<S: Into<String>>(name: S, arguments: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}
