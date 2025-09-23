pub use rmcp::model::ErrorData;

/// Type alias for tool results
pub type ToolResult<T> = std::result::Result<T, ErrorData>;
