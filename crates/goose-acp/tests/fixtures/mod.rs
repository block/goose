use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};
use async_trait::async_trait;
use fs_err as fs;
use goose::config::{GooseMode, PermissionManager};
use goose::model::ModelConfig;
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::providers::openai::OpenAiProvider;
use goose::session_context::SESSION_ID_HEADER;
use goose_acp::server::{serve, AcpServerConfig, GooseAcpAgent};
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

    /// Calling this ensures incidental requests that might error asynchronously, such as
    /// session rename have coherent session IDs.
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
                    let body = String::from_utf8_lossy(&req.body);

                    let actual = req
                        .headers
                        .get(SESSION_ID_HEADER)
                        .and_then(|v| v.to_str().ok());
                    if let Err(e) = expected_session_id.validate(actual) {
                        return ResponseTemplate::new(417)
                            .insert_header("content-type", "application/json")
                            .set_body_json(serde_json::json!({"error": {"message": e}}));
                    }

                    // Session rename (async, unpredictable order) - canned response
                    if body.contains("Reply with only a description in four words or less") {
                        return ResponseTemplate::new(200)
                            .insert_header("content-type", "application/json")
                            .set_body_string(include_str!(
                                "../test_data/openai_session_description.json"
                            ));
                    }

                    let (expected_body, response) = {
                        let mut q = queue.lock().unwrap();
                        q.pop_front().unwrap_or_default()
                    };

                    if body.contains(&expected_body) && !expected_body.is_empty() {
                        return ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(response);
                    }

                    // Coerce non-json to allow a uniform JSON diff error response.
                    let exp = serde_json::from_str(&expected_body)
                        .unwrap_or(serde_json::Value::String(expected_body.clone()));
                    let act = serde_json::from_str(&body)
                        .unwrap_or(serde_json::Value::String(body.to_string()));
                    let diff =
                        assert_json_matches_no_panic(&exp, &act, Config::new(CompareMode::Strict))
                            .unwrap_err();
                    ResponseTemplate::new(417)
                        .insert_header("content-type", "application/json")
                        .set_body_json(serde_json::json!({"error": {"message": diff}}))
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

#[allow(dead_code)]
pub async fn spawn_acp_server_in_process(
    openai_base_url: &str,
    builtins: &[String],
    data_root: &Path,
    goose_mode: GooseMode,
) -> (
    tokio::io::DuplexStream,
    tokio::io::DuplexStream,
    JoinHandle<()>,
    Arc<PermissionManager>,
) {
    fs::create_dir_all(data_root).unwrap();
    let api_client = ApiClient::new(
        openai_base_url.to_string(),
        AuthMethod::BearerToken("test-key".to_string()),
    )
    .unwrap();
    let model_config = ModelConfig::new("gpt-5-nano").unwrap();
    let provider = OpenAiProvider::new(api_client, model_config);

    let config = AcpServerConfig {
        provider: Arc::new(provider),
        builtins: builtins.to_vec(),
        data_dir: data_root.to_path_buf(),
        config_dir: data_root.to_path_buf(),
        goose_mode,
    };

    let (client_read, server_write) = tokio::io::duplex(64 * 1024);
    let (server_read, client_write) = tokio::io::duplex(64 * 1024);

    let agent = Arc::new(GooseAcpAgent::with_config(config).await.unwrap());
    let permission_manager = agent.permission_manager();
    let handle = tokio::spawn(async move {
        if let Err(e) = serve(agent, server_read.compat(), server_write.compat_write()).await {
            tracing::error!("ACP server error: {e}");
        }
    });

    (client_read, client_write, handle, permission_manager)
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
    handle.join().unwrap();
}

pub mod server;
