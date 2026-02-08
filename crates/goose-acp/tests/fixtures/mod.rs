#![recursion_limit = "256"]
#![allow(unused_attributes)]

use async_trait::async_trait;
use fs_err as fs;
use goose::builtin_extension::register_builtin_extensions;
use goose::config::{GooseMode, PermissionManager};
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::providers::base::Provider;
use goose::providers::openai::OpenAiProvider;
use goose::providers::provider_registry::ProviderConstructor;
use goose::session_context::SESSION_ID_HEADER;
use goose_acp::server::{serve, GooseAcpAgent};
use rmcp::model::{ClientNotification, ClientRequest, Meta, ServerResult};
use rmcp::service::{NotificationContext, RequestContext, ServiceRole};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, tool, tool_handler, tool_router,
    ErrorData as McpError, RoleServer, ServerHandler, Service,
};
use sacp::schema::{
    McpServer, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, ToolCallStatus,
};
use std::collections::VecDeque;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const FAKE_CODE: &str = "test-uuid-12345-67890";

const NOT_YET_SET: &str = "session-id-not-yet-set";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    AllowAlways,
    AllowOnce,
    RejectOnce,
    RejectAlways,
    Cancel,
}

#[derive(Default)]
pub struct PermissionMapping;

pub fn map_permission_response(
    _mapping: &PermissionMapping,
    req: &RequestPermissionRequest,
    decision: PermissionDecision,
) -> RequestPermissionResponse {
    let outcome = match decision {
        PermissionDecision::Cancel => RequestPermissionOutcome::Cancelled,
        PermissionDecision::AllowAlways => select_option(req, PermissionOptionKind::AllowAlways),
        PermissionDecision::AllowOnce => select_option(req, PermissionOptionKind::AllowOnce),
        PermissionDecision::RejectOnce => select_option(req, PermissionOptionKind::RejectOnce),
        PermissionDecision::RejectAlways => select_option(req, PermissionOptionKind::RejectAlways),
    };

    RequestPermissionResponse::new(outcome)
}

fn select_option(
    req: &RequestPermissionRequest,
    kind: PermissionOptionKind,
) -> RequestPermissionOutcome {
    req.options
        .iter()
        .find(|opt| opt.kind == kind)
        .map(|opt| {
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                opt.option_id.clone(),
            ))
        })
        .unwrap_or(RequestPermissionOutcome::Cancelled)
}

