use anyhow::Result as AnyhowResult;
use async_trait::async_trait;

use crate::errors::AgentResult;
use mcp_core::{Content, Resource, Tool, ToolCall};

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
    async fn status(&self) -> AnyhowResult<Vec<Resource>>;

    /// Call a tool with the given parameters
    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>>;

    /// Read a resource from a URI. Each system should implement this to handle its own
    /// resource types appropriately. The URI scheme should indicate how to handle the resource.
    /// Common schemes include:
    /// - file:///path/to/file - A file on disk
    /// - str:///content - Direct string content
    /// - http(s)://url - Web content (if supported by the system)
    ///
    /// # Arguments
    /// * `uri` - The URI of the resource to read
    ///
    /// # Returns
    /// The content of the resource as a string. Binary content should be base64 encoded.
    async fn read_resource(&self, uri: &str) -> AgentResult<String>;
}
