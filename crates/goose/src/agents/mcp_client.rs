use rmcp::model::JsonObject;
/// MCP client implementation for Goose
use rmcp::{
    model::{
        CallToolRequest, CallToolRequestParam, CallToolResult, CancelledNotification,
        CancelledNotificationMethod, CancelledNotificationParam, ClientCapabilities, ClientInfo,
        ClientRequest, Extensions, GetPromptRequest, GetPromptRequestParam, GetPromptResult,
        Implementation, InitializeResult, ListPromptsRequest, ListPromptsResult,
        ListResourcesRequest, ListResourcesResult, ListToolsRequest, ListToolsResult,
        LoggingMessageNotification, LoggingMessageNotificationMethod, PaginatedRequestParam,
        ProgressNotification, ProgressNotificationMethod, ProtocolVersion, ReadResourceRequest,
        ReadResourceRequestParam, ReadResourceResult, RequestId, ServerNotification, ServerResult,
    },
    service::{
        ClientInitializeError, PeerRequestOptions, RequestHandle, RunningService, ServiceRole,
    },
    transport::IntoTransport,
    ClientHandler, Peer, RoleClient, ServiceError, ServiceExt,
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

    fn get_info(&self) -> ClientInfo {
        ClientInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ClientCapabilities::builder().build(),
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

/// Helper function to create Extensions with injected trace context
fn create_extensions_with_trace() -> Extensions {
    let mut extensions = Extensions::new();
    let meta = crate::tracing::inject_trace_context();
    extensions.insert(meta);
    extensions
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
            extensions: create_extensions_with_trace(),
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
                    extensions: create_extensions_with_trace(),
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
                    extensions: create_extensions_with_trace(),
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
                    extensions: create_extensions_with_trace(),
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
                    extensions: create_extensions_with_trace(),
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
                    extensions: create_extensions_with_trace(),
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
                    extensions: create_extensions_with_trace(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::{trace::TracerProvider, KeyValue};
    use opentelemetry_sdk::{
        trace::{RandomIdGenerator, Sampler, TracerProvider as SdkTracerProvider},
        Resource,
    };
    use rmcp::model::Meta;
    use tracing::instrument;
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    #[test]
    fn test_create_extensions_with_trace_has_meta() {
        // Set up OpenTelemetry tracer
        let resource = Resource::new(vec![KeyValue::new("service.name", "test")]);
        let provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let tracer = provider.tracer("test");

        // Set up tracing subscriber with OpenTelemetry layer
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry_layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        // Set W3C TraceContext propagator
        crate::tracing::init_otel_propagation();

        // Create a test function with a span
        #[instrument]
        fn create_extensions_in_span() -> Extensions {
            create_extensions_with_trace()
        }

        // Call within a span
        let extensions = create_extensions_in_span();

        // Verify that Meta was inserted into extensions
        let meta = extensions.get::<Meta>();
        assert!(meta.is_some(), "Extensions should contain Meta");

        // Verify Meta contains trace context fields
        let meta = meta.unwrap();
        assert!(
            !meta.0.is_empty(),
            "Meta should contain trace context fields"
        );

        // If there's a traceparent field, verify it has the correct format
        if let Some(traceparent_value) = meta.0.get("traceparent") {
            if let Some(traceparent) = traceparent_value.as_str() {
                assert!(
                    traceparent.starts_with("00-"),
                    "traceparent should start with version 00"
                );
                let parts: Vec<&str> = traceparent.split('-').collect();
                assert_eq!(parts.len(), 4, "traceparent should have 4 parts");
                assert_eq!(parts[0], "00", "version should be 00");
                assert_eq!(parts[1].len(), 32, "trace-id should be 32 hex chars");
                assert_eq!(parts[2].len(), 16, "span-id should be 16 hex chars");
                assert_eq!(parts[3].len(), 2, "flags should be 2 hex chars");
            }
        }
    }

    #[test]
    fn test_create_extensions_with_trace_without_span() {
        // Call without an active span
        let extensions = create_extensions_with_trace();

        // Extensions should still be created, even if Meta is empty
        let meta = extensions.get::<Meta>();
        assert!(
            meta.is_some(),
            "Extensions should contain Meta even without span"
        );
    }

    #[test]
    fn test_create_extensions_with_trace_returns_unique_extensions() {
        // Set up OpenTelemetry tracer
        let resource = Resource::new(vec![KeyValue::new("service.name", "test")]);
        let provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn)
            .build();
        let tracer = provider.tracer("test");

        // Set up tracing subscriber with OpenTelemetry layer
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry_layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        // Set W3C TraceContext propagator
        crate::tracing::init_otel_propagation();

        // Create a test function with a span
        #[instrument]
        fn create_two_extensions_in_span() -> (Extensions, Extensions) {
            let ext1 = create_extensions_with_trace();
            let ext2 = create_extensions_with_trace();
            (ext1, ext2)
        }

        // Call within a span
        let (ext1, ext2) = create_two_extensions_in_span();

        // Both should have Meta
        assert!(ext1.get::<Meta>().is_some());
        assert!(ext2.get::<Meta>().is_some());

        // Both should have the same trace context (same span)
        let meta1 = ext1.get::<Meta>().unwrap();
        let meta2 = ext2.get::<Meta>().unwrap();

        if let (Some(tp1), Some(tp2)) = (meta1.0.get("traceparent"), meta2.0.get("traceparent")) {
            // Both should have traceparent
            assert_eq!(
                tp1, tp2,
                "Both extensions should have the same traceparent within the same span"
            );
        }
    }
}
