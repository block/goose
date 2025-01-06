use std::sync::atomic::{AtomicU64, Ordering};

use mcp_core::protocol::{
    CallToolResult, InitializeResult, JsonRpcError, JsonRpcMessage, JsonRpcNotification,
    JsonRpcRequest, JsonRpcResponse, ListResourcesResult, ListToolsResult, ReadResourceResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;
use tower::ServiceExt; // for Service::ready()

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
pub trait McpClient: Send + Sync + 'static {
    /// Initialize the connection with the server.
    async fn initialize(
        &self,
        info: ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error>;

    /// List available resources.
    async fn list_resources(&self) -> Result<ListResourcesResult, Error>;

    /// Read a resource's content.
    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error>;

    /// List available tools.
    async fn list_tools(&self) -> Result<ListToolsResult, Error>;

    /// Call a specific tool with arguments.
    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error>;
}

/// Standard implementation of the MCP client that sends requests via the provided service.
pub struct McpClientImpl<S> {
    service: Mutex<S>,
    next_id: AtomicU64,
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
            service: Mutex::new(service),
            next_id: AtomicU64::new(1),
        }
    }

    /// Send a JSON-RPC request and check we don't get an error response.
    async fn send_request<R>(&self, method: &str, params: Value) -> Result<R, Error>
    where
        R: for<'de> Deserialize<'de>,
    {
        let mut service = self.service.lock().await;
        service.ready().await.map_err(|_| Error::NotReady)?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: method.to_string(),
            params: Some(params),
        });

        let response_msg = service.call(request).await?;

        match response_msg {
            JsonRpcMessage::Response(JsonRpcResponse {
                id, result, error, ..
            }) => {
                // Verify id matches
                if id != Some(self.next_id.load(Ordering::SeqCst) - 1) {
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
                if id != Some(self.next_id.load(Ordering::SeqCst) - 1) {
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
    async fn send_notification(&self, method: &str, params: Value) -> Result<(), Error> {
        let mut service = self.service.lock().await;
        service.ready().await.map_err(|_| Error::NotReady)?;

        let notification = JsonRpcMessage::Notification(JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(params),
        });

        service.call(notification).await?;
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
        + Sync
        + 'static,
    S::Future: Send,
{
    async fn initialize(
        &self,
        info: ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error> {
        let params = InitializeParams {
            protocol_version: "1.0.0".into(),
            client_info: info,
            capabilities,
        };
        let result: InitializeResult = self
            .send_request("initialize", serde_json::to_value(params)?)
            .await?;

        self.send_notification("notifications/initialized", serde_json::json!({}))
            .await?;

        Ok(result)
    }

    async fn list_resources(&self) -> Result<ListResourcesResult, Error> {
        self.send_request("resources/list", serde_json::json!({}))
            .await
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        let params = serde_json::json!({ "uri": uri });
        self.send_request("resources/read", params).await
    }

    async fn list_tools(&self) -> Result<ListToolsResult, Error> {
        self.send_request("tools/list", serde_json::json!({})).await
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error> {
        let params = serde_json::json!({ "name": name, "arguments": arguments });
        self.send_request("tools/call", params).await
    }
}
