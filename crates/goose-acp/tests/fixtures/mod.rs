#![recursion_limit = "256"]
#![allow(unused_attributes)]

use async_trait::async_trait;
use fs_err as fs;
use goose::builtin_extension::register_builtin_extensions;
use goose::config::{GooseMode, PermissionManager};
use goose::model::ModelConfig;
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::providers::openai::OpenAiProvider;
use goose::session_context::SESSION_ID_HEADER;
use goose_acp::server::{serve, AcpServerConfig, GooseAcpAgent};
use goose_test_support::ExpectedSessionId;
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

                    // Session rename (async, unpredictable order) - canned response
                    if body.contains("Reply with only a description in four words or less") {
                        return ResponseTemplate::new(200)
                            .insert_header("content-type", "application/json")
                            .set_body_string(include_str!(
                                "../test_data/openai_session_description.json"
                            ));
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

pub mod server;
