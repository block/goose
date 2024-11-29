use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::errors::AgentResult;
use crate::models::content::Content;
use crate::models::tool::{Tool, ToolCall};

/// Core trait that defines a system that can be operated by an AI agent
#[async_trait]
pub trait System: Send + Sync {
    /// Get the name of the system
    fn name(&self) -> &str;

    /// Get the system description
    fn description(&self) -> &str;

    /// Get system instructions
    fn instructions(&self) -> &str;

    /// Get available tools
    fn tools(&self) -> &[Tool];

    /// Get current system status
    async fn status(&self) -> AnyhowResult<HashMap<String, Value>>;

    /// Call a tool with the given parameters
    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>>;
}
