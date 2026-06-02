use crate::conversation::tool_result_serde;
use crate::mcp_utils::ToolResult;
use rmcp::model::{CallToolRequestParams, CallToolResult, JsonObject};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema)]
pub enum ToolCallResult<T> {
    Success { value: T },
    Error { error: String },
}

/// Provider-specific metadata for tool requests/responses.
/// Allows providers to store custom data without polluting the core model.
pub type ProviderMetadata = serde_json::Map<String, serde_json::Value>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(ToSchema)]
pub struct ToolRequest {
    pub id: String,
    #[serde(with = "tool_result_serde")]
    #[schema(value_type = Object)]
    pub tool_call: ToolResult<CallToolRequestParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub metadata: Option<ProviderMetadata>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub tool_meta: Option<serde_json::Value>,
}

impl ToolRequest {
    pub fn to_readable_string(&self) -> String {
        match &self.tool_call {
            Ok(tool_call) => {
                format!(
                    "Tool: {}, Args: {}",
                    tool_call.name,
                    serde_json::to_string_pretty(&tool_call.arguments)
                        .unwrap_or_else(|_| "<<invalid json>>".to_string())
                )
            }
            Err(e) => format!("Invalid tool call: {}", e),
        }
    }

    /// Returns true if this tool request was already executed externally
    /// (e.g. by an ACP provider's underlying SDK) and the agent loop must
    /// not redispatch it. See [`TOOL_META_EXTERNAL_DISPATCH_KEY`].
    pub fn is_externally_dispatched(&self) -> bool {
        self.tool_meta
            .as_ref()
            .and_then(|v| v.get(TOOL_META_EXTERNAL_DISPATCH_KEY))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Returns the persisted LLM-generated title for this tool call, if any.
    /// Set asynchronously by [`crate::acp::server`] after `provider.complete_fast`
    /// resolves; survives session reload via SQLite. Falls back to `None` for
    /// older sessions that predate persistence — callers should use a deterministic
    /// title in that case.
    pub fn persisted_title(&self) -> Option<&str> {
        self.tool_meta
            .as_ref()
            .and_then(|v| v.get(TOOL_META_TITLE_KEY))
            .and_then(|v| v.as_str())
    }

    /// Returns the persisted per-chain summary anchored on this tool request,
    /// if any. Only the FIRST tool request in a chain (a run of consecutive
    /// tool blocks within one assistant message) carries this. See
    /// [`crate::acp::server`] for how chains are detected and summarized.
    pub fn persisted_chain_summary(&self) -> Option<PersistedChainSummary> {
        let obj = self
            .tool_meta
            .as_ref()
            .and_then(|v| v.get(TOOL_META_CHAIN_SUMMARY_KEY))?;
        let summary = obj.get("summary").and_then(|v| v.as_str())?.to_string();
        let count = obj.get("count").and_then(|v| v.as_u64())?;
        if count == 0 {
            return None;
        }
        Some(PersistedChainSummary {
            summary,
            count: count as usize,
        })
    }
}

/// A chain summary persisted on the first tool request of a chain.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistedChainSummary {
    pub summary: String,
    pub count: usize,
}

/// Marker key under `ToolRequest.tool_meta` indicating the tool was already
/// executed externally; the agent loop must skip redispatch.
pub const TOOL_META_EXTERNAL_DISPATCH_KEY: &str = "goose.external_dispatch";

/// Key under `ToolRequest.tool_meta` storing the LLM-generated short title
/// for this tool call. Used to make the title survive session reload.
pub const TOOL_META_TITLE_KEY: &str = "goose.toolSummary.title";

/// Key under `ToolRequest.tool_meta` storing the LLM-generated chain summary
/// for the chain that starts at this tool request. Shape: `{ "summary": String,
/// "count": u64 }`. Only attached to the FIRST tool request in a chain.
pub const TOOL_META_CHAIN_SUMMARY_KEY: &str = "goose.toolChain.summary";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(ToSchema)]
pub struct ToolResponse {
    pub id: String,
    #[serde(with = "tool_result_serde::call_tool_result")]
    #[schema(value_type = Object)]
    pub tool_result: ToolResult<CallToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub metadata: Option<ProviderMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(ToSchema)]
pub struct ToolConfirmationRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: JsonObject,
    pub prompt: Option<String>,
}
