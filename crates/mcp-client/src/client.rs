use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tower::ServiceExt; // for Service::ready()

use mcp_core::protocol::{
    CallToolResult, InitializeResult, JsonRpcError, JsonRpcMessage, JsonRpcNotification,
    JsonRpcRequest, JsonRpcResponse, ListResourcesResult, ListToolsResult, ReadResourceResult,
};

/// Error type for MCP client operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Service error: {0}")]
    Service(#[from] super::service::ServiceError),

    #[error("RPC error: code={code}, message={message}")]
    RpcError { code: i32, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unexpected response from server")]
    UnexpectedResponse,

    #[error("Timeout or service not ready")]
    NotReady,
}

#[derive(Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    // Add fields as needed. For now, empty capabilities are fine.
}

#[derive(Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// The MCP client trait defining the interface for MCP operations.
#[async_trait::async_trait]
pub trait McpClient {
    /// Initialize the connection with the server.
    async fn initialize(
        &mut self,
        info: ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error>;

    /// List available resources.
    async fn list_resources(&mut self) -> Result<ListResourcesResult, Error>;

    /// Read a resource's content.
    async fn read_resource(&mut self, uri: &str) -> Result<ReadResourceResult, Error>;

    /// List available tools.
    async fn list_tools(&mut self) -> Result<ListToolsResult, Error>;

    /// Call a specific tool with arguments.
    async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<CallToolResult, Error>;
}

/// Standard implementation of the MCP client that sends requests via the provided service.
pub struct McpClientImpl<S> {
    service: S,
    next_id: u64,
}

impl<S> McpClientImpl<S>
where
    S: tower::Service<
            JsonRpcMessage,
            Response = JsonRpcMessage,
            Error = super::service::ServiceError,
        > + Send,
    S::Future: Send,
{
    pub fn new(service: S) -> Self {
        Self {
            service,
            next_id: 1,
        }
    }

    /// Send a JSON-RPC request and wait for a response.
    async fn send_message<R>(&mut self, method: &str, params: Value) -> Result<R, Error>
    where
        R: for<'de> Deserialize<'de>,
    {
        self.service.ready().await.map_err(|_| Error::NotReady)?;

        let request = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(self.next_id),
            method: method.to_string(),
            params: Some(params),
        });

        self.next_id += 1;

        let response_msg = self.service.call(request).await?;

        match response_msg {
            JsonRpcMessage::Response(JsonRpcResponse {
                id, result, error, ..
            }) => {
                // Verify id matches
                if id != Some(self.next_id - 1) {
                    return Err(Error::UnexpectedResponse);
                }
                if let Some(err) = error {
                    Err(Error::RpcError {
                        code: err.code,
                        message: err.message,
                    })
                } else if let Some(r) = result {
                    Ok(serde_json::from_value(r)?)
                } else {
                    Err(Error::UnexpectedResponse)
                }
            }
            JsonRpcMessage::Error(JsonRpcError { id, error, .. }) => {
                if id != Some(self.next_id - 1) {
                    return Err(Error::UnexpectedResponse);
                }
                Err(Error::RpcError {
                    code: error.code,
                    message: error.message,
                })
            }
            _ => {
                // Requests/notifications not expected as a response
                Err(Error::UnexpectedResponse)
            }
        }
    }

    /// Send a JSON-RPC notification.
    async fn send_notification(&mut self, method: &str, params: Value) -> Result<(), Error> {
        self.service.ready().await.map_err(|_| Error::NotReady)?;

        let notification = JsonRpcMessage::Notification(JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(params),
        });

        self.service.call(notification).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<S> McpClient for McpClientImpl<S>
where
    S: tower::Service<
            JsonRpcMessage,
            Response = JsonRpcMessage,
            Error = super::service::ServiceError,
        > + Send
        + Sync,
    S::Future: Send,
{
    async fn initialize(
        &mut self,
        info: ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error> {
        let params = InitializeParams {
            protocol_version: "1.0.0".into(),
            client_info: info,
            capabilities,
        };
        let result: InitializeResult = self
            .send_message("initialize", serde_json::to_value(params)?)
            .await?;

        self.send_notification("notifications/initialized", serde_json::json!({}))
            .await?;

        Ok(result)
    }

    async fn list_resources(&mut self) -> Result<ListResourcesResult, Error> {
        self.send_message("resources/list", serde_json::json!({}))
            .await
    }

    async fn read_resource(&mut self, uri: &str) -> Result<ReadResourceResult, Error> {
        let params = serde_json::json!({ "uri": uri });
        self.send_message("resources/read", params).await
    }

    async fn list_tools(&mut self) -> Result<ListToolsResult, Error> {
        self.send_message("tools/list", serde_json::json!({})).await
    }

    async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<CallToolResult, Error> {
        let params = serde_json::json!({ "name": name, "arguments": arguments });
        self.send_message("tools/call", params).await
    }
}
