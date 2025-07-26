use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo,
        GetPromptRequestParam, GetPromptResult, Implementation, InitializeResult,
        ListPromptsResult, ListResourcesResult, ListToolsResult, LoggingMessageNotification,
        LoggingMessageNotificationMethod, PaginatedRequestParam, ProgressNotification,
        ProgressNotificationMethod, ProtocolVersion, ReadResourceRequestParam, ReadResourceResult,
        ServerNotification,
    },
    service::{ClientInitializeError, RunningService},
    transport::IntoTransport,
    ClientHandler, RoleClient, ServiceExt,
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
}

impl McpClient {
    pub async fn connect<T, E, A>(
        transport: T,
        _timeout: std::time::Duration, // TODO
    ) -> Result<Self, ClientInitializeError<E>>
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
        })
    }
}

#[async_trait::async_trait]
impl McpClientTrait for McpClient {
    fn get_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    async fn list_resources(&self, cursor: Option<String>) -> Result<ListResourcesResult, Error> {
        self.client
            .lock()
            .await
            .list_resources(Some(PaginatedRequestParam { cursor }))
            .await
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, Error> {
        self.client
            .lock()
            .await
            .read_resource(ReadResourceRequestParam {
                uri: uri.to_string(),
            })
            .await
    }

    async fn list_tools(&self, cursor: Option<String>) -> Result<ListToolsResult, Error> {
        self.client
            .lock()
            .await
            .list_tools(Some(PaginatedRequestParam { cursor }))
            .await
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        self.client
            .lock()
            .await
            .call_tool(CallToolRequestParam {
                name: name.to_string().into(),
                arguments,
            })
            .await
    }

    async fn list_prompts(&self, cursor: Option<String>) -> Result<ListPromptsResult, Error> {
        self.client
            .lock()
            .await
            .list_prompts(Some(PaginatedRequestParam { cursor }))
            .await
    }

    async fn get_prompt(&self, name: &str, arguments: Value) -> Result<GetPromptResult, Error> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        self.client
            .lock()
            .await
            .get_prompt(GetPromptRequestParam {
                name: name.to_string(),
                arguments,
            })
            .await
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        let (tx, rx) = mpsc::channel(16);
        self.notification_subscribers.lock().await.push(tx);
        rx
    }
}
