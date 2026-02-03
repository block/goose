use crate::action_required_manager::ActionRequiredManager;
use crate::agents::types::SharedProvider;
use crate::session_context::{SESSION_ID_HEADER, WORKING_DIR_HEADER};
use rmcp::model::{
    Content, CreateElicitationRequestParams, CreateElicitationResult, ElicitationAction, ErrorCode,
    ExtensionCapabilities, Extensions, JsonObject, Meta,
};
/// MCP client implementation for Goose
use rmcp::{
    model::{
        CallToolRequest, CallToolRequestParams, CallToolResult, CancelledNotification,
        CancelledNotificationMethod, CancelledNotificationParam, ClientCapabilities, ClientInfo,
        ClientRequest, CreateMessageRequestParams, CreateMessageResult, GetPromptRequest,
        GetPromptRequestParams, GetPromptResult, Implementation, InitializeResult,
        ListPromptsRequest, ListPromptsResult, ListResourcesRequest, ListResourcesResult,
        ListToolsRequest, ListToolsResult, LoggingMessageNotification,
        LoggingMessageNotificationMethod, PaginatedRequestParams, ProgressNotification,
        ProgressNotificationMethod, ProtocolVersion, ReadResourceRequest,
        ReadResourceRequestParams, ReadResourceResult, RequestId, Role, SamplingMessage,
        ServerNotification, ServerResult,
    },
    service::{
        ClientInitializeError, PeerRequestOptions, RequestContext, RequestHandle, RunningService,
        ServiceRole,
    },
    transport::IntoTransport,
    ClientHandler, ErrorData, Peer, RoleClient, ServiceError, ServiceExt,
};
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::sync::{
    mpsc::{self, Sender},
    Mutex,
};
use tokio_util::sync::CancellationToken;
pub type BoxError = Box<dyn std::error::Error + Sync + Send>;

pub type Error = rmcp::ServiceError;

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
        session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        working_dir: Option<&str>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error>;

    fn get_info(&self) -> Option<&InitializeResult>;

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
}

pub struct GooseClient {
    notification_handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
    provider: SharedProvider,
    // Single-slot because calls are serialized per MCP client.
    current_session_id: Arc<Mutex<Option<String>>>,
}

impl GooseClient {
    pub fn new(
        handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
        provider: SharedProvider,
    ) -> Self {
        GooseClient {
            notification_handlers: handlers,
            provider,
            current_session_id: Arc::new(Mutex::new(None)),
        }
    }

    async fn set_current_session_id(&self, session_id: &str) {
        let mut slot = self.current_session_id.lock().await;
        *slot = Some(session_id.to_string());
    }

    async fn clear_current_session_id(&self) {
        let mut slot = self.current_session_id.lock().await;
        *slot = None;
    }

