use crate::action_required_manager::{ActionRequiredManager, ElicitationOutcome};
use crate::agents::tool_execution::ToolCallContext;
use crate::agents::types::SharedProvider;
use crate::session_context::{SESSION_ID_HEADER, TOOL_CALL_REQUEST_ID_HEADER, WORKING_DIR_HEADER};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use futures::StreamExt;
use rmcp::model::{
    CreateElicitationRequestParams, CreateElicitationResult, ElicitationAction, ErrorCode,
    ExtensionCapabilities, Extensions, JsonObject, ListRootsResult, LoggingMessageNotification,
    Meta, Root, SamplingMessageContent,
};
/// MCP client implementation for Goose
use rmcp::transport::AuthorizationManager;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, CancelledNotificationParam, ClientCapabilities,
        ClientInfo, ClientRequest, CreateMessageRequestParams, CreateMessageResult,
        GetPromptRequestParams, GetPromptResult, Implementation, InitializeRequestParams,
        InitializeResult, ListPromptsResult, ListResourcesResult, ListToolsResult, Notification,
        PaginatedRequestParams, ProtocolVersion, ReadResourceRequestParams, ReadResourceResult,
        Request, RequestId, RequestOptionalParam, Role, SamplingMessage, ServerNotification,
        ServerResult,
    },
    service::{
        ClientInitializeError, PeerRequestOptions, RequestContext, RequestHandle, RunningService,
        ServiceRole,
    },
    transport::IntoTransport,
    ClientHandler, ErrorData, Peer, RoleClient, ServiceError, ServiceExt,
};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{atomic::AtomicU64, atomic::Ordering, Arc, Mutex as StdMutex},
    time::Duration,
};
use tokio::sync::{
    mpsc::{self, Sender},
    oneshot, Mutex,
};
use tokio_util::sync::CancellationToken;

pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

pub type Error = rmcp::ServiceError;

const MCP_APPS_UI_EXTENSION_ID: &str = "io.modelcontextprotocol/ui";
const MCP_APPS_UI_MIME_TYPE: &str = "text/html;profile=mcp-app";
const MODERN_MCP_PROTOCOL_VERSION: &str = "2026-07-28";

struct ModernTaskSubscription {
    client: reqwest::Client,
    uri: String,
    client_name: String,
    client_version: String,
    notification_subscribers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
    auth_manager: Option<Arc<Mutex<AuthorizationManager>>>,
    request_id: AtomicU64,
    subscribed_workspaces: Mutex<HashSet<String>>,
}

impl ModernTaskSubscription {
    async fn probe(
        client: reqwest::Client,
        uri: String,
        client_name: String,
        client_version: String,
        notification_subscribers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
        auth_manager: Option<Arc<Mutex<AuthorizationManager>>>,
    ) -> Option<Arc<Self>> {
        let subscription = Arc::new(Self {
            client,
            uri,
            client_name,
            client_version,
            notification_subscribers,
            auth_manager,
            request_id: AtomicU64::new(1),
            subscribed_workspaces: Mutex::new(HashSet::new()),
        });
        let result: Value = subscription
            .post("server/discover", serde_json::json!({}), None, false)
            .await
            .ok()?
            .json()
            .await
            .ok()?;
        let result = result.get("result").unwrap_or(&result);
        (result
            .get("supportedVersions")
            .and_then(Value::as_array)
            .is_some_and(|versions| {
                versions
                    .iter()
                    .any(|version| version.as_str() == Some(MODERN_MCP_PROTOCOL_VERSION))
            })
            && result
                .pointer("/capabilities/resources/subscribe")
                .and_then(Value::as_bool)
                == Some(true))
        .then_some(subscription)
    }

    async fn ensure_workspace_subscription(&self, working_dir: &std::path::Path) {
        let workspace = working_dir.display().to_string();
        let mut workspaces = self.subscribed_workspaces.lock().await;
        if !workspaces.insert(workspace.clone()) {
            return;
        }
        drop(workspaces);

        let resource_uri = format!(
            "ferrosa-memory://tasks/workspaces/{}/active",
            URL_SAFE_NO_PAD.encode(workspace.as_bytes())
        );
        let response = self
            .post(
                "subscriptions/listen",
                serde_json::json!({
                    "notifications": {
                        "resourceSubscriptions": [resource_uri]
                    }
                }),
                None,
                true,
            )
            .await;

        let Ok(response) = response else {
            self.subscribed_workspaces.lock().await.remove(&workspace);
            return;
        };

        let (ready_tx, ready_rx) = oneshot::channel();
        let subscribers = self.notification_subscribers.clone();
        tokio::spawn(async move {
            stream_task_subscription(response, subscribers, ready_tx).await;
        });

        if !matches!(
            tokio::time::timeout(Duration::from_secs(5), ready_rx).await,
            Ok(Ok(()))
        ) {
            self.subscribed_workspaces.lock().await.remove(&workspace);
        }
    }

    async fn post(
        &self,
        method: &str,
        params: Value,
        request_name: Option<&str>,
        accept_event_stream: bool,
    ) -> anyhow::Result<reqwest::Response> {
        let request_id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let mut params = params.as_object().cloned().unwrap_or_default();
        params.insert(
            "_meta".to_string(),
            serde_json::json!({
                "io.modelcontextprotocol/protocolVersion": MODERN_MCP_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {},
                "io.modelcontextprotocol/clientInfo": {
                    "name": self.client_name,
                    "version": self.client_version
                }
            }),
        );
        let mut request = self
            .client
            .post(&self.uri)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::ACCEPT,
                if accept_event_stream {
                    "application/json, text/event-stream"
                } else {
                    "application/json"
                },
            )
            .header("MCP-Protocol-Version", MODERN_MCP_PROTOCOL_VERSION)
            .header("Mcp-Method", method)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "method": method,
                "params": params,
            }));
        if let Some(name) = request_name {
            request = request.header("Mcp-Name", mcp_header_value(name));
        }
        if let Some(auth_manager) = &self.auth_manager {
            let token = auth_manager.lock().await.get_access_token().await?;
            request = request.bearer_auth(token);
        }
        Ok(request.send().await?.error_for_status()?)
    }
}

