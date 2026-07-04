use crate::mcp_utils::ToolResult;
use crate::providers::base::Provider;
use rmcp::model::{CallToolResult, Tool};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Type alias for the tool result channel receiver
pub type ToolResultReceiver = Arc<Mutex<mpsc::Receiver<(String, ToolResult<CallToolResult>)>>>;

// We use double Arc here to allow easy provider swaps while sharing concurrent access
pub type SharedProvider = Arc<Mutex<Option<Arc<dyn Provider>>>>;

/// A frontend tool that will be executed by the frontend rather than an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendTool {
    pub name: String,
    pub tool: Tool,
}

/// Session configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Identifier of the underlying Session
    pub id: String,
    /// Maximum number of turns (iterations) allowed without user input
    pub max_turns: Option<u32>,
}
