use crate::providers::base::Provider;
use rmcp::model::JsonRpcMessage;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Configuration for task execution with all necessary dependencies
#[derive(Clone)]
pub struct TaskConfig {
    pub id: String,
    pub provider: Option<Arc<dyn Provider>>,
    pub mcp_tx: mpsc::Sender<JsonRpcMessage>,
    pub max_turns: Option<usize>,
}

impl fmt::Debug for TaskConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskConfig")
            .field("id", &self.id)
            .field("provider", &"<dyn Provider>")
            .field("max_turns", &self.max_turns)
            .finish()
    }
}

impl TaskConfig {
    /// Create a new TaskConfig with all required dependencies
    pub fn new(provider: Option<Arc<dyn Provider>>, mcp_tx: mpsc::Sender<JsonRpcMessage>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            provider,
            mcp_tx,
            max_turns: Some(10),
        }
    }

    /// Get a reference to the provider
    pub fn provider(&self) -> Option<&Arc<dyn Provider>> {
        self.provider.as_ref()
    }

    /// Get a clone of the MCP sender
    pub fn mcp_tx(&self) -> mpsc::Sender<JsonRpcMessage> {
        self.mcp_tx.clone()
    }
}