fn mcp_header_value(value: &str) -> String {
    if !value.is_empty()
        && value.trim_matches([' ', '\t']) == value
        && value.bytes().all(|byte| matches!(byte, 0x21..=0x7e))
    {
        return value.to_string();
    }
    format!("=?base64?{}?=", URL_SAFE_NO_PAD.encode(value.as_bytes()))
}

async fn stream_task_subscription(
    response: reqwest::Response,
    subscribers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
    ready_tx: oneshot::Sender<()>,
) {
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut ready_tx = Some(ready_tx);
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            return;
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some((end, delimiter_len)) = next_sse_event(&buffer) {
            let event = buffer.drain(..end).collect::<String>();
            buffer.drain(..delimiter_len);
            let Some(message) = sse_message(&event) else {
                continue;
            };
            if message.get("method").and_then(Value::as_str)
                == Some("notifications/subscriptions/acknowledged")
            {
                if let Some(ready_tx) = ready_tx.take() {
                    let _ = ready_tx.send(());
                }
            }
            if let Some(notification) = modern_task_subscription_notification(&message) {
                subscribers.lock().await.iter().for_each(|subscriber| {
                    let _ = subscriber.try_send(notification.clone());
                });
            }
        }
    }
}

fn next_sse_event(buffer: &str) -> Option<(usize, usize)> {
    match (buffer.find("\n\n"), buffer.find("\r\n\r\n")) {
        (Some(lf), Some(crlf)) if crlf < lf => Some((crlf, 4)),
        (Some(lf), _) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

fn sse_message(event: &str) -> Option<Value> {
    let data = event
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::from_str(&data).ok()
}

fn modern_task_subscription_notification(message: &Value) -> Option<ServerNotification> {
    (message.get("method").and_then(Value::as_str) == Some("notifications/resources/updated")).then(
        || {
            ServerNotification::CustomNotification(rmcp::model::CustomNotification::new(
                "notifications/resources/updated",
                message.get("params").cloned(),
            ))
        },
    )
}

fn resolve_sampling_model_config() -> anyhow::Result<goose_providers::model::ModelConfig> {
    let config = crate::config::Config::global();
    let provider_name = config.get_goose_provider()?;
    let model_name = config.get_goose_model()?;
    crate::model_config::model_config_from_user_config(&provider_name, &model_name)
}

fn default_mcp_apps_ui_extensions() -> ExtensionCapabilities {
    let mut extensions = ExtensionCapabilities::new();
    let mut ui_extension_settings = JsonObject::new();
    ui_extension_settings.insert(
        "mimeTypes".to_string(),
        serde_json::json!([MCP_APPS_UI_MIME_TYPE]),
    );
    extensions.insert(MCP_APPS_UI_EXTENSION_ID.to_string(), ui_extension_settings);
    extensions
}

#[derive(Debug, Clone, Default)]
pub struct GooseMcpHostInfo {
    pub explicit_extensions: bool,
    pub extensions: ExtensionCapabilities,
    pub client_name: Option<String>,
    pub client_version: Option<String>,
}

impl GooseMcpHostInfo {
    pub fn mcpui_enabled(&self) -> bool {
        self.extensions.contains_key(MCP_APPS_UI_EXTENSION_ID)
    }
}

#[async_trait::async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn list_tools(
        &self,
        session_id: &str,
        next_cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error>;

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error>;

    fn get_info(&self) -> Option<&InitializeResult>;

    /// Return the extension's current instructions. The default reads from
    /// `get_info()`, but platform extensions can override this to provide
    /// dynamically computed instructions (e.g. freshly discovered skills).
    fn get_instructions(&self) -> Option<String> {
        self.get_info().and_then(|info| info.instructions.clone())
    }

    async fn list_resources(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _session_id: &str,
        _uri: &str,
        _cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_prompts(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _session_id: &str,
        _name: &str,
        _arguments: Value,
        _cancel_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }

    async fn get_moim(&self, _session_id: &str) -> Option<String> {
        None
    }

    async fn update_working_dir(&self, _new_dir: PathBuf) -> Result<(), Error> {
        Ok(())
    }
}

struct ActiveToolCallGuard {
    active_tool_calls: Arc<StdMutex<HashMap<String, Vec<String>>>>,
    session_id: String,
    tool_call_request_id: String,
}

impl Drop for ActiveToolCallGuard {
    fn drop(&mut self) {
        let mut active_tool_calls = self
            .active_tool_calls
            .lock()
            .expect("active_tool_calls mutex poisoned");
        if let Some(calls) = active_tool_calls.get_mut(&self.session_id) {
            if let Some(pos) = calls.iter().position(|id| id == &self.tool_call_request_id) {
                calls.remove(pos);
            }
            if calls.is_empty() {
                active_tool_calls.remove(&self.session_id);
            }
        }
    }
}

pub struct GooseClient {
    notification_handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
    provider: SharedProvider,
    session_id: Mutex<Option<String>>,
    active_tool_calls: Arc<StdMutex<HashMap<String, Vec<String>>>>,
    client_name: String,
    capabilities: GooseMcpClientCapabilities,
    working_dir: Arc<tokio::sync::RwLock<PathBuf>>,
}

impl GooseClient {
    pub fn new(
        handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
        provider: SharedProvider,
        client_name: String,
        capabilities: GooseMcpClientCapabilities,
        working_dir: PathBuf,
    ) -> Self {
        GooseClient {
            notification_handlers: handlers,
            provider,
            session_id: Mutex::new(None),
            active_tool_calls: Arc::new(StdMutex::new(HashMap::new())),
            client_name,
            capabilities,
            working_dir: Arc::new(tokio::sync::RwLock::new(working_dir)),
        }
    }

    pub fn shared_working_dir(&self) -> Arc<tokio::sync::RwLock<PathBuf>> {
        self.working_dir.clone()
    }

    async fn set_session_id(&self, session_id: &str) {
        let mut slot = self.session_id.lock().await;
        assert!(
            slot.as_deref().is_none_or(|s| s == session_id),
            "McpClient received requests from different sessions"
        );
        *slot = Some(session_id.to_string());
    }

    async fn current_session_id(&self) -> Option<String> {
        self.session_id.lock().await.clone()
    }

    async fn resolve_session_id(&self, extensions: &Extensions) -> Option<String> {
        // Prefer explicit MCP metadata, then the active request scope.
        let current_session_id = self.current_session_id().await;
        Self::session_id_from_extensions(extensions).or(current_session_id)
    }

    fn session_id_from_extensions(extensions: &Extensions) -> Option<String> {
        let meta = extensions.get::<Meta>()?;
        meta.0
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(SESSION_ID_HEADER))
            .and_then(|(_, value)| value.as_str())
            .map(|value| value.to_string())
    }

    fn tool_call_request_id_from_extensions(extensions: &Extensions) -> Option<String> {
        let meta = extensions.get::<Meta>()?;
        meta.0
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(TOOL_CALL_REQUEST_ID_HEADER))
            .and_then(|(_, value)| value.as_str())
            .map(|value| value.to_string())
    }

    fn register_active_tool_call(
        &self,
        session_id: &str,
        tool_call_request_id: &str,
    ) -> ActiveToolCallGuard {
        self.active_tool_calls
            .lock()
            .expect("active_tool_calls mutex poisoned")
            .entry(session_id.to_string())
            .or_default()
            .push(tool_call_request_id.to_string());
        ActiveToolCallGuard {
            active_tool_calls: self.active_tool_calls.clone(),
            session_id: session_id.to_string(),
            tool_call_request_id: tool_call_request_id.to_string(),
        }
    }

    fn resolve_tool_call_request_id(
        &self,
        session_id: &str,
        extensions: &Extensions,
    ) -> Result<String, ErrorData> {
        if let Some(tool_call_request_id) = Self::tool_call_request_id_from_extensions(extensions) {
            return Ok(tool_call_request_id);
        }

        let active_tool_calls = self
            .active_tool_calls
            .lock()
            .expect("active_tool_calls mutex poisoned");
        match active_tool_calls.get(session_id).map(Vec::as_slice) {
            Some([tool_call_request_id]) => Ok(tool_call_request_id.clone()),
            Some(calls) if calls.len() > 1 => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Cannot correlate elicitation request: multiple tool calls are active and the \
                 server did not echo the tool call request id",
                None,
            )),
            _ => Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Could not resolve tool call request id for elicitation request",
                None,
            )),
        }
    }

    fn resolved_extensions(&self) -> ExtensionCapabilities {
        if let Some(host_info) = &self.capabilities.host_info {
            if host_info.explicit_extensions {
                return host_info.extensions.clone();
            }
        }

        if self.capabilities.mcpui {
            return default_mcp_apps_ui_extensions();
        }

        ExtensionCapabilities::new()
    }

    fn resolved_client_info(&self) -> Implementation {
        let name = self
            .capabilities
            .host_info
            .as_ref()
            .and_then(|host_info| host_info.client_name.clone())
            .unwrap_or_else(|| self.client_name.clone());
        let version = self
            .capabilities
            .host_info
            .as_ref()
            .and_then(|host_info| host_info.client_version.clone())
            .unwrap_or_else(|| {
                std::env::var("GOOSE_MCP_CLIENT_VERSION")
                    .unwrap_or(env!("CARGO_PKG_VERSION").to_owned())
            });

        Implementation::new(name, version)
    }
}

