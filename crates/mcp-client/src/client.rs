use rmcp::{
    model::{
        CallToolRequest, CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo,
        ClientRequest, GetPromptRequest, GetPromptRequestParam, GetPromptResult, Implementation,
        InitializeResult, ListPromptsRequest, ListPromptsResult, ListResourcesRequest,
        ListResourcesResult, ListToolsRequest, ListToolsResult, LoggingMessageNotification,
        LoggingMessageNotificationMethod, PaginatedRequestParam, ProgressNotification,
        ProgressNotificationMethod, ProtocolVersion, ReadResourceRequest, ReadResourceRequestParam,
        ReadResourceResult, ServerNotification, ServerResult,
    },
    service::{ClientInitializeError, PeerRequestOptions, RunningService},
    transport::IntoTransport,
    ClientHandler, RoleClient, ServiceError, ServiceExt,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};

pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

pub type Error = rmcp::ServiceError;

#[async_trait::async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn list_resources(
        &self,
        next_cursor: Option<String>,
    ) -> Result<ListResourcesResult, Error>;

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error>;

    async fn list_tools(&self, next_cursor: Option<String>) -> Result<ListToolsResult, Error>;

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error>;

    async fn list_prompts(&self, next_cursor: Option<String>) -> Result<ListPromptsResult, Error>;

    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult, Error>;

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification>;

    fn get_info(&self) -> Option<&InitializeResult>;
}

pub struct GooseClient {
    notification_handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
}

impl GooseClient {
    pub fn new(handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>) -> Self {
        GooseClient {
            notification_handlers: handlers,
        }
    }
}

impl ClientHandler for GooseClient {
    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        context: rmcp::service::NotificationContext<rmcp::RoleClient>,
    ) -> () {
        self.notification_handlers
            .lock()
            .await
            .iter()
            .for_each(|handler| {
                let _ = handler.try_send(ServerNotification::ProgressNotification(
                    ProgressNotification {
                        params: params.clone(),
                        method: ProgressNotificationMethod,
                        extensions: context.extensions.clone(),
                    },
                ));
            });
    }

    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        context: rmcp::service::NotificationContext<rmcp::RoleClient>,
    ) -> () {
        self.notification_handlers
            .lock()
            .await
            .iter()
            .for_each(|handler| {
                let _ = handler.try_send(ServerNotification::LoggingMessageNotification(
                    LoggingMessageNotification {
                        params: params.clone(),
                        method: LoggingMessageNotificationMethod,
                        extensions: context.extensions.clone(),
                    },
                ));
            });
    }

    fn get_info(&self) -> ClientInfo {
        ClientInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ClientCapabilities::builder().build(),
            client_info: Implementation {
                name: "goose".to_string(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        }
    }
}

/// The MCP client is the interface for MCP operations.
pub struct McpClient {
    client: Mutex<RunningService<RoleClient, GooseClient>>,
    notification_subscribers: Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>>,
    server_info: Option<InitializeResult>,
    timeout: std::time::Duration,
}

impl McpClient {
    pub async fn connect<T, E, A>(
        transport: T,
        timeout: std::time::Duration,
    ) -> Result<Self, ClientInitializeError>
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
    {
        let notification_subscribers =
            Arc::new(Mutex::new(Vec::<mpsc::Sender<ServerNotification>>::new()));

        let client = GooseClient::new(notification_subscribers.clone());
        let client: rmcp::service::RunningService<rmcp::RoleClient, GooseClient> =
            client.serve(transport).await?;
        let server_info = client.peer_info().cloned();

        Ok(Self {
            client: Mutex::new(client),
            notification_subscribers,
            server_info,
            timeout,
        })
    }

    fn get_request_options(&self) -> PeerRequestOptions {
        PeerRequestOptions {
            timeout: Some(self.timeout),
            meta: None,
        }
    }
}

#[async_trait::async_trait]
impl McpClientTrait for McpClient {
    fn get_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    async fn list_resources(&self, cursor: Option<String>) -> Result<ListResourcesResult, Error> {
        let res = self
            .client
            .lock()
            .await
            .send_request_with_option(
                ClientRequest::ListResourcesRequest(ListResourcesRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: Default::default(),
                }),
                self.get_request_options(),
            )
            .await?
            .await_response()
            .await?;
        match res {
            ServerResult::ListResourcesResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        let res = self
            .client
            .lock()
            .await
            .send_request_with_option(
                ClientRequest::ReadResourceRequest(ReadResourceRequest {
                    params: ReadResourceRequestParam {
                        uri: uri.to_string(),
                    },
                    method: Default::default(),
                    extensions: Default::default(),
                }),
                self.get_request_options(),
            )
            .await?
            .await_response()
            .await?;
        match res {
            ServerResult::ReadResourceResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_tools(&self, cursor: Option<String>) -> Result<ListToolsResult, Error> {
        let res = self
            .client
            .lock()
            .await
            .send_request_with_option(
                ClientRequest::ListToolsRequest(ListToolsRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: Default::default(),
                }),
                self.get_request_options(),
            )
            .await?
            .await_response()
            .await?;
        match res {
            ServerResult::ListToolsResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        let res = self
            .client
            .lock()
            .await
            .send_request_with_option(
                ClientRequest::CallToolRequest(CallToolRequest {
                    params: CallToolRequestParam {
                        name: name.to_string().into(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: Default::default(),
                }),
                self.get_request_options(),
            )
            .await?
            .await_response()
            .await?;
        match res {
            ServerResult::CallToolResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_prompts(&self, cursor: Option<String>) -> Result<ListPromptsResult, Error> {
        let res = self
            .client
            .lock()
            .await
            .send_request_with_option(
                ClientRequest::ListPromptsRequest(ListPromptsRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: Default::default(),
                }),
                self.get_request_options(),
            )
            .await?
            .await_response()
            .await?;
        match res {
            ServerResult::ListPromptsResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult, Error> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        let res = self
            .client
            .lock()
            .await
            .send_request_with_option(
                ClientRequest::GetPromptRequest(GetPromptRequest {
                    params: GetPromptRequestParam {
                        name: name.to_string(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: Default::default(),
                }),
                self.get_request_options(),
            )
            .await?
            .await_response()
            .await?;
        match res {
            ServerResult::GetPromptResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        let (tx, rx) = mpsc::channel(16);
        self.notification_subscribers.lock().await.push(tx);
        rx
    }
}
