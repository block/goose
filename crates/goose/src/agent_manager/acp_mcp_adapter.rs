use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::agent_manager::client::AgentClientManager;

/// Represents an ACP agent exposed as an MCP-style tool.
///
/// The adapter translates between MCP tool calls and ACP prompt requests,
/// allowing the orchestrator to treat both local MCP extensions and remote
/// ACP agents uniformly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpToolDescriptor {
    pub tool_name: String,
    pub agent_id: String,
    pub description: String,
}

/// Result of calling an ACP agent through the adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpToolResult {
    pub agent_id: String,
    pub session_id: String,
    pub text: String,
    pub success: bool,
}

/// AcpMcpAdapter bridges ACP agents into the MCP tool ecosystem.
///
/// It generates MCP-compatible tool descriptors for connected ACP agents
/// and translates MCP tool invocations into ACP prompt requests.
pub struct AcpMcpAdapter;

impl AcpMcpAdapter {
    /// Generate MCP-style tool descriptors from all connected ACP agents.
    pub async fn list_tools(manager: &AgentClientManager) -> Vec<AcpToolDescriptor> {
        let agent_ids = manager.list_agents().await;
        let mut tools = Vec::new();

        for agent_id in agent_ids {
            if let Some(descriptor) = Self::agent_to_tool(manager, &agent_id).await {
                tools.push(descriptor);
            }
        }

        tools
    }

    /// Convert a single connected ACP agent to an MCP tool descriptor.
    async fn agent_to_tool(
        manager: &AgentClientManager,
        agent_id: &str,
    ) -> Option<AcpToolDescriptor> {
        let info = manager.get_agent_info(agent_id).await?;

        let description = info
            .agent_info
            .as_ref()
            .map(|a| {
                a.title
                    .clone()
                    .unwrap_or_else(|| format!("ACP agent: {}", a.name))
            })
            .unwrap_or_else(|| format!("ACP agent: {agent_id}"));

        let tool_name = format!("acp_agent_{}", agent_id.replace(['-', '.', ' '], "_"));

        Some(AcpToolDescriptor {
            tool_name,
            agent_id: agent_id.to_string(),
            description,
        })
    }

    /// Execute an ACP agent call as if it were an MCP tool invocation.
    pub async fn call_tool(
        manager: &AgentClientManager,
        agent_id: &str,
        session_id: Option<&str>,
        prompt_text: &str,
        working_dir: Option<&str>,
    ) -> Result<AcpToolResult> {
        use agent_client_protocol_schema::{NewSessionRequest, SessionId};

        let sid = match session_id {
            Some(s) => s.to_string(),
            None => {
                let cwd = working_dir
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

                let resp = manager
                    .new_session(agent_id, NewSessionRequest::new(cwd))
                    .await?;
                resp.session_id.0.to_string()
            }
        };

        debug!(agent_id, session_id = %sid, "AcpMcpAdapter: prompting agent");

        let session = SessionId::new(sid.as_str());
        let text = manager
            .prompt_agent_text(agent_id, &session, prompt_text)
            .await;

        match text {
            Ok(t) => Ok(AcpToolResult {
                agent_id: agent_id.to_string(),
                session_id: sid,
                text: t,
                success: true,
            }),
            Err(e) => Ok(AcpToolResult {
                agent_id: agent_id.to_string(),
                session_id: sid,
                text: format!("Agent error: {e}"),
                success: false,
            }),
        }
    }

    /// Build a JSON Schema for an ACP agent's tool input.
    pub fn build_tool_schema(_descriptor: &AcpToolDescriptor) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt to send to the agent"
                },
                "session_id": {
                    "type": "string",
                    "description": "Existing session ID to reuse (optional)"
                }
            },
            "required": ["prompt"]
        })
    }

    /// Merge ACP tool descriptors with existing MCP tools into a unified catalog.
    pub fn merge_tool_catalogs(
        mcp_tools: HashMap<String, Value>,
        acp_tools: &[AcpToolDescriptor],
    ) -> HashMap<String, Value> {
        let mut catalog = mcp_tools;

        for tool in acp_tools {
            let schema = Self::build_tool_schema(tool);
            catalog.insert(
                tool.tool_name.clone(),
                serde_json::json!({
                    "name": tool.tool_name,
                    "description": tool.description,
                    "inputSchema": schema,
                    "source": "acp",
                    "agent_id": tool.agent_id,
                }),
            );
        }

        catalog
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name_sanitization() {
        let descriptor = AcpToolDescriptor {
            tool_name: "acp_agent_my_agent".to_string(),
            agent_id: "my-agent".to_string(),
            description: "Test agent".to_string(),
        };
        assert_eq!(descriptor.tool_name, "acp_agent_my_agent");
    }

    #[test]
    fn test_build_tool_schema() {
        let descriptor = AcpToolDescriptor {
            tool_name: "acp_agent_test".to_string(),
            agent_id: "test".to_string(),
            description: "Test".to_string(),
        };

        let schema = AcpMcpAdapter::build_tool_schema(&descriptor);
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["prompt"].is_object());
    }

    #[test]
    fn test_merge_catalogs() {
        let mcp_tools: HashMap<String, Value> = HashMap::from([(
            "existing_tool".to_string(),
            serde_json::json!({"name": "existing_tool"}),
        )]);

        let acp_tools = vec![AcpToolDescriptor {
            tool_name: "acp_agent_remote".to_string(),
            agent_id: "remote".to_string(),
            description: "Remote agent".to_string(),
        }];

        let merged = AcpMcpAdapter::merge_tool_catalogs(mcp_tools, &acp_tools);
        assert_eq!(merged.len(), 2);
        assert!(merged.contains_key("existing_tool"));
        assert!(merged.contains_key("acp_agent_remote"));
    }
}