fn working_dir_roots(dir: &std::path::Path) -> ListRootsResult {
    let uri = url::Url::from_file_path(dir)
        .map(|u| u.to_string())
        .unwrap_or_else(|()| format!("file://{}", dir.display()));
    ListRootsResult::new(vec![Root::new(uri).with_name("working_directory")])
}

impl ClientHandler for GooseClient {
    async fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> Result<ListRootsResult, ErrorData> {
        Ok(working_dir_roots(&self.working_dir.read().await))
    }

    async fn on_progress(
        &self,
        params: rmcp::model::ProgressNotificationParam,
        context: rmcp::service::NotificationContext<rmcp::RoleClient>,
    ) {
        self.notification_handlers
            .lock()
            .await
            .iter()
            .for_each(|handler| {
                let mut not = Notification::new(params.clone());
                not.extensions = context.extensions.clone();
                let _ = handler.try_send(ServerNotification::ProgressNotification(not));
            });
    }

    async fn on_logging_message(
        &self,
        params: rmcp::model::LoggingMessageNotificationParam,
        context: rmcp::service::NotificationContext<rmcp::RoleClient>,
    ) {
        self.notification_handlers
            .lock()
            .await
            .iter()
            .for_each(|handler| {
                let mut notification = LoggingMessageNotification::new(params.clone());
                notification.extensions = context.extensions.clone();
                let _ =
                    handler.try_send(ServerNotification::LoggingMessageNotification(notification));
            });
    }

    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, ErrorData> {
        let provider = self
            .provider
            .lock()
            .await
            .as_ref()
            .ok_or(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Could not use provider",
                None,
            ))?
            .clone();

        // Prefer explicit MCP metadata, then the active request scope.
        let session_id = self.resolve_session_id(&context.extensions).await;

        let provider_ready_messages: Vec<crate::conversation::message::Message> = params
            .messages
            .iter()
            .map(|msg| {
                let base = match msg.role {
                    Role::User => crate::conversation::message::Message::user(),
                    Role::Assistant => crate::conversation::message::Message::assistant(),
                };

                match msg.content.first().and_then(|c| c.as_text()) {
                    Some(text) => base.with_text(&text.text),
                    None => base,
                }
            })
            .collect();

        let system_prompt = params
            .system_prompt
            .as_deref()
            .unwrap_or("You are a general-purpose AI agent called goose");

