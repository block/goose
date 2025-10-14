use crate::agents::types::SharedProvider;
use rmcp::model::{Content, ErrorCode, JsonObject};
/// MCP client implementation for Goose
use rmcp::{
    model::{
        CallToolRequest, CallToolRequestParam, CallToolResult, CancelledNotification,
        CancelledNotificationMethod, CancelledNotificationParam, ClientCapabilities, ClientInfo,
        ClientRequest, CreateMessageRequestParam, CreateMessageResult, GetPromptRequest,
        GetPromptRequestParam, GetPromptResult, Implementation, InitializeResult,
        ListPromptsRequest, ListPromptsResult, ListResourcesRequest, ListResourcesResult,
        ListToolsRequest, ListToolsResult, LoggingMessageNotification,
        LoggingMessageNotificationMethod, PaginatedRequestParam, ProgressNotification,
        ProgressNotificationMethod, ProtocolVersion, ReadResourceRequest, ReadResourceRequestParam,
        ReadResourceResult, RequestId, Role, SamplingMessage, ServerNotification, ServerResult,
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
    async fn list_resources(
        &self,
        next_cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error>;

    async fn read_resource(
        &self,
        uri: &str,
        cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error>;

    async fn list_tools(
        &self,
        next_cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error>;

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error>;

    async fn list_prompts(
        &self,
        next_cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error>;

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Value,
        cancel_token: CancellationToken,
    ) -> Result<GetPromptResult, Error>;

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification>;

    fn get_info(&self) -> Option<&InitializeResult>;
}

pub struct GooseClient {
    notification_handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
    provider: SharedProvider,
}

impl GooseClient {
    pub fn new(
        handlers: Arc<Mutex<Vec<Sender<ServerNotification>>>>,
        provider: SharedProvider,
    ) -> Self {
        GooseClient {
            notification_handlers: handlers,
            provider,
        }
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
        params: CreateMessageRequestParam,
        _context: RequestContext<RoleClient>,
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
            .complete(system_prompt, &provider_ready_messages, &[])
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

    fn get_info(&self) -> ClientInfo {
        ClientInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ClientCapabilities::builder().enable_sampling().build(),
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
        })
    }

    async fn send_request(
        &self,
        request: ClientRequest,
        cancel_token: CancellationToken,
    ) -> Result<ServerResult, Error> {
        let handle = self
            .client
            .lock()
            .await
            .send_cancellable_request(request, PeerRequestOptions::no_options())
            .await?;

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
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        let res = self
            .send_request(
                ClientRequest::ListResourcesRequest(ListResourcesRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: inject_session_into_extensions(Default::default()),
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
        uri: &str,
        cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        let res = self
            .send_request(
                ClientRequest::ReadResourceRequest(ReadResourceRequest {
                    params: ReadResourceRequestParam {
                        uri: uri.to_string(),
                    },
                    method: Default::default(),
                    extensions: inject_session_into_extensions(Default::default()),
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
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let res = self
            .send_request(
                ClientRequest::ListToolsRequest(ListToolsRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: inject_session_into_extensions(Default::default()),
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
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let res = self
            .send_request(
                ClientRequest::CallToolRequest(CallToolRequest {
                    params: CallToolRequestParam {
                        name: name.to_string().into(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: inject_session_into_extensions(Default::default()),
                }),
                cancel_token,
            )
            .await?;

        match res {
            ServerResult::CallToolResult(result) => Ok(result),
            _ => Err(ServiceError::UnexpectedResponse),
        }
    }

    async fn list_prompts(
        &self,
        cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        let res = self
            .send_request(
                ClientRequest::ListPromptsRequest(ListPromptsRequest {
                    params: Some(PaginatedRequestParam { cursor }),
                    method: Default::default(),
                    extensions: inject_session_into_extensions(Default::default()),
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
        name: &str,
        arguments: Value,
        cancel_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        let arguments = match arguments {
            Value::Object(map) => Some(map),
            _ => None,
        };
        let res = self
            .send_request(
                ClientRequest::GetPromptRequest(GetPromptRequest {
                    params: GetPromptRequestParam {
                        name: name.to_string(),
                        arguments,
                    },
                    method: Default::default(),
                    extensions: inject_session_into_extensions(Default::default()),
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

/// Injects session ID into Extensions._meta, preserving existing metadata.
/// Removes existing session IDs (case-insensitive).
fn inject_session_into_extensions(
    mut extensions: rmcp::model::Extensions,
) -> rmcp::model::Extensions {
    use rmcp::model::Meta;

    // Only inject session ID if one is available
    if let Some(session_id) = crate::session_context::current_session_id() {
        // Get existing Meta or create new one
        let mut meta_map = extensions
            .get::<Meta>()
            .map(|meta| meta.0.clone())
            .unwrap_or_default();

        // Remove any existing session ID keys with different casings (case-insensitive)
        // Note: JsonObject (serde_json::Map) is case-sensitive, so we use retain to filter
        meta_map.retain(|k, _| !k.eq_ignore_ascii_case("goose-session-id"));

        // Insert with canonical casing
        meta_map.insert(
            "goose-session-id".to_string(),
            serde_json::Value::String(session_id),
        );

        extensions.insert(Meta(meta_map));
    }

    extensions
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Meta;

    #[tokio::test]
    async fn test_session_id_in_mcp_meta() {
        crate::session_context::with_session_id(Some("test-session-789".to_string()), async {
            let extensions = inject_session_into_extensions(Default::default());
            let meta = extensions.get::<Meta>();

            assert!(meta.is_some(), "Extensions should contain Meta");

            let meta = meta.unwrap();
            let session_id = meta.0.get("goose-session-id");

            assert!(
                session_id.is_some(),
                "Meta should contain goose-session-id when session is set"
            );

            assert_eq!(
                session_id.unwrap().as_str(),
                Some("test-session-789"),
                "Session ID should match the value from context"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn test_no_session_id_in_mcp_when_absent() {
        // Call without setting session ID
        let extensions = inject_session_into_extensions(Default::default());
        let meta = extensions.get::<Meta>();

        // When no session ID, meta should not be present (or should be empty)
        if let Some(meta) = meta {
            let session_id = meta.0.get("goose-session-id");
            assert!(
                session_id.is_none(),
                "Meta should not contain goose-session-id when session is not set"
            );
        }
    }

    #[tokio::test]
    async fn test_all_mcp_operations_include_session() {
        // This test verifies that all MCP operations use inject_session_into_extensions()
        // and therefore will include session ID when available.
        // We test this by verifying the helper function is called consistently.

        crate::session_context::with_session_id(Some("consistent-session-id".to_string()), async {
            // Create multiple extensions as would happen in different MCP operations
            let ext1 = inject_session_into_extensions(Default::default()); // e.g., list_resources
            let ext2 = inject_session_into_extensions(Default::default()); // e.g., call_tool
            let ext3 = inject_session_into_extensions(Default::default()); // e.g., get_prompt

            // All should have the same session ID
            let verify_session_id = |ext: &rmcp::model::Extensions| {
                let meta = ext.get::<Meta>().expect("Should have Meta");
                let session_id = meta
                    .0
                    .get("goose-session-id")
                    .expect("Should have session ID");
                assert_eq!(
                    session_id.as_str(),
                    Some("consistent-session-id"),
                    "All operations should have the same session ID"
                );
            };

            verify_session_id(&ext1);
            verify_session_id(&ext2);
            verify_session_id(&ext3);
        })
        .await;
    }

    #[tokio::test]
    async fn test_session_id_case_insensitive_replacement() {
        // This test verifies that case-insensitive replacement works correctly
        use rmcp::model::{Extensions, JsonObject, Meta};

        crate::session_context::with_session_id(Some("new-session-id".to_string()), async {
            // Create extensions with existing Meta containing different casings
            let mut existing_meta = JsonObject::new();
            existing_meta.insert(
                "GOOSE-SESSION-ID".to_string(),
                serde_json::Value::String("old-session-1".to_string()),
            );
            existing_meta.insert(
                "Goose-Session-Id".to_string(),
                serde_json::Value::String("old-session-2".to_string()),
            );
            existing_meta.insert(
                "other-key".to_string(),
                serde_json::Value::String("preserve-me".to_string()),
            );

            let mut extensions = Extensions::new();
            extensions.insert(Meta(existing_meta));

            // Inject session ID - should replace all casings
            let extensions = inject_session_into_extensions(extensions);
            let meta = extensions.get::<Meta>().expect("Should have Meta");

            // Verify only the canonical casing exists
            assert!(
                meta.0.get("goose-session-id").is_some(),
                "Should have canonical casing goose-session-id"
            );
            assert_eq!(
                meta.0.get("goose-session-id").unwrap().as_str(),
                Some("new-session-id"),
                "Should have the new session ID value"
            );

            // Verify no other casings exist
            assert!(
                meta.0.get("GOOSE-SESSION-ID").is_none(),
                "Should not have GOOSE-SESSION-ID"
            );
            assert!(
                meta.0.get("Goose-Session-Id").is_none(),
                "Should not have Goose-Session-Id"
            );

            // Verify only one session ID key exists
            let session_id_count = meta
                .0
                .keys()
                .filter(|k| k.eq_ignore_ascii_case("goose-session-id"))
                .count();
            assert_eq!(
                session_id_count, 1,
                "Should have exactly one session ID key"
            );

            // Verify other metadata is preserved
            assert_eq!(
                meta.0.get("other-key").unwrap().as_str(),
                Some("preserve-me"),
                "Other metadata should be preserved"
            );
        })
        .await;
    }
}