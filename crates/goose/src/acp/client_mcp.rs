//! Tools an ACP client serves over its own connection, following the
//! MCP-over-ACP draft (`mcp/message`). Only `tools/list` and `tools/call`
//! are forwarded; every request is independent, with no MCP handshake.

use crate::agents::mcp_client::{Error as McpError, McpClientTrait};
use crate::agents::ToolCallContext;
use agent_client_protocol::{Client, ConnectionTo, UntypedMessage};
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, ErrorCode, ErrorData, InitializeResult, JsonObject, ListToolsResult,
    ProtocolVersion,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

const MCP_MESSAGE: &str = "mcp/message";

pub(crate) struct AcpClientMcp {
    cx: ConnectionTo<Client>,
    server_id: String,
}

impl AcpClientMcp {
    pub(crate) fn new(cx: ConnectionTo<Client>, server_id: String) -> Self {
        Self { cx, server_id }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        session_id: &str,
        method: &str,
        mut params: JsonObject,
        cancel_token: CancellationToken,
    ) -> Result<T, McpError> {
        params.insert(
            "_meta".to_string(),
            json!({
                "io.modelcontextprotocol/protocolVersion": ProtocolVersion::V_2026_07_28,
                "io.modelcontextprotocol/clientCapabilities": {},
                "io.modelcontextprotocol/clientInfo": {
                    "name": "goose",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        );
        let envelope = json!({
            "serverId": self.server_id,
            "requestId": uuid::Uuid::now_v7().to_string(),
            "method": method,
            "params": params,
            "_meta": { "sessionId": session_id },
        });
        let message = UntypedMessage::new(MCP_MESSAGE, envelope)
            .map_err(|error| transport_error(error.to_string()))?;

        let response = tokio::select! {
            response = self.cx.send_request(message).block_task() => response,
            () = cancel_token.cancelled() => {
                return Err(McpError::Cancelled { reason: None });
            }
        }
        .map_err(|error| transport_error(error.to_string()))?;

        parse_carrier(response)
    }
}

fn transport_error(message: String) -> McpError {
    McpError::McpError(ErrorData::new(ErrorCode::INTERNAL_ERROR, message, None))
}

/// The response carries exactly one inner MCP outcome, `result` or `error`.
fn parse_carrier<T: DeserializeOwned>(mut response: Value) -> Result<T, McpError> {
    if let Some(result) = response.get_mut("result").map(Value::take) {
        return serde_json::from_value(result)
            .map_err(|error| transport_error(format!("invalid MCP result: {error}")));
    }
    if let Some(error) = response.get_mut("error").map(Value::take) {
        let error: ErrorData = serde_json::from_value(error)
            .map_err(|error| transport_error(format!("invalid MCP error: {error}")))?;
        return Err(McpError::McpError(error));
    }
    Err(transport_error(
        "mcp/message response has neither result nor error".to_string(),
    ))
}

#[async_trait]
impl McpClientTrait for AcpClientMcp {
    async fn list_tools(
        &self,
        session_id: &str,
        next_cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, McpError> {
        let mut params = JsonObject::new();
        if let Some(cursor) = next_cursor {
            params.insert("cursor".to_string(), Value::String(cursor));
        }
        self.request(session_id, "tools/list", params, cancel_token)
            .await
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        let mut params = JsonObject::new();
        params.insert("name".to_string(), Value::String(name.to_string()));
        params.insert(
            "arguments".to_string(),
            Value::Object(arguments.unwrap_or_default()),
        );
        self.request(&ctx.session_id, "tools/call", params, cancel_token)
            .await
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carrier_result_is_unwrapped() {
        let result: CallToolResult = parse_carrier(json!({
            "result": {
                "resultType": "complete",
                "content": [{ "type": "text", "text": "hello" }],
                "isError": false
            }
        }))
        .unwrap();
        assert_eq!(result.is_error, Some(false));
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn carrier_error_stays_an_mcp_error() {
        let error = parse_carrier::<CallToolResult>(json!({
            "error": { "code": -32602, "message": "Unknown tool", "data": null }
        }))
        .unwrap_err();
        match error {
            McpError::McpError(data) => {
                assert_eq!(data.code, ErrorCode::INVALID_PARAMS);
                assert_eq!(data.message, "Unknown tool");
            }
            other => panic!("expected an MCP error, got {other:?}"),
        }
    }

    #[test]
    fn empty_carrier_is_rejected() {
        assert!(parse_carrier::<CallToolResult>(json!({})).is_err());
    }
}