        let model_config = resolve_sampling_model_config().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Could not resolve model config",
                Some(Value::from(e.to_string())),
            )
        })?;
        let (response, usage) = crate::session_context::with_session_id(
            session_id.clone(),
            provider.complete(&model_config, system_prompt, &provider_ready_messages, &[]),
        )
        .await
        .map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Unexpected error while completing the prompt",
                Some(Value::from(e.to_string())),
            )
        })?;

        Ok(CreateMessageResult::new(
            SamplingMessage::new(
                Role::Assistant,
                if let Some(content) = response.content.first() {
                    match content {
                        crate::conversation::message::MessageContent::Text(text) => {
                            SamplingMessageContent::text(&text.text)
                        }
                        crate::conversation::message::MessageContent::Image(img) => {
                            SamplingMessageContent::Image(rmcp::model::RawImageContent {
                                data: img.data.clone(),
                                mime_type: img.mime_type.clone(),
                                meta: None,
                            })
                        }
                        _ => SamplingMessageContent::text(""),
                    }
                } else {
                    SamplingMessageContent::text("")
                },
            ),
            usage.model,
        )
        .with_stop_reason(CreateMessageResult::STOP_REASON_END_TURN))
    }

    async fn create_elicitation(
        &self,
        request: CreateElicitationRequestParams,
        context: RequestContext<RoleClient>,
    ) -> Result<CreateElicitationResult, ErrorData> {
        let session_id = self
            .resolve_session_id(&context.extensions)
            .await
            .ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Could not resolve session id for elicitation request",
                    None,
                )
            })?;
        let tool_call_request_id =
            self.resolve_tool_call_request_id(&session_id, &context.extensions)?;

        let (message, schema_value) = match &request {
            CreateElicitationRequestParams::FormElicitationParams {
                message,
                requested_schema,
                ..
            } => {
                let schema_value = serde_json::to_value(requested_schema).map_err(|e| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("Failed to serialize elicitation schema: {}", e),
                        None,
                    )
                })?;
                (message.clone(), schema_value)
            }
            CreateElicitationRequestParams::UrlElicitationParams { message, url, .. } => {
                (message.clone(), serde_json::json!({ "url": url }))
            }
        };

        ActionRequiredManager::global()
            .request_and_wait(
                session_id,
                tool_call_request_id,
                message,
                schema_value,
                Duration::from_secs(300),
            )
            .await
            .map(|response| match response {
                ElicitationOutcome::Accept(user_data) => {
                    CreateElicitationResult::new(ElicitationAction::Accept).with_content(user_data)
                }
                ElicitationOutcome::Decline => {
                    CreateElicitationResult::new(ElicitationAction::Decline)
                }
                ElicitationOutcome::Cancel => {
                    CreateElicitationResult::new(ElicitationAction::Cancel)
                }
            })
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Elicitation request timed out or failed: {}", e),
                    None,
                )
            })
    }

    #[allow(deprecated)]
    fn get_info(&self) -> ClientInfo {
        let extensions = self.resolved_extensions();

        InitializeRequestParams::new(
            ClientCapabilities::builder()
                .enable_roots()
                .enable_extensions_with(extensions)
                .enable_sampling()
                .enable_elicitation()
                .build(),
            self.resolved_client_info(),
        )
        .with_protocol_version(ProtocolVersion::V_2025_03_26)
    }
}

#[derive(Debug, Clone)]
pub struct GooseMcpClientCapabilities {
    pub mcpui: bool,
    pub host_info: Option<GooseMcpHostInfo>,
}

/// The MCP client is the interface for MCP operations.
pub struct McpClient {
    client: Mutex<RunningService<RoleClient, GooseClient>>,
    notification_subscribers: Arc<Mutex<Vec<mpsc::Sender<ServerNotification>>>>,
    modern_task_subscription: Option<Arc<ModernTaskSubscription>>,
    server_info: Option<InitializeResult>,
    timeout: std::time::Duration,
    docker_container: Option<String>,
}

impl McpClient {
    pub async fn connect<T, E, A>(
        transport: T,
        timeout: std::time::Duration,
        provider: SharedProvider,
        client_name: String,
        capabilities: GooseMcpClientCapabilities,
        working_dir: PathBuf,
    ) -> Result<Self, ClientInitializeError>
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
    {
        Self::connect_with_container(
            transport,
            timeout,
            provider,
            None,
            client_name,
            capabilities,
            working_dir,
        )
        .await
    }

    pub async fn connect_with_container<T, E, A>(
        transport: T,
        timeout: std::time::Duration,
        provider: SharedProvider,
        docker_container: Option<String>,
        client_name: String,
        capabilities: GooseMcpClientCapabilities,
        working_dir: PathBuf,
    ) -> Result<Self, ClientInitializeError>
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
    {
        let notification_subscribers =
            Arc::new(Mutex::new(Vec::<mpsc::Sender<ServerNotification>>::new()));

        let client = GooseClient::new(
            notification_subscribers.clone(),
            provider,
            client_name.clone(),
            capabilities.clone(),
            working_dir,
        );
        let client: rmcp::service::RunningService<rmcp::RoleClient, GooseClient> =
            client.serve(transport).await?;
        let server_info = client.peer_info().map(|info| (*info).clone());

        Ok(Self {
            client: Mutex::new(client),
            notification_subscribers,
            modern_task_subscription: None,
            server_info,
            timeout,
            docker_container,
        })
    }

    pub(crate) async fn with_modern_task_subscription(
        mut self,
        client: reqwest::Client,
        uri: String,
        client_name: String,
        auth_manager: Option<Arc<Mutex<AuthorizationManager>>>,
    ) -> Self {
        self.modern_task_subscription = ModernTaskSubscription::probe(
            client,
            uri,
            client_name,
            env!("CARGO_PKG_VERSION").to_string(),
            self.notification_subscribers.clone(),
            auth_manager,
        )
        .await;
        self
    }

    pub fn docker_container(&self) -> Option<&str> {
        self.docker_container.as_deref()
    }

    async fn do_update_working_dir(&self, new_dir: PathBuf) -> Result<(), Error> {
        let client = self.client.lock().await;
        let shared = client.service().shared_working_dir();
        *shared.write().await = new_dir;
        client.peer().notify_roots_list_changed().await?;
        Ok(())
    }

    async fn send_request_with_context(
        &self,
        session_id: &str,
        working_dir: Option<&str>,
        tool_call_request_id: Option<&str>,
        request: ClientRequest,
        cancel_token: CancellationToken,
    ) -> Result<ServerResult, Error> {
        let request = inject_session_context_into_request(
            request,
            Some(session_id),
            working_dir,
            tool_call_request_id,
        );
        let active_tool_call = tool_call_request_id.filter(|id| !id.is_empty());
        // The inner mutex is held only for the send; the actual response wait
        // happens outside the lock so concurrent calls can overlap. The guard
        // unregisters the active tool call on drop, covering cancellation and
        // dropped reply streams as well as normal completion.
        let (handle, _active_tool_call_guard) = {
            let client = self.client.lock().await;
            client.service().set_session_id(session_id).await;
            let guard = active_tool_call.map(|tool_call_request_id| {
                client
                    .service()
                    .register_active_tool_call(session_id, tool_call_request_id)
            });
            let handle = client
                .send_cancellable_request(request, PeerRequestOptions::no_options())
                .await?;
            (handle, guard)
        };

        await_response(handle, self.timeout, &cancel_token).await
    }
}

