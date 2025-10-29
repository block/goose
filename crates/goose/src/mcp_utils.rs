pub use rmcp::model::ErrorData;

/// Type alias for tool results
pub type ToolResult<T> = Result<T, ErrorData>;
