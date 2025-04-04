use mcp_core::{Content, ToolResult};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Type alias for the tool result channel receiver
pub type ToolResultReceiver = Arc<Mutex<mpsc::Receiver<(String, ToolResult<Vec<Content>>)>>>;
