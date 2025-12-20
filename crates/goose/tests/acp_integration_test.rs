use sacp::schema::{
    ContentBlock, ContentChunk, InitializeRequest, NewSessionRequest, PromptRequest,
    SessionNotification, SessionUpdate, StopReason, TextContent, VERSION as PROTOCOL_VERSION,
};
use sacp::{ClientToAgent, JrConnectionCx};
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

    let updates = Arc::new(Mutex::new(Vec::<SessionNotification>::new()));
    let child = spawn_goose_acp(&mock_server).await;

    run_acp_session(child, work_dir.path(), updates.clone(), |cx, session_id| {
        let updates = updates.clone();
        async move {
            let response = cx
                .send_request(PromptRequest {
                    session_id,
                    prompt: vec![ContentBlock::Text(TextContent {
                        text: "test message".to_string(),
                        annotations: None,
                        meta: None,
                    })],
                    meta: None,
                })
                .block_task()
                .await
                .unwrap();

            assert_eq!(response.stop_reason, StopReason::EndTurn);

            wait_for_text(&updates, BASIC_TEXT, Duration::from_secs(5)).await;
        }
    })
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
            SessionUpdate::AgentMessageChunk(ContentChunk { content, .. }) => match content {
                ContentBlock::Text(t) => Some(t.text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
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

async fn run_acp_session<F, Fut>(
    mut child: Child,
    work_dir: &Path,
    updates: Arc<Mutex<Vec<SessionNotification>>>,
    test_fn: F,
) where
    F: FnOnce(JrConnectionCx<ClientToAgent>, sacp::schema::SessionId) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

    let work_dir = work_dir.to_path_buf();
    let transport = sacp::ByteStreams::new(outgoing, incoming);

    ClientToAgent::builder()
        .on_receive_notification(
            {
                let updates = updates.clone();
                async move |notification: SessionNotification, _cx| {
                    updates.lock().unwrap().push(notification);
                    Ok(())
                }
            },
            sacp::on_receive_notification!(),
        )
        .with_client(transport, |cx: JrConnectionCx<ClientToAgent>| async move {
            cx.send_request(InitializeRequest {
                protocol_version: PROTOCOL_VERSION,
                client_capabilities: Default::default(),
                client_info: Default::default(),
                meta: None,
            })
            .block_task()
            .await
            .unwrap();

            let session = cx
                .send_request(NewSessionRequest {
                    mcp_servers: vec![],
                    cwd: work_dir,
                    meta: None,
                })
                .block_task()
                .await
                .unwrap();

            test_fn(cx.clone(), session.session_id).await;

            Ok(())
        })
        .await
        .unwrap();
}
