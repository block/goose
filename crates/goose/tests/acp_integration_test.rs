mod common;

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, tool, tool_handler, tool_router,
    ErrorData as McpError, ServerHandler,
};
use sacp::schema::{
    ContentBlock, ContentChunk, InitializeRequest, McpServer, McpServerHttp, NewSessionRequest,
    PromptRequest, ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
    StopReason, TextContent,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::collections::VecDeque;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Fake code returned by the MCP server - an LLM couldn't know this from memory
const FAKE_CODE: &str = "test-uuid-12345-67890";

#[tokio::test]
async fn test_acp_basic_completion() {
    let prompt = "what is 1+1";
    let mock_server = setup_mock_openai(vec![(
        format!(r#"</info-msg>\n{prompt}","role":"user""#),
        include_str!("./test_data/openai_basic_response.txt"),
    )])
    .await;

    run_acp_session(
        &mock_server,
        vec![],
        &[],
        tempfile::tempdir().unwrap().path(),
        |cx, session_id, updates| async move {
            let response = cx
                .send_request(PromptRequest::new(
                    session_id,
                    vec![ContentBlock::Text(TextContent::new(prompt))],
                ))
                .block_task()
                .await
                .unwrap();

            assert_eq!(response.stop_reason, StopReason::EndTurn);
            wait_for_text(&updates, "2", Duration::from_secs(5)).await;
        },
    )
    .await;
}

#[tokio::test]
async fn test_acp_with_mcp_http_server() {
    let prompt = "Use the get_code tool and output only its result.";
    let (mcp_url, _handle) = spawn_mcp_http_server().await;

    let mock_server = setup_mock_openai(vec![
        (
            format!(r#"</info-msg>\n{prompt}","role":"user""#),
            include_str!("./test_data/openai_tool_call_response.txt"),
        ),
        (
            format!(r#""content":"{FAKE_CODE}","role":"tool""#),
            include_str!("./test_data/openai_tool_result_response.txt"),
        ),
    ])
    .await;

    run_acp_session(
        &mock_server,
        vec![McpServer::Http(McpServerHttp::new("lookup", &mcp_url))],
        &[],
        tempfile::tempdir().unwrap().path(),
        |cx, session_id, updates| async move {
            let response = cx
                .send_request(PromptRequest::new(
                    session_id,
                    vec![ContentBlock::Text(TextContent::new(prompt))],
                ))
                .block_task()
                .await
                .unwrap();

            assert_eq!(response.stop_reason, StopReason::EndTurn);
            wait_for_text(&updates, FAKE_CODE, Duration::from_secs(5)).await;
        },
    )
    .await;
}

#[tokio::test]
async fn test_acp_with_builtin_and_mcp() {
    let prompt =
        "Search for get_code and text_editor tools. Use them to save the code to /tmp/result.txt.";
    let (mcp_url, _handle) = spawn_mcp_http_server().await;

    let mock_server = setup_mock_openai(vec![
        (
            format!(r#"</info-msg>\n{prompt}","role":"user""#),
            include_str!("./test_data/openai_builtin_search.txt"),
        ),
        (
            r#"lookup/get_code: Get the code"#.into(),
            include_str!("./test_data/openai_builtin_read_modules.txt"),
        ),
        (
            r#"lookup[\"get_code\"]({}): string - Get the code"#.into(),
            include_str!("./test_data/openai_builtin_execute.txt"),
        ),
        (
            r#"Successfully wrote to /tmp/result.txt"#.into(),
            include_str!("./test_data/openai_builtin_final.txt"),
        ),
    ])
    .await;

    run_acp_session(
        &mock_server,
        vec![McpServer::Http(McpServerHttp::new("lookup", &mcp_url))],
        &["code_execution", "developer"],
        tempfile::tempdir().unwrap().path(),
        |cx, session_id, updates| async move {
            let response = cx
                .send_request(PromptRequest::new(
                    session_id,
                    vec![ContentBlock::Text(TextContent::new(prompt))],
                ))
                .block_task()
                .await
                .unwrap();

            assert_eq!(response.stop_reason, StopReason::EndTurn);
            wait_for_text(&updates, FAKE_CODE, Duration::from_secs(10)).await;
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
        if actual.contains(expected) {
            return;
        }
        if tokio::time::Instant::now() > deadline {
            assert_eq!(actual, expected);
            return;
        }
        tokio::task::yield_now().await;
    }
}

/// Each entry is (expected_body_substring, response_body).
/// Session description requests are handled automatically.
async fn setup_mock_openai(exchanges: Vec<(String, &'static str)>) -> MockServer {
    let mock_server = MockServer::start().await;
    let queue: VecDeque<(String, &'static str)> = exchanges.into_iter().collect();
    let queue = Arc::new(Mutex::new(queue));

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with({
            let queue = queue.clone();
            move |req: &wiremock::Request| {
                let body = String::from_utf8_lossy(&req.body);

                if body.contains("Reply with only a description in four words or less") {
                    return ResponseTemplate::new(200)
                        .insert_header("content-type", "application/json")
                        .set_body_string(include_str!(
                            "./test_data/openai_session_description.json"
                        ));
                }

                let (expected, response) = {
                    let mut q = queue.lock().unwrap();
                    match q.pop_front() {
                        Some(item) => item,
                        None => {
                            return ResponseTemplate::new(500)
                                .set_body_string(format!("unexpected request: {body}"));
                        }
                    }
                };

                if !body.contains(&expected) {
                    return ResponseTemplate::new(500).set_body_string(format!(
                        "expected body to contain: {expected}\nactual: {body}"
                    ));
                }

                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response)
            }
        })
        .mount(&mock_server)
        .await;

    mock_server
}

fn extract_text(updates: &[SessionNotification]) -> String {
    updates
        .iter()
        .filter_map(|n| match &n.update {
            SessionUpdate::AgentMessageChunk(ContentChunk {
                content: ContentBlock::Text(t),
                ..
            }) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

async fn spawn_goose_acp(mock_server: &MockServer, builtins: &[&str], data_root: &Path) -> Child {
    let mut cmd = Command::new(&*common::GOOSE_BINARY);
    cmd.args(["acp"]);
    if !builtins.is_empty() {
        cmd.arg("--with-builtin").arg(builtins.join(","));
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GOOSE_PROVIDER", "openai")
        .env("GOOSE_MODEL", "gpt-5-nano")
        .env("GOOSE_MODE", "approve")
        .env("OPENAI_HOST", mock_server.uri())
        .env("OPENAI_API_KEY", "test-key")
        .env("GOOSE_PATH_ROOT", data_root)
        .env(
            "RUST_LOG",
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        )
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

async fn run_acp_session<F, Fut>(
    mock_server: &MockServer,
    mcp_servers: Vec<McpServer>,
    builtins: &[&str],
    data_root: &Path,
    test_fn: F,
) where
    F: FnOnce(
        JrConnectionCx<ClientToAgent>,
        sacp::schema::SessionId,
        Arc<Mutex<Vec<SessionNotification>>>,
    ) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut child = spawn_goose_acp(mock_server, builtins, data_root).await;
    let work_dir = tempfile::tempdir().unwrap();
    let updates = Arc::new(Mutex::new(Vec::new()));
    let outgoing = child.stdin.take().unwrap().compat_write();
    let incoming = child.stdout.take().unwrap().compat();

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
        .on_receive_request(
            async move |request: RequestPermissionRequest, request_cx, _connection_cx| {
                let option_id = request.options.first().map(|opt| opt.option_id.clone());
                match option_id {
                    Some(id) => request_cx.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)),
                    )),
                    None => request_cx.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    )),
                }
            },
            sacp::on_receive_request!(),
        )
        .connect_to(transport)
        .unwrap()
        .run_until({
            let updates = updates.clone();
            move |cx: JrConnectionCx<ClientToAgent>| async move {
                cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                    .block_task()
                    .await
                    .unwrap();

                let session = cx
                    .send_request(
                        NewSessionRequest::new(work_dir.path().to_path_buf())
                            .mcp_servers(mcp_servers),
                    )
                    .block_task()
                    .await
                    .unwrap();

                test_fn(cx.clone(), session.session_id, updates).await;
                Ok(())
            }
        })
        .await
        .unwrap();
}

#[derive(Clone)]
struct Lookup {
    tool_router: ToolRouter<Lookup>,
}

#[tool_router]
impl Lookup {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Returns a fake code that an LLM couldn't know from memory
    #[tool(description = "Get the code")]
    fn get_code(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(FAKE_CODE)]))
    }
}

#[tool_handler]
impl ServerHandler for Lookup {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2025_03_26,
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

async fn spawn_mcp_http_server() -> (String, JoinHandle<()>) {
    let service = StreamableHttpService::new(
        || Ok(Lookup::new()),
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

    (url, handle)
}