    async fn current_session_id(&self) -> Option<String> {
        let slot = self.current_session_id.lock().await;
        slot.clone()
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
}

impl ClientHandler for GooseClient {
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
    ) {
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

                match msg.content.as_text() {
                    Some(text) => base.with_text(&text.text),
                    None => base.with_content(msg.content.clone().into()),
                }
            })
            .collect();

        let system_prompt = params
            .system_prompt
            .as_deref()
            .unwrap_or("You are a general-purpose AI agent called goose");

        let (response, usage) = provider
            .complete_with_model(
                session_id.as_deref(),
                &provider.get_model_config(),
                system_prompt,
                &provider_ready_messages,
                &[],
            )
            .await
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Unexpected error while completing the prompt",
                    Some(Value::from(e.to_string())),
                )
            })?;

        Ok(CreateMessageResult {
            model: usage.model,
            stop_reason: Some(CreateMessageResult::STOP_REASON_END_TURN.to_string()),
            message: SamplingMessage {
                role: Role::Assistant,
                // TODO(alexhancock): MCP sampling currently only supports one content on each SamplingMessage
                // https://modelcontextprotocol.io/specification/draft/client/sampling#messages
                // This doesn't mesh well with goose's approach which has Vec<MessageContent>
                // There is a proposal to MCP which is agreed to go in the next version to have SamplingMessages support multiple content parts
                // https://github.com/modelcontextprotocol/modelcontextprotocol/pull/198
                // Until that is formalized, we can take the first message content from the provider and use it
                content: if let Some(content) = response.content.first() {
                    match content {
                        crate::conversation::message::MessageContent::Text(text) => {
                            Content::text(&text.text)
                        }
                        crate::conversation::message::MessageContent::Image(img) => {
                            Content::image(&img.data, &img.mime_type)
                        }
                        // TODO(alexhancock) - Content::Audio? goose's messages don't currently have it
                        _ => Content::text(""),
                    }
                } else {
                    Content::text("")
                },
            },
        })
    }

    async fn create_elicitation(
        &self,
        request: CreateElicitationRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateElicitationResult, ErrorData> {
        let schema_value = serde_json::to_value(&request.requested_schema).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to serialize elicitation schema: {}", e),
                None,
            )
        })?;

        ActionRequiredManager::global()
            .request_and_wait(
                request.message.clone(),
                schema_value,
                Duration::from_secs(300),
            )
            .await
            .map(|user_data| CreateElicitationResult {
                action: ElicitationAction::Accept,
                content: Some(user_data),
            })
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Elicitation request timed out or failed: {}", e),
                    None,
                )
            })
    }

    fn get_info(&self) -> ClientInfo {
        // Build MCP Apps UI extension capability
        // See: https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx
        let mut ui_extension_settings = JsonObject::new();
        ui_extension_settings.insert(
            "mimeTypes".to_string(),
            serde_json::json!(["text/html;profile=mcp-app"]),
        );

        let mut extensions = ExtensionCapabilities::new();
        extensions.insert(
            "io.modelcontextprotocol/ui".to_string(),
            ui_extension_settings,
        );

        ClientInfo {
            meta: None,
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ClientCapabilities::builder()
                .enable_extensions_with(extensions)
                .enable_sampling()
                .enable_elicitation()
                .build(),
            client_info: Implementation {
                name: "goose".to_string(),
                version: std::env::var("GOOSE_MCP_CLIENT_VERSION")
                    .unwrap_or(env!("CARGO_PKG_VERSION").to_owned()),
                icons: None,
                title: None,
                website_url: None,
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
    docker_container: Option<String>,
}

impl McpClient {
    pub async fn connect<T, E, A>(
        transport: T,
        timeout: std::time::Duration,
        provider: SharedProvider,
    ) -> Result<Self, ClientInitializeError>
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
    {
        Self::connect_with_container(transport, timeout, provider, None).await
    }

    pub async fn connect_with_container<T, E, A>(
        transport: T,
        timeout: std::time::Duration,
        provider: SharedProvider,
        docker_container: Option<String>,
    ) -> Result<Self, ClientInitializeError>
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + From<std::io::Error> + Send + Sync + 'static,
    {
        let notification_subscribers =
            Arc::new(Mutex::new(Vec::<mpsc::Sender<ServerNotification>>::new()));

        let client = GooseClient::new(notification_subscribers.clone(), provider);
        let client: rmcp::service::RunningService<rmcp::RoleClient, GooseClient> =
            client.serve(transport).await?;
        let server_info = client.peer_info().cloned();

        Ok(Self {
            client: Mutex::new(client),
            notification_subscribers,
            server_info,
            timeout,
            docker_container,
        })
    }

    pub fn docker_container(&self) -> Option<&str> {
        self.docker_container.as_deref()
    }

    async fn send_request_with_context(
        &self,
        session_id: &str,
        working_dir: Option<&str>,
        request: ClientRequest,
        cancel_token: CancellationToken,
    ) -> Result<ServerResult, Error> {
        let request = inject_session_context_into_request(request, Some(session_id), working_dir);
        // ExtensionManager serializes calls per MCP connection, so one current_session_id slot
        // is sufficient for mapping callbacks to the active request session.
        let handle = {
            let client = self.client.lock().await;
            client.service().set_current_session_id(session_id).await;
            client
                .send_cancellable_request(request, PeerRequestOptions::no_options())
                .await
        };

        let handle = match handle {
            Ok(handle) => handle,
            Err(err) => {
                let client = self.client.lock().await;
                client.service().clear_current_session_id().await;
                return Err(err);
            }
        };

        let result = await_response(handle, self.timeout, &cancel_token).await;

        let client = self.client.lock().await;
        client.service().clear_current_session_id().await;

        result
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
        CancelledNotification {
            params: CancelledNotificationParam { request_id, reason },
            method: CancelledNotificationMethod,
            extensions: Default::default(),
        }
        .into(),
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
                ClientRequest::ListResourcesRequest(ListResourcesRequest {
                    params: Some(PaginatedRequestParams { meta: None, cursor }),
                    method: Default::default(),
                    extensions: Default::default(),
                }),
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
                ClientRequest::ReadResourceRequest(ReadResourceRequest {
                    params: ReadResourceRequestParams {
                        meta: None,
                        uri: uri.to_string(),
                    },
                    method: Default::default(),
                    extensions: Default::default(),
                }),
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
                ClientRequest::ListToolsRequest(ListToolsRequest {
                    params: Some(PaginatedRequestParams { meta: None, cursor }),
                    method: Default::default(),
                    extensions: Default::default(),
                }),
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
        session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        working_dir: Option<&str>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let request = ClientRequest::CallToolRequest(CallToolRequest {
            params: CallToolRequestParams {
                meta: None,
                task: None,
                name: name.to_string().into(),
                arguments,
            },
            method: Default::default(),
            extensions: Default::default(),
        });

        let result = self
            .send_request_with_context(session_id, working_dir, request, cancel_token)
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
                ClientRequest::ListPromptsRequest(ListPromptsRequest {
                    params: Some(PaginatedRequestParams { meta: None, cursor }),
                    method: Default::default(),
                    extensions: Default::default(),
                }),
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
        let res = self
            .send_request_with_context(
                session_id,
                None,
                ClientRequest::GetPromptRequest(GetPromptRequest {
                    params: GetPromptRequestParams {
                        meta: None,
                        name: name.to_string(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: Default::default(),
                }),
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
}

/// Injects the given session_id and working_dir into Extensions._meta.
/// None (or empty) removes any existing values.
fn inject_session_context_into_extensions(
    mut extensions: Extensions,
    session_id: Option<&str>,
    working_dir: Option<&str>,
) -> Extensions {
    let session_id = session_id.filter(|id| !id.is_empty());
    let working_dir = working_dir.filter(|dir| !dir.is_empty());
    let mut meta_map = extensions
        .get::<Meta>()
        .map(|meta| meta.0.clone())
        .unwrap_or_default();

    // JsonObject is case-sensitive, so we use retain for case-insensitive removal
    meta_map.retain(|k, _| {
        !k.eq_ignore_ascii_case(SESSION_ID_HEADER) && !k.eq_ignore_ascii_case(WORKING_DIR_HEADER)
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

    extensions.insert(Meta(meta_map));
    extensions
}

fn inject_session_context_into_request(
    request: ClientRequest,
    session_id: Option<&str>,
    working_dir: Option<&str>,
) -> ClientRequest {
    match request {
        ClientRequest::ListResourcesRequest(mut req) => {
            req.extensions =
                inject_session_context_into_extensions(req.extensions, session_id, working_dir);
            ClientRequest::ListResourcesRequest(req)
        }
        ClientRequest::ReadResourceRequest(mut req) => {
            req.extensions =
                inject_session_context_into_extensions(req.extensions, session_id, working_dir);
            ClientRequest::ReadResourceRequest(req)
        }
        ClientRequest::ListToolsRequest(mut req) => {
            req.extensions =
                inject_session_context_into_extensions(req.extensions, session_id, working_dir);
            ClientRequest::ListToolsRequest(req)
        }
        ClientRequest::CallToolRequest(mut req) => {
            req.extensions =
                inject_session_context_into_extensions(req.extensions, session_id, working_dir);
            ClientRequest::CallToolRequest(req)
        }
        ClientRequest::ListPromptsRequest(mut req) => {
            req.extensions =
                inject_session_context_into_extensions(req.extensions, session_id, working_dir);
            ClientRequest::ListPromptsRequest(req)
        }
        ClientRequest::GetPromptRequest(mut req) => {
            req.extensions =
                inject_session_context_into_extensions(req.extensions, session_id, working_dir);
            ClientRequest::GetPromptRequest(req)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use test_case::test_case;

    fn new_client() -> GooseClient {
        GooseClient::new(Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(None)))
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
        ClientRequest::ListResourcesRequest(ListResourcesRequest {
            params: Some(PaginatedRequestParams {
                meta: None,
                cursor: None,
            }),
            method: Default::default(),
            extensions,
        })
    }

    fn read_resource_request(extensions: Extensions) -> ClientRequest {
        ClientRequest::ReadResourceRequest(ReadResourceRequest {
            params: ReadResourceRequestParams {
                meta: None,
                uri: "test://resource".to_string(),
            },
            method: Default::default(),
            extensions,
        })
    }

    fn list_tools_request(extensions: Extensions) -> ClientRequest {
        ClientRequest::ListToolsRequest(ListToolsRequest {
            params: Some(PaginatedRequestParams {
                meta: None,
                cursor: None,
            }),
            method: Default::default(),
            extensions,
        })
    }

    fn call_tool_request(extensions: Extensions) -> ClientRequest {
        ClientRequest::CallToolRequest(CallToolRequest {
            params: CallToolRequestParams {
                meta: None,
                task: None,
                name: "tool".to_string().into(),
                arguments: None,
            },
            method: Default::default(),
            extensions,
        })
    }

    fn list_prompts_request(extensions: Extensions) -> ClientRequest {
        ClientRequest::ListPromptsRequest(ListPromptsRequest {
            params: Some(PaginatedRequestParams {
                meta: None,
                cursor: None,
            }),
            method: Default::default(),
            extensions,
        })
    }

    fn get_prompt_request(extensions: Extensions) -> ClientRequest {
        ClientRequest::GetPromptRequest(GetPromptRequest {
            params: GetPromptRequestParams {
                meta: None,
                name: "prompt".to_string(),
                arguments: None,
            },
            method: Default::default(),
            extensions,
        })
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
            let client = new_client();
            if let Some(session_id) = current_session {
                let mut slot = client.current_session_id.lock().await;
                *slot = Some(session_id.to_string());
            }

            let extensions =
                inject_session_context_into_extensions(Extensions::new(), ext_session, None);

            let resolved = client.resolve_session_id(&extensions).await;

            let expected = expected.map(str::to_string);
            assert_eq!(resolved, expected);
        });
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
        let request = inject_session_context_into_request(request, Some(session_id), None);
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
    }

    #[test]
    fn test_session_id_in_mcp_meta() {
        let session_id = "test-session-789";
        let extensions =
            inject_session_context_into_extensions(Default::default(), Some(session_id), None);
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

        let extensions = inject_session_context_into_extensions(extensions, session_id, None);
        let mcp_meta = extensions.get::<Meta>().unwrap();

        assert_eq!(&mcp_meta.0, expected_meta.as_object().unwrap());
    }

    #[test]
    fn test_client_info_advertises_mcp_apps_ui_extension() {
        let client = new_client();
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
}
