#![recursion_limit = "256"]
#![allow(unused_attributes)]

use async_trait::async_trait;
use fs_err as fs;
pub use goose::acp::{map_permission_response, PermissionDecision, PermissionMapping};
use goose::builtin_extension::register_builtin_extensions;
use goose::config::{GooseMode, PermissionManager};
use goose::providers::api_client::{ApiClient, AuthMethod as ApiAuthMethod};
use goose::providers::base::Provider;
use goose::providers::openai::OpenAiProvider;
use goose::providers::provider_registry::ProviderConstructor;
use goose::session_context::SESSION_ID_HEADER;
use goose_acp::server::{serve, GooseAcpAgent};
use goose_test_support::{ExpectedSessionId, TEST_MODEL};
use sacp::schema::{
    AuthMethod, CreateTerminalResponse, KillTerminalCommandResponse, McpServer,
    ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalResponse, SessionModeState,
    SessionModelState, TerminalExitStatus, TerminalId, TerminalOutputResponse, ToolCallStatus,
    WaitForTerminalExitResponse, WriteTextFileRequest, WriteTextFileResponse,
};
use std::collections::VecDeque;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
        expected_session_id: Arc<dyn ExpectedSessionId>,
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
    data_root: &std::path::Path,
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
                    ApiClient::new(base_url, ApiAuthMethod::BearerToken("test-key".to_string()))
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

type ReadTextFileHandler =
    Arc<dyn Fn(&ReadTextFileRequest) -> Result<ReadTextFileResponse, String> + Send + Sync>;
type WriteTextFileHandler =
    Arc<dyn Fn(&WriteTextFileRequest) -> Result<WriteTextFileResponse, String> + Send + Sync>;

#[derive(Clone)]
pub struct FsFixture {
    calls: Arc<Mutex<Vec<Result<(), String>>>>,
}

impl FsFixture {
    pub fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn read_handler(&self, expected_path: &str, content: &str) -> ReadTextFileHandler {
        let calls = self.calls.clone();
        let expected_path = expected_path.to_string();
        let content = content.to_string();
        Arc::new(move |req: &ReadTextFileRequest| {
            let path = req.path.to_str().unwrap_or("");
            if path != expected_path {
                let err = format!("expected path {expected_path}, got {path}");
                calls.lock().unwrap().push(Err(err.clone()));
                return Err(err);
            }
            calls.lock().unwrap().push(Ok(()));
            Ok(ReadTextFileResponse::new(&content))
        })
    }

    pub fn write_handler(
        &self,
        expected_path: &str,
        expected_content: &str,
    ) -> WriteTextFileHandler {
        let calls = self.calls.clone();
        let expected_path = expected_path.to_string();
        let expected_content = expected_content.to_string();
        Arc::new(move |req: &WriteTextFileRequest| {
            let path = req.path.to_str().unwrap_or("");
            if path != expected_path {
                let err = format!("expected path {expected_path}, got {path}");
                calls.lock().unwrap().push(Err(err.clone()));
                return Err(err);
            }
            if req.content != expected_content {
                let err = format!("expected content {expected_content}, got {}", req.content);
                calls.lock().unwrap().push(Err(err.clone()));
                return Err(err);
            }
            calls.lock().unwrap().push(Ok(()));
            Ok(WriteTextFileResponse::new())
        })
    }

    pub fn assert_called(&self) {
        let calls = self.calls.lock().unwrap();
        assert!(!calls.is_empty(), "fs handler was never called");
        let errors: Vec<_> = calls.iter().filter_map(|c| c.as_ref().err()).collect();
        assert!(errors.is_empty(), "fs handler errors: {errors:?}");
    }
}

pub struct TerminalFixture {
    calls: Arc<Mutex<Vec<Result<(), String>>>>,
    pub command: String,
    pub terminal_id: TerminalId,
    pub output: String,
    pub exit_code: u32,
}

impl TerminalFixture {
    pub fn new(command: &str, terminal_id: &str, output: &str, exit_code: u32) -> Arc<Self> {
        Arc::new(Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            command: command.to_string(),
            terminal_id: TerminalId::new(terminal_id),
            output: output.to_string(),
            exit_code,
        })
    }

    pub fn record_call(&self, result: Result<(), String>) {
        self.calls.lock().unwrap().push(result);
    }

    pub fn assert_called(&self) {
        let calls = self.calls.lock().unwrap();
        assert!(!calls.is_empty());
        let errors: Vec<_> = calls.iter().filter_map(|c| c.as_ref().err()).collect();
        assert!(errors.is_empty());
    }

    pub fn create_response(&self) -> CreateTerminalResponse {
        CreateTerminalResponse::new(self.terminal_id.clone())
    }

    pub fn wait_response(&self) -> WaitForTerminalExitResponse {
        WaitForTerminalExitResponse::new(TerminalExitStatus::new().exit_code(self.exit_code))
    }

    pub fn output_response(&self) -> TerminalOutputResponse {
        TerminalOutputResponse::new(&self.output, false)
            .exit_status(TerminalExitStatus::new().exit_code(self.exit_code))
    }

    pub fn release_response(&self) -> ReleaseTerminalResponse {
        ReleaseTerminalResponse::new()
    }

    pub fn kill_response(&self) -> KillTerminalCommandResponse {
        KillTerminalCommandResponse::new()
    }
}

pub struct SessionResult<S> {
    pub session: S,
    pub models: Option<SessionModelState>,
    pub modes: Option<SessionModeState>,
}

pub struct TestConnectionConfig {
    pub mcp_servers: Vec<McpServer>,
    pub builtins: Vec<String>,
    pub goose_mode: GooseMode,
    pub cwd: Option<tempfile::TempDir>,
    pub data_root: PathBuf,
    pub provider_factory: Option<ProviderConstructor>,
    pub read_text_file: Option<ReadTextFileHandler>,
    pub write_text_file: Option<WriteTextFileHandler>,
    pub terminal: Option<Arc<TerminalFixture>>,
}

impl Default for TestConnectionConfig {
    fn default() -> Self {
        Self {
            mcp_servers: Vec::new(),
            builtins: Vec::new(),
            goose_mode: GooseMode::default(),
            cwd: None,
            data_root: PathBuf::new(),
            provider_factory: None,
            read_text_file: None,
            write_text_file: None,
            terminal: None,
        }
    }
}

#[async_trait]
pub trait Connection: Sized {
    type Session: Session;

    fn expected_session_id() -> Arc<dyn ExpectedSessionId>;
    async fn new(config: TestConnectionConfig, openai: OpenAiFixture) -> Self;
    async fn new_session(&mut self) -> SessionResult<Self::Session>;
    async fn load_session(
        &mut self,
        session_id: &str,
        mcp_servers: Vec<McpServer>,
    ) -> SessionResult<Self::Session>;
    async fn set_mode(&self, session_id: &str, mode_id: &str) -> anyhow::Result<()>;
    async fn set_model(&self, session_id: &str, model_id: &str) -> anyhow::Result<()>;
    fn auth_methods(&self) -> &[AuthMethod];
    fn reset_openai(&self);
    fn reset_permissions(&self);
}

#[async_trait]
pub trait Session {
    fn session_id(&self) -> &sacp::schema::SessionId;
    async fn prompt(&mut self, text: &str, decision: PermissionDecision) -> TestOutput;
    async fn prompt_with_image(
        &mut self,
        text: &str,
        image_b64: &str,
        mime_type: &str,
        decision: PermissionDecision,
    ) -> TestOutput;
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

pub mod provider;
pub mod server;