async fn await_response(
    handle: RequestHandle<RoleClient>,
    timeout: Duration,
    cancel_token: &CancellationToken,
) -> Result<<RoleClient as ServiceRole>::PeerResp, ServiceError> {
    let receiver = handle.rx;
    let peer = handle.peer;
    let request_id = handle.id;
    tokio::select! {
        result = receiver => {
            result.map_err(|_e| ServiceError::TransportClosed)?
        }
        _ = tokio::time::sleep(timeout) => {
            send_cancel_message(&peer, request_id, Some("timed out".to_owned())).await?;
            Err(ServiceError::Timeout{timeout})
        }
        _ = cancel_token.cancelled() => {
            send_cancel_message(&peer, request_id, Some("operation cancelled".to_owned())).await?;
            Err(ServiceError::Cancelled { reason: None })
        }
    }
}

async fn send_cancel_message(
    peer: &Peer<RoleClient>,
    request_id: RequestId,
    reason: Option<String>,
) -> Result<(), ServiceError> {
    peer.send_notification(
        Notification::new(CancelledNotificationParam { request_id, reason }).into(),
    )
    .await
}

#[async_trait::async_trait]
impl McpClientTrait for McpClient {
    fn get_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    async fn list_resources(
        &self,
        session_id: &str,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        let res = self
            .send_request_with_context(
                session_id,
                None,
                None,
                ClientRequest::ListResourcesRequest(RequestOptionalParam::with_param(
                    PaginatedRequestParams::default().with_cursor(cursor),
                )),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ListResourcesResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn read_resource(
        &self,
        session_id: &str,
        uri: &str,
        cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        let res = self
            .send_request_with_context(
                session_id,
                None,
                None,
                ClientRequest::ReadResourceRequest(Request::new(ReadResourceRequestParams::new(
                    uri.to_string(),
                ))),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ReadResourceResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_tools(
        &self,
        session_id: &str,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let res = self
            .send_request_with_context(
                session_id,
                None,
                None,
                ClientRequest::ListToolsRequest(RequestOptionalParam::with_param(
                    PaginatedRequestParams::default().with_cursor(cursor),
                )),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ListToolsResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        if let (Some(subscription), Some(working_dir)) =
            (&self.modern_task_subscription, ctx.working_dir.as_deref())
        {
            subscription
                .ensure_workspace_subscription(working_dir)
                .await;
        }
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }
        let request = ClientRequest::CallToolRequest(Request::new(params));

        let result = self
            .send_request_with_context(
                &ctx.session_id,
                ctx.working_dir_str(),
                ctx.tool_call_request_id.as_deref(),
                request,
                cancel_token,
            )
            .await;

        match result? {
            ServerResult::CallToolResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_prompts(
        &self,
        session_id: &str,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        let res = self
            .send_request_with_context(
                session_id,
                None,
                None,
                ClientRequest::ListPromptsRequest(RequestOptionalParam::with_param(
                    PaginatedRequestParams::default().with_cursor(cursor),
                )),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::ListPromptsResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn get_prompt(
        &self,
        session_id: &str,
        name: &str,
        arguments: Value,
        cancel_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        let mut params = GetPromptRequestParams::new(name.to_string());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }
        let res = self
            .send_request_with_context(
                session_id,
                None,
                None,
                ClientRequest::GetPromptRequest(Request::new(params)),
                cancel_token,
            )
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

    async fn update_working_dir(&self, new_dir: PathBuf) -> Result<(), Error> {
        self.do_update_working_dir(new_dir).await
    }
}

/// Injects the given session_id and working_dir into Extensions._meta.
/// None (or empty) removes any existing values.
fn inject_session_context_into_extensions(
    mut extensions: Extensions,
    session_id: Option<&str>,
    working_dir: Option<&str>,
    tool_call_request_id: Option<&str>,
) -> Extensions {
    let session_id = session_id.filter(|id| !id.is_empty());
    let working_dir = working_dir.filter(|dir| !dir.is_empty());
    let tool_call_request_id = tool_call_request_id.filter(|id| !id.is_empty());
    let mut meta_map = extensions
        .get::<Meta>()
        .map(|meta| meta.0.clone())
        .unwrap_or_default();

    // JsonObject is case-sensitive, so we use retain for case-insensitive removal
    meta_map.retain(|k, _| {
        !k.eq_ignore_ascii_case(SESSION_ID_HEADER)
            && !k.eq_ignore_ascii_case(WORKING_DIR_HEADER)
            && !k.eq_ignore_ascii_case(TOOL_CALL_REQUEST_ID_HEADER)
    });

    if let Some(session_id) = session_id {
        meta_map.insert(
            SESSION_ID_HEADER.to_string(),
            Value::String(session_id.to_string()),
        );
    }

    if let Some(working_dir) = working_dir {
        meta_map.insert(
            WORKING_DIR_HEADER.to_string(),
            Value::String(working_dir.to_string()),
        );
    }

    if let Some(tool_call_request_id) = tool_call_request_id {
        meta_map.insert(
            TOOL_CALL_REQUEST_ID_HEADER.to_string(),
            Value::String(tool_call_request_id.to_string()),
        );
    }

    extensions.insert(Meta(meta_map));
    extensions
}

fn inject_session_context_into_request(
    request: ClientRequest,
    session_id: Option<&str>,
    working_dir: Option<&str>,
    tool_call_request_id: Option<&str>,
) -> ClientRequest {
    match request {
        ClientRequest::ListResourcesRequest(mut req) => {
            req.extensions = inject_session_context_into_extensions(
                req.extensions,
                session_id,
                working_dir,
                None,
            );
            ClientRequest::ListResourcesRequest(req)
        }
        ClientRequest::ReadResourceRequest(mut req) => {
            req.extensions = inject_session_context_into_extensions(
                req.extensions,
                session_id,
                working_dir,
                None,
            );
            ClientRequest::ReadResourceRequest(req)
        }
        ClientRequest::ListToolsRequest(mut req) => {
            req.extensions = inject_session_context_into_extensions(
                req.extensions,
                session_id,
                working_dir,
                None,
            );
            ClientRequest::ListToolsRequest(req)
        }
        ClientRequest::CallToolRequest(mut req) => {
            req.extensions = inject_session_context_into_extensions(
                req.extensions,
                session_id,
                working_dir,
                tool_call_request_id,
            );
            ClientRequest::CallToolRequest(req)
        }
        ClientRequest::ListPromptsRequest(mut req) => {
            req.extensions = inject_session_context_into_extensions(
                req.extensions,
                session_id,
                working_dir,
                None,
            );
            ClientRequest::ListPromptsRequest(req)
        }
        ClientRequest::GetPromptRequest(mut req) => {
            req.extensions = inject_session_context_into_extensions(
                req.extensions,
                session_id,
                working_dir,
                None,
            );
            ClientRequest::GetPromptRequest(req)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::GoosePlatform;
    use serde_json::json;
    use test_case::test_case;

    fn new_client(platform: GoosePlatform) -> GooseClient {
        let capabilities = match platform {
            GoosePlatform::GooseDesktop => GooseMcpClientCapabilities {
                mcpui: true,
                host_info: None,
            },
            GoosePlatform::GooseCli => GooseMcpClientCapabilities {
                mcpui: false,
                host_info: None,
            },
        };

        GooseClient::new(
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(None)),
            platform.to_string(),
            capabilities,
            std::env::current_dir().unwrap_or_default(),
        )
    }

    fn request_extensions(request: &ClientRequest) -> Option<&Extensions> {
        match request {
            ClientRequest::ListResourcesRequest(req) => Some(&req.extensions),
            ClientRequest::ReadResourceRequest(req) => Some(&req.extensions),
            ClientRequest::ListToolsRequest(req) => Some(&req.extensions),
            ClientRequest::CallToolRequest(req) => Some(&req.extensions),
            ClientRequest::ListPromptsRequest(req) => Some(&req.extensions),
            ClientRequest::GetPromptRequest(req) => Some(&req.extensions),
            _ => None,
        }
    }

    fn list_resources_request(extensions: Extensions) -> ClientRequest {
        let mut req = RequestOptionalParam::with_param(PaginatedRequestParams::default());
        req.extensions = extensions;
        ClientRequest::ListResourcesRequest(req)
    }

    fn read_resource_request(extensions: Extensions) -> ClientRequest {
        let mut req = Request::new(ReadResourceRequestParams::new(
            "test://resource".to_string(),
        ));
        req.extensions = extensions;
        ClientRequest::ReadResourceRequest(req)
    }

    fn list_tools_request(extensions: Extensions) -> ClientRequest {
        let mut req = RequestOptionalParam::with_param(PaginatedRequestParams::default());
        req.extensions = extensions;
        ClientRequest::ListToolsRequest(req)
    }

    fn call_tool_request(extensions: Extensions) -> ClientRequest {
        let mut req = Request::new(CallToolRequestParams::new("tool".to_string()));
        req.extensions = extensions;
        ClientRequest::CallToolRequest(req)
    }

    fn list_prompts_request(extensions: Extensions) -> ClientRequest {
        let mut req = RequestOptionalParam::with_param(PaginatedRequestParams::default());
        req.extensions = extensions;
        ClientRequest::ListPromptsRequest(req)
    }

    fn get_prompt_request(extensions: Extensions) -> ClientRequest {
        let mut req = Request::new(GetPromptRequestParams::new("prompt".to_string()));
        req.extensions = extensions;
        ClientRequest::GetPromptRequest(req)
    }

    #[test_case(
        Some("ext-session"),
        Some("current-session"),
        Some("ext-session");
        "extensions win"
    )]
    #[test_case(
        None,
        Some("current-session"),
        Some("current-session");
        "current when no extensions"
    )]
    #[test_case(
        None,
        None,
        None;
        "no session when no extensions or current"
    )]
    fn test_resolve_session_id(
        ext_session: Option<&str>,
        current_session: Option<&str>,
        expected: Option<&str>,
    ) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let client = new_client(GoosePlatform::GooseCli);
            if let Some(session_id) = current_session {
                client.set_session_id(session_id).await;
            }

            let extensions =
                inject_session_context_into_extensions(Extensions::new(), ext_session, None, None);

            let resolved = client.resolve_session_id(&extensions).await;

            let expected = expected.map(str::to_string);
            assert_eq!(resolved, expected);
        });
    }

