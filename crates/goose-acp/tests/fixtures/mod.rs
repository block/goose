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
use goose_test_support::{ExpectedSessionId, TEST_MODEL};
use sacp::schema::{
    McpServer, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionModelState, ToolCallStatus,
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

pub struct OpenAiFixture {
    _server: MockServer,
    base_url: String,
    exchanges: Vec<(String, String)>,
    queue: Arc<Mutex<VecDeque<(String, String)>>>,
}

impl OpenAiFixture {
    /// Mock OpenAI streaming endpoint. Exchanges are (pattern, response) pairs.
    /// On mismatch, returns 417 of the diff in OpenAI error format.
    pub async fn new(
        exchanges: Vec<(String, &'static str)>,
        expected_session_id: ExpectedSessionId,
    ) -> Self {
        let exchanges = exchanges
            .into_iter()
            .map(|(pattern, response)| (pattern, response.to_string()))
            .collect();
        Self::new_dynamic(exchanges, expected_session_id).await
    }

    pub async fn new_dynamic(
        exchanges: Vec<(String, String)>,
        expected_session_id: ExpectedSessionId,
    ) -> Self {
        let mock_server = MockServer::start().await;
        let queue = Arc::new(Mutex::new(VecDeque::from(exchanges.clone())));

        // Always return the models when asked, as there is no POST data to validate
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string(include_str!("../test_data/openai_models.json")),
            )
            .mount(&mock_server)
            .await;

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
    provider_factory: Option<ProviderConstructor>,
) -> (DuplexTransport, JoinHandle<()>, Arc<PermissionManager>) {
    fs::create_dir_all(data_root).unwrap();
    let config_path = data_root.join(goose::config::base::CONFIG_YAML_NAME);
    if !config_path.exists() {
        fs::write(
            &config_path,
            format!("GOOSE_MODEL: {TEST_MODEL}\nGOOSE_PROVIDER: openai\n"),
        )
        .unwrap();
    }
    let provider_factory = provider_factory.unwrap_or_else(|| {
        let base_url = openai_base_url.to_string();
        Arc::new(move |model_config, _extensions| {
            let base_url = base_url.clone();
            Box::pin(async move {
                let api_client =
                    ApiClient::new(base_url, AuthMethod::BearerToken("test-key".to_string()))
                        .unwrap();
                let provider: Arc<dyn Provider> =
                    Arc::new(OpenAiProvider::new(api_client, model_config));
                Ok(provider)
            })
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

pub struct TestConnectionConfig {
    pub mcp_servers: Vec<McpServer>,
    pub builtins: Vec<String>,
    pub goose_mode: GooseMode,
    pub fs_read_text_file: bool,
    pub fs_write_text_file: bool,
    pub data_root: PathBuf,
    pub provider_factory: Option<ProviderConstructor>,
}

impl Default for TestConnectionConfig {
    fn default() -> Self {
        Self {
            mcp_servers: Vec::new(),
            builtins: Vec::new(),
            goose_mode: GooseMode::Auto,
            fs_read_text_file: false,
            fs_write_text_file: false,
            data_root: PathBuf::new(),
            provider_factory: None,
        }
    }
}

#[async_trait]
pub trait Connection: Sized {
    type Session: Session;

    async fn new(config: TestConnectionConfig, openai: OpenAiFixture) -> Self;
    async fn new_session(&mut self) -> (Self::Session, Option<SessionModelState>);
    async fn load_session(
        &mut self,
        session_id: &str,
    ) -> (Self::Session, Option<SessionModelState>);
    fn reset_openai(&self);
    fn reset_permissions(&self);
}

#[async_trait]
pub trait Session {
    fn session_id(&self) -> &sacp::schema::SessionId;
    async fn prompt(&mut self, text: &str, decision: PermissionDecision) -> TestOutput;
    async fn set_model(&self, model_id: &str);
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
