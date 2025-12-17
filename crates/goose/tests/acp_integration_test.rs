use agent_client_protocol::{
    self as acp, Agent, Client, ClientSideConnection, ContentBlock, InitializeRequest,
    NewSessionRequest, PromptRequest, ProtocolVersion, SessionNotification, SessionUpdate,
    TextContent,
};
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BASIC_RESPONSE: &str = include_str!("./test_data/openai_chat_completion_streaming.txt");
const BASIC_TEXT: &str = "Hello! How can I assist you today? 🌍";

#[tokio::test]
async fn test_acp_basic_completion() {
    let mock_server = setup_mock_openai(BASIC_RESPONSE).await;
    let work_dir = tempfile::tempdir().unwrap();

    let (client, updates) = TestClient::new();
    let child = spawn_goose_acp(&mock_server).await;

    run_acp_session(
        client,
        child,
        work_dir.path(),
        |conn, session_id| async move {
            let response = conn
                .prompt(PromptRequest::new(
                    session_id,
                    vec![ContentBlock::Text(TextContent::new("test message"))],
                ))
                .await
                .unwrap();

            assert_eq!(response.stop_reason, acp::StopReason::EndTurn);

            wait_for_text(&updates, BASIC_TEXT, Duration::from_secs(5)).await;
        },
    )
    .await;
}

async fn wait_for_text(
    updates: &Arc<Mutex<Vec<SessionNotification>>>,
    expected: &str,
    timeout: Duration,
) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let actual = extract_text(&updates.lock().unwrap());
        if actual == expected {
            return;
        }
        if tokio::time::Instant::now() > deadline {
            assert_eq!(actual, expected);
            return;
        }
        tokio::task::yield_now().await;
    }
}

async fn setup_mock_openai(streaming_response: &str) -> MockServer {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(streaming_response),
        )
        .mount(&mock_server)
        .await;

    mock_server
}

fn extract_text(updates: &[SessionNotification]) -> String {
    updates
        .iter()
        .filter_map(|n| match &n.update {
            SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
                ContentBlock::Text(t) => Some(t.text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

struct TestClient {
    updates: Arc<Mutex<Vec<SessionNotification>>>,
}

impl TestClient {
    fn new() -> (Self, Arc<Mutex<Vec<SessionNotification>>>) {
        let updates = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                updates: updates.clone(),
            },
            updates,
        )
    }
}

#[async_trait::async_trait(?Send)]
impl Client for TestClient {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        Err(acp::Error::method_not_found())
    }

    async fn session_notification(&self, args: SessionNotification) -> acp::Result<()> {
        self.updates.lock().unwrap().push(args);
        Ok(())
    }
}

async fn spawn_goose_acp(mock_server: &MockServer) -> Child {
    Command::new("cargo")
        .args(["run", "-p", "goose-cli", "--", "acp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GOOSE_PROVIDER", "openai")
        .env("GOOSE_MODEL", "gpt-5-nano")
        .env("OPENAI_HOST", mock_server.uri())
        .env("OPENAI_API_KEY", "test-key")
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

async fn run_acp_session<F, Fut>(client: TestClient, mut child: Child, work_dir: &Path, test_fn: F)
where
    F: FnOnce(ClientSideConnection, acp::SessionId) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

    let work_dir = work_dir.to_path_buf();
    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (conn, handle_io) = ClientSideConnection::new(client, outgoing, incoming, |fut| {
                tokio::task::spawn_local(fut);
            });
            tokio::task::spawn_local(handle_io);

            conn.initialize(InitializeRequest::new(ProtocolVersion::V1))
                .await
                .unwrap();

            let session = conn
                .new_session(NewSessionRequest::new(&work_dir))
                .await
                .unwrap();

            test_fn(conn, session.session_id).await;
        })
        .await;
}