    #[test]
    fn test_resolve_tool_call_request_id_from_extensions() {
        let client = new_client(GoosePlatform::GooseCli);
        let _guard = client.register_active_tool_call("session-a", "active-tool-call");
        let extensions = inject_session_context_into_extensions(
            Extensions::new(),
            Some("session-a"),
            None,
            Some("extension-tool-call"),
        );

        let resolved = client
            .resolve_tool_call_request_id("session-a", &extensions)
            .unwrap();

        assert_eq!(resolved, "extension-tool-call");
    }

    #[test]
    fn test_resolve_tool_call_request_id_from_active_call() {
        let client = new_client(GoosePlatform::GooseCli);
        let _guard = client.register_active_tool_call("session-a", "active-tool-call");

        let resolved = client
            .resolve_tool_call_request_id("session-a", &Extensions::new())
            .unwrap();

        assert_eq!(resolved, "active-tool-call");
    }

    #[test]
    fn test_resolve_tool_call_request_id_errors_when_calls_overlap() {
        let client = new_client(GoosePlatform::GooseCli);
        let _guard_a = client.register_active_tool_call("session-a", "active-tool-call-a");
        let _guard_b = client.register_active_tool_call("session-a", "active-tool-call-b");

        let error = client
            .resolve_tool_call_request_id("session-a", &Extensions::new())
            .expect_err("ambiguous elicitation should not resolve to an arbitrary call");

        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
    }

    #[test]
    fn test_resolve_tool_call_request_id_prefers_echoed_id_while_calls_overlap() {
        let client = new_client(GoosePlatform::GooseCli);
        let _guard_a = client.register_active_tool_call("session-a", "active-tool-call-a");
        let _guard_b = client.register_active_tool_call("session-a", "active-tool-call-b");
        let extensions = inject_session_context_into_extensions(
            Extensions::new(),
            Some("session-a"),
            None,
            Some("active-tool-call-a"),
        );

        let resolved = client
            .resolve_tool_call_request_id("session-a", &extensions)
            .unwrap();

        assert_eq!(resolved, "active-tool-call-a");
    }

