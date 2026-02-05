//! MCP server connection management
//!
//! Each Session owns its MCP connections. Connections are established
//! when a session starts and dropped when the session ends.

use std::collections::HashMap;

use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, Tool},
    service::RunningService,
    transport::TokioChildProcess,
};
use tokio::process::Command;
use tracing::{instrument, info, error};

use crate::{Error, Result};

/// A connection to an MCP server
pub struct McpConnection {
    /// Name/identifier for this MCP server
    pub name: String,
    /// The running MCP client service
    service: RunningService<RoleClient, ()>,
}

impl McpConnection {
    /// Connect to an MCP server via stdio (command execution)
    #[instrument(fields(server_name = %name, command = %command))]
    pub async fn connect_stdio(
        name: &str,
        command: &str,
        args: &[&str],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        info!("Connecting to MCP server");

        let mut cmd = Command::new(command);
        cmd.args(args);

        // Add environment variables
        for (key, value) in env {
            cmd.env(key, value);
        }

        let transport = TokioChildProcess::new(cmd)
            .map_err(|e| Error::Mcp(format!("Failed to spawn MCP server '{}': {}", name, e)))?;

        let service = ().serve(transport)
            .await
            .map_err(|e| Error::Mcp(format!("Failed to connect to MCP server '{}': {}", name, e)))?;

        info!("Connected to MCP server");
        Ok(Self { 
            name: name.to_string(), 
            service,
        })
    }

    /// List all tools available from this MCP server
    #[instrument(skip(self), fields(server = %self.name))]
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let tools = self.service.peer()
            .list_all_tools()
            .await
            .map_err(|e| Error::Mcp(format!("Failed to list tools from '{}': {}", self.name, e)))?;
        
        info!(tool_count = tools.len(), "Listed tools from MCP server");
        Ok(tools)
    }

    /// Call a tool on this MCP server
    #[instrument(skip(self, arguments), fields(server = %self.name, tool = %tool_name))]
    pub async fn call_tool(&self, tool_name: &str, arguments: Option<serde_json::Map<String, serde_json::Value>>) -> Result<CallToolResult> {
        let params = CallToolRequestParams {
            name: tool_name.to_string().into(),
            arguments,
            meta: None,
            task: None,
        };

        let result = self.service.peer()
            .call_tool(params)
            .await
            .map_err(|e| {
                error!(error = %e, "Tool call failed");
                Error::Mcp(format!("Tool call '{}' failed on '{}': {}", tool_name, self.name, e))
            })?;

        info!(is_error = result.is_error.unwrap_or(false), "Tool call completed");
        Ok(result)
    }

    /// Get the server name
    pub fn name(&self) -> &str {
        &self.name
    }
}
