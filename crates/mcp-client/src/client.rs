use mcp_core::protocol::{
    CallToolResult, Implementation, InitializeResult, JsonRpcError, JsonRpcMessage,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, ListResourcesResult, ListToolsResult,
    ReadResourceResult, ServerCapabilities, METHOD_NOT_FOUND,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use tokio::sync::Mutex;
use tower::{Service, ServiceExt}; // for Service::ready()

pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

/// Error type for MCP client operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Transport error: {0}")]
    Transport(#[from] super::transport::Error),

    #[error("RPC error: code={code}, message={message}")]
    RpcError { code: i32, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unexpected response from server: {0}")]
    UnexpectedResponse(String),

    #[error("Not initialized")]
    NotInitialized,

    #[error("Timeout or service not ready")]
    NotReady,

    #[error("Request timed out")]
    Timeout(#[from] tower::timeout::error::Elapsed),

    #[error("Error from mcp-server: {0}")]
    ServerBoxError(BoxError),

    #[error("Call to '{server}' failed for '{method}'. {source}")]
    McpServerError {
        method: String,
        server: String,
        #[source]
        source: BoxError,
    },
}

// BoxError from mcp-server gets converted to our Error type
impl From<BoxError> for Error {
    fn from(err: BoxError) -> Self {
        Error::ServerBoxError(err)
    }
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

#[async_trait::async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn initialize(
        &mut self,
        info: ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error>;

    async fn list_resources(
        &self,
        next_cursor: Option<String>,
    ) -> Result<ListResourcesResult, Error>;

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error>;

    async fn list_tools(&self, next_cursor: Option<String>) -> Result<ListToolsResult, Error>;

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error>;
}

/// The MCP client is the interface for MCP operations.
pub struct McpClient<S>
where
    S: Service<JsonRpcMessage, Response = JsonRpcMessage> + Clone + Send + Sync + 'static,
    S::Error: Into<Error>,
    S::Future: Send,
{
    service: Mutex<S>,
    next_id: AtomicU64,
    server_capabilities: Option<ServerCapabilities>,
    server_info: Option<Implementation>,
}

impl<S> McpClient<S>
where
    S: Service<JsonRpcMessage, Response = JsonRpcMessage> + Clone + Send + Sync + 'static,
    S::Error: Into<Error>,
    S::Future: Send,
{
    pub fn new(service: S) -> Self {
        Self {
            service: Mutex::new(service),
            next_id: AtomicU64::new(1),
            server_capabilities: None,
            server_info: None,
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
            params: Some(params.clone()),
        });

        let response_msg = service
            .call(request)
            .await
            .map_err(|e| Error::McpServerError {
                server: self
                    .server_info
                    .as_ref()
                    .map(|s| s.name.clone())
                    .unwrap_or("".to_string()),
                method: method.to_string(),
                // we don't need include params because it can be really large
                source: Box::new(e.into()),
            })?;

        match response_msg {
            JsonRpcMessage::Response(JsonRpcResponse {
                id, result, error, ..
            }) => {
                // Verify id matches
                if id != Some(self.next_id.load(Ordering::SeqCst) - 1) {
                    return Err(Error::UnexpectedResponse(
                        "id mismatch for JsonRpcResponse".to_string(),
                    ));
                }
                if let Some(err) = error {
                    Err(Error::RpcError {
                        code: err.code,
                        message: err.message,
                    })
                } else if let Some(r) = result {
                    Ok(serde_json::from_value(r)?)
                } else {
                    Err(Error::UnexpectedResponse("missing result".to_string()))
                }
            }
            JsonRpcMessage::Error(JsonRpcError { id, error, .. }) => {
                if id != Some(self.next_id.load(Ordering::SeqCst) - 1) {
                    return Err(Error::UnexpectedResponse(
                        "id mismatch for JsonRpcError".to_string(),
                    ));
                }
                Err(Error::RpcError {
                    code: error.code,
                    message: error.message,
                })
            }
            _ => {
                // Requests/notifications not expected as a response
                Err(Error::UnexpectedResponse(
                    "unexpected message type".to_string(),
                ))
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
            params: Some(params.clone()),
        });

        service
            .call(notification)
            .await
            .map_err(|e| Error::McpServerError {
                server: self
                    .server_info
                    .as_ref()
                    .map(|s| s.name.clone())
                    .unwrap_or("".to_string()),
                method: method.to_string(),
                // we don't need include params because it can be really large
                source: Box::new(e.into()),
            })?;

        Ok(())
    }

    // Check if the client has completed initialization
    fn completed_initialization(&self) -> bool {
        self.server_capabilities.is_some()
    }
}

#[async_trait::async_trait]
impl<S> McpClientTrait for McpClient<S>
where
    S: Service<JsonRpcMessage, Response = JsonRpcMessage> + Clone + Send + Sync + 'static,
    S::Error: Into<Error>,
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
            .send_request("initialize", serde_json::to_value(params)?)
            .await?;

        self.send_notification("notifications/initialized", serde_json::json!({}))
            .await?;

        self.server_capabilities = Some(result.capabilities.clone());

        self.server_info = Some(result.server_info.clone());

        Ok(result)
    }

    async fn list_resources(
        &self,
        next_cursor: Option<String>,
    ) -> Result<ListResourcesResult, Error> {
        if !self.completed_initialization() {
            return Err(Error::NotInitialized);
        }
        // If resources is not supported, return an empty list
        if self
            .server_capabilities
            .as_ref()
            .unwrap()
            .resources
            .is_none()
        {
            return Ok(ListResourcesResult {
                resources: vec![],
                next_cursor: None,
            });
        }

        let payload = next_cursor
            .map(|cursor| serde_json::json!({"cursor": cursor}))
            .unwrap_or_else(|| serde_json::json!({}));

        self.send_request("resources/list", payload).await
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        if !self.completed_initialization() {
            return Err(Error::NotInitialized);
        }
        // If resources is not supported, return an error
        if self
            .server_capabilities
            .as_ref()
            .unwrap()
            .resources
            .is_none()
        {
            return Err(Error::RpcError {
                code: METHOD_NOT_FOUND,
                message: "Server does not support 'resources' capability".to_string(),
            });
        }

        let params = serde_json::json!({ "uri": uri });
        self.send_request("resources/read", params).await
    }

    async fn list_tools(&self, next_cursor: Option<String>) -> Result<ListToolsResult, Error> {
        if !self.completed_initialization() {
            return Err(Error::NotInitialized);
        }
        // If tools is not supported, return an empty list
        if self.server_capabilities.as_ref().unwrap().tools.is_none() {
            return Ok(ListToolsResult {
                tools: vec![],
                next_cursor: None,
            });
        }

        let payload = next_cursor
            .map(|cursor| serde_json::json!({"cursor": cursor}))
            .unwrap_or_else(|| serde_json::json!({}));

        self.send_request("tools/list", payload).await
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error> {
        if !self.completed_initialization() {
            return Err(Error::NotInitialized);
        }
        // If tools is not supported, return an error
        if self.server_capabilities.as_ref().unwrap().tools.is_none() {
            return Err(Error::RpcError {
                code: METHOD_NOT_FOUND,
                message: "Server does not support 'tools' capability".to_string(),
            });
        }

        let params = serde_json::json!({ "name": name, "arguments": arguments });

        // TODO ERROR: check that if there is an error, we send back is_error: true with msg
        // https://modelcontextprotocol.io/docs/concepts/tools#error-handling-2
        self.send_request("tools/call", params).await
    }
}
