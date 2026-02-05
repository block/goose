//! MCP Extension - Wraps an MCP server as an Extension
//!
//! This bridges the MCP protocol to our Extension trait, allowing
//! MCP servers to be used seamlessly alongside native extensions.

use std::collections::HashMap;

use async_trait::async_trait;
use rig::completion::ToolDefinition;
use serde_json::{Map, Value};
use tracing::{info, instrument};

use super::Extension;
use crate::mcp::McpConnection;
use crate::{Error, Result};

/// An extension backed by an MCP server process.
pub struct McpExtension {
    /// Extension name
    name: String,

    /// Human-readable description
    description: String,

    /// The underlying MCP connection
    connection: McpConnection,
}

impl McpExtension {
    /// Connect to an MCP server and create an extension.
    #[instrument(fields(name = %name, command = %command))]
    pub async fn connect(
        name: &str,
        description: &str,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        info!("Connecting to MCP server for extension");

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let connection = McpConnection::connect_stdio(name, command, &args_refs, env).await?;

        Ok(Self {
            name: name.to_string(),
            description: description.to_string(),
            connection,
        })
    }

    /// Get the tool name with extension prefix
    fn prefixed_tool_name(&self, tool_name: &str) -> String {
        format!("{}__{}", self.name, tool_name)
    }

    /// Strip the extension prefix from a tool name
    fn strip_prefix<'a>(&self, full_name: &'a str) -> Option<&'a str> {
        let prefix = format!("{}__", self.name);
        full_name.strip_prefix(&prefix)
    }
}

#[async_trait]
impl Extension for McpExtension {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn instructions(&self) -> Option<&str> {
        // MCP servers don't provide instructions (yet)
        // Could potentially be added to MCP protocol in the future
        None
    }

    async fn list_tools(&self) -> Result<Vec<ToolDefinition>> {
        let mcp_tools = self.connection.list_tools().await?;

        // Convert MCP tools to rig ToolDefinitions with prefixed names
        let tools = mcp_tools
            .iter()
            .map(|t| ToolDefinition {
                name: self.prefixed_tool_name(&t.name),
                description: t.description.as_deref().unwrap_or("").to_string(),
                parameters: t.schema_as_json_value(),
            })
            .collect();

        Ok(tools)
    }

    async fn call_tool(&self, name: &str, arguments: Option<Map<String, Value>>) -> Result<String> {
        // Strip the extension prefix to get the actual MCP tool name
        let mcp_tool_name = self.strip_prefix(name).ok_or_else(|| {
            Error::Extension(format!(
                "Tool '{}' does not belong to extension '{}'",
                name, self.name
            ))
        })?;

        let result = self.connection.call_tool(mcp_tool_name, arguments).await?;

        // Convert MCP result to string
        let content = result
            .content
            .into_iter()
            .map(|c| serde_json::to_string(&c).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_tool_name_prefixing() {
        // Test the prefix/strip logic directly
        let name = "browser";
        let prefix = format!("{}__", name);
        
        // Prefixing
        let prefixed = format!("{}__{}", name, "navigate");
        assert_eq!(prefixed, "browser__navigate");
        
        // Stripping
        assert_eq!("browser__navigate".strip_prefix(&prefix), Some("navigate"));
        assert_eq!("other__navigate".strip_prefix(&prefix), None);
    }
}
