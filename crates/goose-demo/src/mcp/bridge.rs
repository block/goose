//! Bridge between MCP tools and rig's tool system
//!
//! This module converts MCP Tool definitions to rig's ToolDefinition format
//! so they can be included in completion requests.

use rig::completion::ToolDefinition;
use rmcp::model::Tool as McpTool;

/// Converts MCP tools to rig tool definitions
pub struct McpToolBridge;

impl McpToolBridge {
    /// Convert an MCP Tool to a rig ToolDefinition
    pub fn to_rig_tool(mcp_tool: &McpTool) -> ToolDefinition {
        ToolDefinition {
            name: mcp_tool.name.to_string(),
            description: mcp_tool
                .description
                .as_deref()
                .unwrap_or("")
                .to_string(),
            parameters: mcp_tool.schema_as_json_value(),
        }
    }

    /// Convert multiple MCP tools to rig tool definitions
    pub fn to_rig_tools(mcp_tools: &[McpTool]) -> Vec<ToolDefinition> {
        mcp_tools.iter().map(Self::to_rig_tool).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;
    use std::sync::Arc;

    #[test]
    fn test_tool_conversion() {
        let mcp_tool = Tool::new(
            "test_tool",
            "A test tool",
            Arc::new(serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string" }
                }
            }).as_object().unwrap().clone()),
        );

        let rig_tool = McpToolBridge::to_rig_tool(&mcp_tool);

        assert_eq!(rig_tool.name, "test_tool");
        assert_eq!(rig_tool.description, "A test tool");
    }
}
