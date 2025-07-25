use mcp_core::protocol::{
    CallToolResult, Implementation, InitializeResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, ReadResourceResult, ServerCapabilities, METHOD_NOT_FOUND,
};
use rmcp::model::{
    GetPromptResult, JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest,
    JsonRpcResponse, JsonRpcVersion2_0, Notification, NumberOrString, Request, RequestId,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tower::{timeout::TimeoutLayer, Layer, Service, ServiceExt};

use crate::{McpService, TransportHandle};

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

    async fn list_prompts(&self, next_cursor: Option<String>) -> Result<ListPromptsResult, Error>;

    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult, Error>;

    async fn subscribe(&self) -> mpsc::Receiver<JsonRpcMessage>;
}

/// The MCP client is the interface for MCP operations.
pub struct McpClient<T>
where
    T: TransportHandle + Send + Sync + 'static,
{
    service: Mutex<tower::timeout::Timeout<McpService<T>>>,
    next_id_counter: AtomicU64, // Added for atomic ID generation
    server_capabilities: Option<ServerCapabilities>,
    server_info: Option<Implementation>,
    notification_subscribers: Arc<Mutex<Vec<mpsc::Sender<JsonRpcMessage>>>>,
}

impl<T> McpClient<T>
where
    T: TransportHandle + Send + Sync + 'static,
{
    pub async fn connect(transport: T, timeout: std::time::Duration) -> Result<Self, Error> {
        let service = McpService::new(transport.clone());
        let service_ptr = service.clone();
        let notification_subscribers =
            Arc::new(Mutex::new(Vec::<mpsc::Sender<JsonRpcMessage>>::new()));
        let subscribers_ptr = notification_subscribers.clone();

        tokio::spawn(async move {
            loop {
                match transport.receive().await {
                    Ok(message) => {
                        tracing::info!("Received message: {:?}", message);
                        match message {
                            JsonRpcMessage::Response(JsonRpcResponse {
                                id: NumberOrString::Number(id),
                                ..
                            })
                            | JsonRpcMessage::Error(JsonRpcError {
                                id: NumberOrString::Number(id),
                                ..
                            }) => {
                                service_ptr.respond(&id.to_string(), Ok(message)).await;
                            }
                            _ => {
                                let mut subs = subscribers_ptr.lock().await;
                                subs.retain(|sub| sub.try_send(message.clone()).is_ok());
                            }
                        }
                    }
                    Err(e) => {
                        service_ptr.hangup(e).await;
                        subscribers_ptr.lock().await.clear();
                        break;
                    }
                }
            }
        });

        let middleware = TimeoutLayer::new(timeout);

        Ok(Self {
            service: Mutex::new(middleware.layer(service)),
            next_id_counter: AtomicU64::new(1),
            server_capabilities: None,
            server_info: None,
            notification_subscribers,
        })
    }

    /// Send a JSON-RPC request and check we don't get an error response.
    async fn send_request<R>(&self, method: &str, params: Value) -> Result<R, Error>
    where
        R: for<'de> Deserialize<'de>,
    {
        let mut service = self.service.lock().await;
        service.ready().await.map_err(|_| Error::NotReady)?;
        let id_num = self.next_id_counter.fetch_add(1, Ordering::SeqCst);
        let id = RequestId::Number(id_num as u32);

        let mut params = params.clone();
        params["_meta"] = json!({
            "progressToken": format!("prog-{}", id),
        });

        let request = JsonRpcMessage::Request(JsonRpcRequest {
            jsonrpc: JsonRpcVersion2_0,
            id,
            request: Request {
                method: method.to_string(),
                params: params.as_object().unwrap().clone(),
                extensions: Default::default(),
            },
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
                source: Box::<Error>::new(e.into()),
            })?;

        match response_msg {
            JsonRpcMessage::Response(JsonRpcResponse { id, result, .. }) => {
                // Verify id matches - convert current id to match expected format
                let expected_id = RequestId::Number((id_num) as u32);
                if id != expected_id {
                    return Err(Error::UnexpectedResponse(
                        "id mismatch for JsonRpcResponse".to_string(),
                    ));
                }
                Ok(serde_json::from_value(serde_json::to_value(result)?)?)
            }
            JsonRpcMessage::Error(JsonRpcError { id, error, .. }) => {
                let expected_id = RequestId::Number((id_num) as u32);
                if id != expected_id {
                    return Err(Error::UnexpectedResponse(
                        "id mismatch for JsonRpcError".to_string(),
                    ));
                }
                Err(Error::RpcError {
                    code: error.code.0,                 // Extract the i32 from ErrorCode
                    message: error.message.to_string(), // Convert Cow to String
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
            jsonrpc: JsonRpcVersion2_0,
            notification: Notification {
                method: method.to_string(),
                params: params.as_object().unwrap().clone(),
                extensions: Default::default(),
            },
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
                source: Box::<Error>::new(e.into()),
            })?;

        Ok(())
    }

    // Check if the client has completed initialization
    fn completed_initialization(&self) -> bool {
        self.server_capabilities.is_some()
    }
}

#[async_trait::async_trait]
impl<T> McpClientTrait for McpClient<T>
where
    T: TransportHandle + Send + Sync + 'static,
{
    async fn initialize(
        &mut self,
        info: ClientInfo,
        capabilities: ClientCapabilities,
    ) -> Result<InitializeResult, Error> {
        let params = InitializeParams {
            protocol_version: "2025-03-26".to_string(),
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

    async fn list_prompts(&self, next_cursor: Option<String>) -> Result<ListPromptsResult, Error> {
        if !self.completed_initialization() {
            return Err(Error::NotInitialized);
        }

        // If prompts is not supported, return an error
        if self.server_capabilities.as_ref().unwrap().prompts.is_none() {
            return Err(Error::RpcError {
                code: METHOD_NOT_FOUND,
                message: "Server does not support 'prompts' capability".to_string(),
            });
        }

        let payload = next_cursor
            .map(|cursor| serde_json::json!({"cursor": cursor}))
            .unwrap_or_else(|| serde_json::json!({}));

        self.send_request("prompts/list", payload).await
    }

    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult, Error> {
        if !self.completed_initialization() {
            return Err(Error::NotInitialized);
        }

        // If prompts is not supported, return an error
        if self.server_capabilities.as_ref().unwrap().prompts.is_none() {
            return Err(Error::RpcError {
                code: METHOD_NOT_FOUND,
                message: "Server does not support 'prompts' capability".to_string(),
            });
        }

        let params = serde_json::json!({ "name": name, "arguments": arguments });

        self.send_request("prompts/get", params).await
    }

    async fn subscribe(&self) -> mpsc::Receiver<JsonRpcMessage> {
        let (tx, rx) = mpsc::channel(16);
        self.notification_subscribers.lock().await.push(tx);
        rx
    }
}