    #[test]
    fn test_dropping_guard_unregisters_active_tool_call() {
        let client = new_client(GoosePlatform::GooseCli);
        let guard_a = client.register_active_tool_call("session-a", "active-tool-call-a");
        let _guard_b = client.register_active_tool_call("session-a", "active-tool-call-b");

        drop(guard_a);

        let resolved = client
            .resolve_tool_call_request_id("session-a", &Extensions::new())
            .unwrap();

        assert_eq!(resolved, "active-tool-call-b");
    }

    #[test_case(list_resources_request; "list_resources")]
    #[test_case(read_resource_request; "read_resource")]
    #[test_case(list_tools_request; "list_tools")]
    #[test_case(call_tool_request; "call_tool")]
    #[test_case(list_prompts_request; "list_prompts")]
    #[test_case(get_prompt_request; "get_prompt")]
    fn test_request_injects_session(request_builder: fn(Extensions) -> ClientRequest) {
        let session_id = "test-session-id";
        let mut extensions = Extensions::new();
        extensions.insert(
            serde_json::from_value::<Meta>(json!({
                "Goose-Session-Id": "old-session-id",
                "other-key": "preserve-me"
            }))
            .unwrap(),
        );

        let request = request_builder(extensions);
        let request = inject_session_context_into_request(request, Some(session_id), None, None);
        let extensions = request_extensions(&request).expect("request should have extensions");
        let meta = extensions
            .get::<Meta>()
            .expect("extensions should contain meta");

        assert_eq!(
            meta.0.get(SESSION_ID_HEADER),
            Some(&Value::String(session_id.to_string()))
        );
        assert_eq!(
            meta.0.get("other-key"),
            Some(&Value::String("preserve-me".to_string()))
        );
        if matches!(request, ClientRequest::CallToolRequest(_)) {
            assert!(!meta.0.contains_key(TOOL_CALL_REQUEST_ID_HEADER));
        }
    }

    #[test]
    fn test_session_id_in_mcp_meta() {
        let session_id = "test-session-789";
        let extensions = inject_session_context_into_extensions(
            Default::default(),
            Some(session_id),
            None,
            None,
        );
        let mcp_meta = extensions.get::<Meta>().unwrap();

        assert_eq!(
            &mcp_meta.0,
            json!({
                SESSION_ID_HEADER: session_id
            })
            .as_object()
            .unwrap()
        );
    }

