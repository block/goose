//! ACP extension request/response wire types and error converters.

use sacp::schema::McpServer;
use sacp::{JrRequest, JrResponsePayload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JrRequest)]
#[request(method = "_goose/session/new_with_recipe", response = NewWithRecipeResponse)]
pub struct NewWithRecipeRequest {
    pub cwd: String,
    pub recipe: serde_json::Value,
    #[serde(default)]
    pub mcp_servers: Vec<McpServer>,
}

#[derive(Debug, Serialize, Deserialize, JrResponsePayload)]
pub struct NewWithRecipeResponse {
    pub session_id: String,
    pub prompt: Option<String>,
    pub max_turns: Option<usize>,
    pub model_state: serde_json::Value,
}

// Client-side only: server-side CallToolRequest uses JsonSchema (for OpenAPI); this uses JrRequest (for sacp).
#[derive(Debug, Clone, Serialize, Deserialize, JrRequest)]
#[request(method = "_goose/tool/call", response = CallToolResponse)]
pub struct CallToolRequest {
    pub session_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub arguments: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, JrResponsePayload)]
pub struct CallToolResponse {
    pub content: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JrRequest)]
#[request(method = "_goose/session/get", response = GetSessionResponse)]
pub struct GetSessionRequest {
    pub session_id: String,
    #[serde(default)]
    pub include_messages: bool,
}

#[derive(Debug, Serialize, Deserialize, JrResponsePayload)]
pub struct GetSessionResponse {
    pub session: serde_json::Value,
}

pub(crate) fn to_sacp_error(e: impl std::fmt::Display) -> sacp::Error {
    sacp::Error::internal_error().data(e.to_string())
}

pub(crate) fn sacp_error_to_anyhow(e: sacp::Error) -> anyhow::Error {
    anyhow::anyhow!("{e:?}")
}