#[derive(Clone)]
pub struct ExpectedSessionId {
    value: Arc<Mutex<String>>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Default for ExpectedSessionId {
    fn default() -> Self {
        Self {
            value: Arc::new(Mutex::new(NOT_YET_SET.to_string())),
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ExpectedSessionId {
    pub fn set(&self, id: &sacp::schema::SessionId) {
        *self.value.lock().unwrap() = id.0.to_string();
    }

    pub fn validate(&self, actual: Option<&str>) -> Result<(), String> {
        let expected = self.value.lock().unwrap();

        let err = match actual {
            Some(act) if act == *expected => None,
            _ => Some(format!(
                "{} mismatch: expected '{}', got {:?}",
                SESSION_ID_HEADER, expected, actual
            )),
        };
        match err {
            Some(e) => {
                self.errors.lock().unwrap().push(e.clone());
                Err(e)
            }
            None => Ok(()),
        }
    }

    /// Calling this ensures requests have coherent session IDs.
    pub fn assert_matches(&self, actual: &str) {
        let result = self.validate(Some(actual));
        assert!(result.is_ok(), "{}", result.unwrap_err());
        let e = self.errors.lock().unwrap();
        assert!(e.is_empty(), "Session ID validation errors: {:?}", *e);
    }
}

pub struct OpenAiFixture {
    _server: MockServer,
    base_url: String,
    exchanges: Vec<(String, &'static str)>,
    queue: Arc<Mutex<VecDeque<(String, &'static str)>>>,
}

impl OpenAiFixture {
    /// Mock OpenAI streaming endpoint. Exchanges are (pattern, response) pairs.
    /// On mismatch, returns 417 of the diff in OpenAI error format.
    pub async fn new(
        exchanges: Vec<(String, &'static str)>,
        expected_session_id: ExpectedSessionId,
    ) -> Self {
        let mock_server = MockServer::start().await;
        let queue = Arc::new(Mutex::new(VecDeque::from(exchanges.clone())));

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with({
                let queue = queue.clone();
                let expected_session_id = expected_session_id.clone();
                move |req: &wiremock::Request| {
                    let body = std::str::from_utf8(&req.body).unwrap_or("");

                    // Validate session ID header
                    let actual = req
                        .headers
                        .get(SESSION_ID_HEADER)
                        .and_then(|v| v.to_str().ok());
                    if let Err(e) = expected_session_id.validate(actual) {
                        return ResponseTemplate::new(417)
                            .insert_header("content-type", "application/json")
                            .set_body_json(serde_json::json!({"error": {"message": e}}));
                    }

                    // See if the actual request matches the expected pattern
                    let mut q = queue.lock().unwrap();
                    let (expected_body, response) = q.front().cloned().unwrap_or_default();
                    if !expected_body.is_empty() && body.contains(&expected_body) {
                        q.pop_front();
                        return ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(response);
                    }
                    drop(q);

                    // If there was no body, the request was unexpected. Otherwise, it is a mismatch.
                    let message = if expected_body.is_empty() {
                        format!("Unexpected request:\n  {}", body)
                    } else {
                        format!(
                            "Expected body to contain:\n  {}\n\nActual body:\n  {}",
                            expected_body, body
                        )
                    };
                    // Use OpenAI's error response schema so the provider will pass the error through.
                    ResponseTemplate::new(417)
                        .insert_header("content-type", "application/json")
                        .set_body_json(serde_json::json!({"error": {"message": message}}))
                }
            })
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        Self {
            _server: mock_server,
            base_url,
            exchanges,
            queue,
        }
    }

    pub fn uri(&self) -> &str {
        &self.base_url
    }

    pub fn reset(&self) {
        let mut queue = self.queue.lock().unwrap();
        *queue = VecDeque::from(self.exchanges.clone());
    }
}

#[derive(Clone)]
struct Lookup {
    tool_router: ToolRouter<Lookup>,
}

impl Default for Lookup {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl Lookup {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the code")]
    fn get_code(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(FAKE_CODE)]))
    }
}

#[tool_handler]
impl ServerHandler for Lookup {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "lookup".into(),
                version: "1.0.0".into(),
                ..Default::default()
            },
            instructions: Some("Lookup server with get_code tool.".into()),
        }
    }
}

trait HasMeta {
    fn meta(&self) -> &Meta;
}

impl<R: ServiceRole> HasMeta for RequestContext<R> {
    fn meta(&self) -> &Meta {
        &self.meta
    }
}

impl<R: ServiceRole> HasMeta for NotificationContext<R> {
    fn meta(&self) -> &Meta {
        &self.meta
    }
}

struct ValidatingService<S> {
    inner: S,
    expected_session_id: ExpectedSessionId,
}

impl<S> ValidatingService<S> {
    fn new(inner: S, expected_session_id: ExpectedSessionId) -> Self {
        Self {
            inner,
            expected_session_id,
        }
    }

    fn validate<C: HasMeta>(&self, context: &C) -> Result<(), McpError> {
        let actual = context
            .meta()
            .0
            .get(SESSION_ID_HEADER)
            .and_then(|v| v.as_str());
        self.expected_session_id
            .validate(actual)
            .map_err(|e| McpError::new(ErrorCode::INVALID_REQUEST, e, None))
    }
}

impl<S: Service<RoleServer>> Service<RoleServer> for ValidatingService<S> {
    async fn handle_request(
        &self,
        request: ClientRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, McpError> {
        if !matches!(request, ClientRequest::InitializeRequest(_)) {
            self.validate(&context)?;
        }
        self.inner.handle_request(request, context).await
    }

    async fn handle_notification(
        &self,
        notification: ClientNotification,
        context: NotificationContext<RoleServer>,
    ) -> Result<(), McpError> {
        if !matches!(notification, ClientNotification::InitializedNotification(_)) {
            self.validate(&context).ok();
        }
        self.inner.handle_notification(notification, context).await
    }

    fn get_info(&self) -> ServerInfo {
        self.inner.get_info()
    }
}

pub struct McpFixture {
    pub url: String,
    // Keep the server alive in tests; underscore avoids unused field warnings.
    _handle: JoinHandle<()>,
}

impl McpFixture {
    pub async fn new(expected_session_id: ExpectedSessionId) -> Self {
        let service = StreamableHttpService::new(
            {
                let expected_session_id = expected_session_id.clone();
                move || {
                    Ok(ValidatingService::new(
                        Lookup::new(),
                        expected_session_id.clone(),
                    ))
                }
            },
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default(),
        );
        let router = axum::Router::new().nest_service("/mcp", service);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}/mcp");

        let handle = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        Self {
            url,
            _handle: handle,
        }
    }
}

pub type DuplexTransport = sacp::ByteStreams<
    tokio_util::compat::Compat<tokio::io::DuplexStream>,
    tokio_util::compat::Compat<tokio::io::DuplexStream>,
>;

/// Wires up duplex streams, spawns `serve` for the given agent, and returns
/// a ready-to-use sacp transport plus the server handle.
#[allow(dead_code)]
pub async fn serve_agent_in_process(
    agent: Arc<GooseAcpAgent>,
) -> (DuplexTransport, JoinHandle<()>) {
    let (client_read, server_write) = tokio::io::duplex(64 * 1024);
    let (server_read, client_write) = tokio::io::duplex(64 * 1024);

    let handle = tokio::spawn(async move {
        if let Err(e) = serve(agent, server_read.compat(), server_write.compat_write()).await {
            tracing::error!("ACP server error: {e}");
        }
    });

    let transport = sacp::ByteStreams::new(client_write.compat_write(), client_read.compat());
    (transport, handle)
}

#[allow(dead_code)]
pub async fn spawn_acp_server_in_process(
    openai_base_url: &str,
    builtins: &[String],
    data_root: &Path,
    goose_mode: GooseMode,
) -> (DuplexTransport, JoinHandle<()>, Arc<PermissionManager>) {
    fs::create_dir_all(data_root).unwrap();
    // ensure_provider reads the model from config lazily, so tests need a config.yaml.
    let config_path = data_root.join(goose::config::base::CONFIG_YAML_NAME);
    if !config_path.exists() {
        fs::write(&config_path, "GOOSE_MODEL: gpt-5-nano\n").unwrap();
    }
    let base_url = openai_base_url.to_string();
    let provider_factory: ProviderConstructor = Arc::new(move |model_config| {
        let base_url = base_url.clone();
        Box::pin(async move {
            let api_client =
                ApiClient::new(base_url, AuthMethod::BearerToken("test-key".to_string())).unwrap();
            let provider: Arc<dyn Provider> =
                Arc::new(OpenAiProvider::new(api_client, model_config));
            Ok(provider)
        })
    });

    let agent = Arc::new(
        GooseAcpAgent::new(
            provider_factory,
            builtins.to_vec(),
            data_root.to_path_buf(),
            data_root.to_path_buf(),
            goose_mode,
            true,
        )
        .await
        .unwrap(),
    );
    let permission_manager = agent.permission_manager();
    let (transport, handle) = serve_agent_in_process(agent).await;

    (transport, handle, permission_manager)
}

pub struct TestOutput {
    pub text: String,
    pub tool_status: Option<ToolCallStatus>,
}

pub struct TestSessionConfig {
    pub mcp_servers: Vec<McpServer>,
    pub builtins: Vec<String>,
    pub goose_mode: GooseMode,
    pub data_root: PathBuf,
}

impl Default for TestSessionConfig {
    fn default() -> Self {
        Self {
            mcp_servers: Vec::new(),
            builtins: Vec::new(),
            goose_mode: GooseMode::Auto,
            data_root: PathBuf::new(),
        }
    }
}

#[async_trait]
pub trait Session {
    async fn new(config: TestSessionConfig, openai: OpenAiFixture) -> Self
    where
        Self: Sized;
    fn id(&self) -> &sacp::schema::SessionId;
    fn reset_openai(&self);
    fn reset_permissions(&self);
    async fn prompt(&mut self, text: &str, decision: PermissionDecision) -> TestOutput;
}

#[allow(dead_code)]
pub fn run_test<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    register_builtin_extensions(goose_mcp::BUILTIN_EXTENSIONS.clone());

    let handle = std::thread::Builder::new()
        .name("acp-test".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(fut);
        })
        .unwrap();
    if let Err(err) = handle.join() {
        // Re-raise the original panic so the test shows the real failure message.
        std::panic::resume_unwind(err);
    }
}

/// Connects to the given agent via in-process duplex streams, sends an
/// `InitializeRequest`, and returns the response.
#[allow(dead_code)]
pub async fn initialize_agent(agent: Arc<GooseAcpAgent>) -> sacp::schema::InitializeResponse {
    let (transport, _handle) = serve_agent_in_process(agent).await;
    sacp::ClientToAgent::builder()
        .connect_to(transport)
        .unwrap()
        .run_until(|cx: sacp::JrConnectionCx<sacp::ClientToAgent>| async move {
            let resp = cx
                .send_request(sacp::schema::InitializeRequest::new(
                    sacp::schema::ProtocolVersion::LATEST,
                ))
                .block_task()
                .await
                .unwrap();
            Ok::<_, sacp::Error>(resp)
        })
        .await
        .unwrap()
}

pub mod server;