    #[test_case(
        Some("new-session-id"),
        json!({
            SESSION_ID_HEADER: "new-session-id",
            "other-key": "preserve-me"
        });
        "replace"
    )]
    #[test_case(
        None,
        json!({
            "other-key": "preserve-me"
        });
        "remove"
    )]
    #[test_case(
        Some(""),
        json!({
            "other-key": "preserve-me"
        });
        "empty removes"
    )]
    fn test_session_id_case_insensitive_replacement(
        session_id: Option<&str>,
        expected_meta: serde_json::Value,
    ) {
        use rmcp::model::Extensions;
        use serde_json::from_value;

        let mut extensions = Extensions::new();
        extensions.insert(
            from_value::<Meta>(json!({
                SESSION_ID_HEADER: "old-session-1",
                "Agent-Session-Id": "old-session-2",
                "other-key": "preserve-me"
            }))
            .unwrap(),
        );

        let extensions = inject_session_context_into_extensions(extensions, session_id, None, None);
        let mcp_meta = extensions.get::<Meta>().unwrap();

        assert_eq!(&mcp_meta.0, expected_meta.as_object().unwrap());
    }

    #[test]
    fn test_tool_call_request_id_injected_only_for_call_tool() {
        let session_id = "test-session-id";
        let tool_call_request_id = "tool-request-1";

        let call_request = inject_session_context_into_request(
            call_tool_request(Extensions::new()),
            Some(session_id),
            None,
            Some(tool_call_request_id),
        );
        let call_meta = request_extensions(&call_request)
            .and_then(|extensions| extensions.get::<Meta>())
            .expect("call request should have meta");
        assert_eq!(
            call_meta.0.get(TOOL_CALL_REQUEST_ID_HEADER),
            Some(&Value::String(tool_call_request_id.to_string()))
        );

        let tools_request = inject_session_context_into_request(
            list_tools_request(Extensions::new()),
            Some(session_id),
            None,
            Some(tool_call_request_id),
        );
        let tools_meta = request_extensions(&tools_request)
            .and_then(|extensions| extensions.get::<Meta>())
            .expect("list tools request should have meta");
        assert!(!tools_meta.0.contains_key(TOOL_CALL_REQUEST_ID_HEADER));
    }

    #[test]
    fn test_client_info_advertises_mcp_apps_ui_extension() {
        let client = new_client(GoosePlatform::GooseDesktop);
        let info = ClientHandler::get_info(&client);

        // Verify the client advertises the MCP Apps UI extension capability
        let extensions = info
            .capabilities
            .extensions
            .expect("capabilities should have extensions");

        let ui_ext = extensions
            .get("io.modelcontextprotocol/ui")
            .expect("should have io.modelcontextprotocol/ui extension");

        let mime_types = ui_ext
            .get("mimeTypes")
            .expect("ui extension should have mimeTypes");

        assert_eq!(mime_types, &json!(["text/html;profile=mcp-app"]));
    }

    #[test]
    fn test_client_capabilities_advertise_roots() {
        let client = new_client(GoosePlatform::GooseCli);
        let info = ClientHandler::get_info(&client);
        assert!(
            info.capabilities.roots.is_some(),
            "client should advertise roots capability"
        );
    }

    #[test]
    fn test_explicit_host_info_passes_through_client_identity() {
        let client = GooseClient::new(
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(None)),
            GoosePlatform::GooseDesktop.to_string(),
            GooseMcpClientCapabilities {
                mcpui: true,
                host_info: Some(GooseMcpHostInfo {
                    explicit_extensions: true,
                    extensions: ExtensionCapabilities::new(),
                    client_name: Some("goose2".to_string()),
                    client_version: Some("0.1.0".to_string()),
                }),
            },
            std::env::current_dir().unwrap_or_default(),
        );

        let info = ClientHandler::get_info(&client);
        assert_eq!(info.client_info.name, "goose2");
        assert_eq!(info.client_info.version, "0.1.0");
        let extensions = info
            .capabilities
            .extensions
            .expect("client should still serialize an extensions object");
        assert!(
            !extensions.contains_key(MCP_APPS_UI_EXTENSION_ID),
            "explicit empty host extensions should disable platform fallback"
        );
    }

    #[test]
    fn test_explicit_host_extensions_override_platform_fallback() {
        let client = GooseClient::new(
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(None)),
            GoosePlatform::GooseCli.to_string(),
            GooseMcpClientCapabilities {
                mcpui: false,
                host_info: Some(GooseMcpHostInfo {
                    explicit_extensions: true,
                    extensions: default_mcp_apps_ui_extensions(),
                    client_name: Some("goose2".to_string()),
                    client_version: Some("0.1.0".to_string()),
                }),
            },
            std::env::current_dir().unwrap_or_default(),
        );

        let info = ClientHandler::get_info(&client);
        let extensions = info
            .capabilities
            .extensions
            .expect("capabilities should have explicit host extensions");

        assert!(extensions.contains_key(MCP_APPS_UI_EXTENSION_ID));
        assert_eq!(info.client_info.name, "goose2");
    }

    #[test]
    fn test_host_identity_does_not_disable_platform_fallback_without_explicit_extensions() {
        let client = GooseClient::new(
            Arc::new(Mutex::new(Vec::new())),
            Arc::new(Mutex::new(None)),
            GoosePlatform::GooseDesktop.to_string(),
            GooseMcpClientCapabilities {
                mcpui: true,
                host_info: Some(GooseMcpHostInfo {
                    explicit_extensions: false,
                    extensions: ExtensionCapabilities::new(),
                    client_name: Some("goose2".to_string()),
                    client_version: Some("0.1.0".to_string()),
                }),
            },
            std::env::current_dir().unwrap_or_default(),
        );

        let info = ClientHandler::get_info(&client);
        let extensions = info
            .capabilities
            .extensions
            .expect("platform fallback should still advertise MCP Apps UI");

        assert!(extensions.contains_key(MCP_APPS_UI_EXTENSION_ID));
        assert_eq!(info.client_info.name, "goose2");
    }

    #[test]
    fn test_working_dir_roots_returns_current_dir_as_root() {
        let dir = PathBuf::from("/tmp/test-project");
        let result = working_dir_roots(&dir);
        assert_eq!(result.roots.len(), 1);
        assert_eq!(result.roots[0].uri, "file:///tmp/test-project");
        assert_eq!(result.roots[0].name.as_deref(), Some("working_directory"));
    }

    #[test]
    fn modern_task_subscription_forwards_resource_updates() {
        let notification = modern_task_subscription_notification(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/updated",
            "params": {
                "uri": "ferrosa-memory://tasks/workspaces/L3JlcG8vZ29vc2U/active",
                "_meta": {
                    "io.modelcontextprotocol/subscriptionId": 42
                }
            }
        }))
        .expect("resource update should become a Goose MCP notification");

        let ServerNotification::CustomNotification(notification) = notification else {
            panic!("expected a custom notification");
        };
        assert_eq!(notification.method, "notifications/resources/updated");
        assert_eq!(
            notification.params.expect("notification params")["_meta"]
                ["io.modelcontextprotocol/subscriptionId"],
            42
        );
    }

    #[test]
    fn modern_task_subscription_parses_crlf_sse_events() {
        let mut buffer = "event: message\r\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/resources/updated\",\"params\":{}}\r\n\r\n".to_string();
        let (end, delimiter_len) = next_sse_event(&buffer).expect("CRLF-delimited SSE event");
        let event = buffer.drain(..end).collect::<String>();
        buffer.drain(..delimiter_len);

        let message = sse_message(&event).expect("SSE JSON data");
        assert_eq!(
            message.get("method").and_then(Value::as_str),
            Some("notifications/resources/updated")
        );
        assert!(buffer.is_empty());
    }

    #[tokio::test]
    async fn modern_task_subscription_streams_workspace_updates() {
        use axum::{
            response::{sse::Event, IntoResponse, Sse},
            routing::post,
            Json, Router,
        };
        use std::convert::Infallible;

        let app = Router::new().route(
            "/mcp",
            post(|Json(request): Json<Value>| async move {
                if request["method"] == "subscriptions/listen" {
                    let id = request["id"].clone();
                    let events = async_stream::stream! {
                        yield Ok::<_, Infallible>(Event::default().data(serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/subscriptions/acknowledged",
                            "params": { "_meta": { "io.modelcontextprotocol/subscriptionId": id } }
                        }).to_string()));
                        yield Ok::<_, Infallible>(Event::default().data(serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/resources/updated",
                            "params": {
                                "uri": "ferrosa-memory://tasks/workspaces/L3JlcG8vZ29vc2U/active",
                                "_meta": { "io.modelcontextprotocol/subscriptionId": id }
                            }
                        }).to_string()));
                    };
                    Sse::new(events).into_response()
                } else if request["method"] == "server/discover" {
                    Json(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "result": {
                            "supportedVersions": [MODERN_MCP_PROTOCOL_VERSION],
                            "capabilities": { "resources": { "subscribe": true } }
                        }
                    }))
                    .into_response()
                } else {
                    Json(serde_json::json!({ "jsonrpc": "2.0", "id": request["id"], "result": {} }))
                        .into_response()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let (sender, mut receiver) = mpsc::channel(1);
        subscribers.lock().await.push(sender);
        let subscription = ModernTaskSubscription::probe(
            reqwest::Client::new(),
            format!("http://{address}/mcp"),
            "goose-test".to_string(),
            "1.0.0".to_string(),
            subscribers,
            None,
        )
        .await
        .expect("modern server should be detected");

        subscription
            .ensure_workspace_subscription(std::path::Path::new("/repo/goose"))
            .await;

        let ServerNotification::CustomNotification(notification) =
            tokio::time::timeout(Duration::from_secs(1), receiver.recv())
                .await
                .unwrap()
                .expect("resource update")
        else {
            panic!("expected a custom notification");
        };
        assert_eq!(notification.method, "notifications/resources/updated");
        assert_eq!(
            notification.params.expect("notification params")["uri"],
            "ferrosa-memory://tasks/workspaces/L3JlcG8vZ29vc2U/active"
        );

        server.abort();
    }
}
