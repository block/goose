mod common;

use common::{setup_mock_openai, spawn_mcp_http_server, FAKE_CODE};
use fs_err as fs;
use goose::config::GooseMode;
use goose::model::ModelConfig;
use goose::providers::api_client::{ApiClient, AuthMethod};
use goose::providers::openai::OpenAiProvider;
use goose_acp::server::{serve, GooseAcpAgent, GooseAcpConfig};
use sacp::schema::{
    ContentBlock, ContentChunk, InitializeRequest, McpServer, McpServerHttp, NewSessionRequest,
    PermissionOptionKind, PromptRequest, ProtocolVersion, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SelectedPermissionOutcome,
    SessionNotification, SessionUpdate, StopReason, TextContent, ToolCallId, ToolCallStatus,
    ToolCallUpdate, ToolCallUpdateFields,
};
use sacp::{ClientToAgent, JrConnectionCx};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use test_case::test_case;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use wiremock::MockServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_acp_basic_completion() {
    let temp_dir = tempfile::tempdir().unwrap();
    let prompt = "what is 1+1";
    let mock_server = setup_mock_openai(vec![(
        format!(r#"</info-msg>\n{prompt}""#),
        include_str!("./test_data/openai_basic_response.txt"),
    )])
    .await;

    run_acp_session(
        &mock_server,
        vec![],
        &[],
        temp_dir.path(),
        GooseMode::Auto,
        None,
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
            wait_for(
                &updates,
                &SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new("2"),
                ))),
            )
            .await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_acp_with_mcp_http_server() {
    let temp_dir = tempfile::tempdir().unwrap();
    let prompt = "Use the get_code tool and output only its result.";
    let (mcp_url, _handle) = spawn_mcp_http_server().await;

    let mock_server = setup_mock_openai(vec![
        (
            format!(r#"</info-msg>\n{prompt}""#),
            include_str!("./test_data/openai_tool_call_response.txt"),
        ),
        (
            format!(r#""content":"{FAKE_CODE}""#),
            include_str!("./test_data/openai_tool_result_response.txt"),
        ),
    ])
    .await;

    run_acp_session(
        &mock_server,
        vec![McpServer::Http(McpServerHttp::new("lookup", mcp_url))],
        &[],
        temp_dir.path(),
        GooseMode::Auto,
        None,
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
            wait_for(
                &updates,
                &SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new(FAKE_CODE),
                ))),
            )
            .await;
        },
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_acp_with_builtin_and_mcp() {
    let temp_dir = tempfile::tempdir().unwrap();
    let prompt =
        "Search for get_code and text_editor tools. Use them to save the code to /tmp/result.txt.";
    let (lookup_url, _lookup_handle) = spawn_mcp_http_server().await;

    let mock_server = setup_mock_openai(vec![
        (
            format!(r#"</info-msg>\n{prompt}""#),
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
        vec![McpServer::Http(McpServerHttp::new("lookup", lookup_url))],
        &["code_execution", "developer"],
        temp_dir.path(),
        GooseMode::Auto,
        None,
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
            wait_for(
                &updates,
                &SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                    TextContent::new(FAKE_CODE),
                ))),
            )
            .await;
        },
    )
    .await;
}

async fn wait_for(updates: &Arc<Mutex<Vec<SessionNotification>>>, expected: &SessionUpdate) {
    let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
    let mut context = String::new();

    loop {
        let matched = {
            let guard = updates.lock().unwrap();
            context.clear();

            match expected {
                SessionUpdate::AgentMessageChunk(chunk) => {
                    let expected_text = match &chunk.content {
                        ContentBlock::Text(t) => &t.text,
                        other => panic!("wait_for: unhandled content {:?}", other),
                    };
                    for n in guard.iter() {
                        if let SessionUpdate::AgentMessageChunk(c) = &n.update {
                            if let ContentBlock::Text(t) = &c.content {
                                if t.text.is_empty() {
                                    context.clear();
                                } else {
                                    context.push_str(&t.text);
                                }
                            }
                        }
                    }
                    context.contains(expected_text)
                }
                SessionUpdate::ToolCallUpdate(expected_update) => {
                    for n in guard.iter() {
                        if let SessionUpdate::ToolCallUpdate(u) = &n.update {
                            context.push_str(&format!("{:?}\n", u));
                            if u.fields.status == expected_update.fields.status {
                                return;
                            }
                        }
                    }
                    false
                }
                other => panic!("wait_for: unhandled update {:?}", other),
            }
        };

        if matched {
            return;
        }
        if tokio::time::Instant::now() > deadline {
            panic!("Timeout waiting for {:?}\n\n{}", expected, context);
        }
        tokio::task::yield_now().await;
    }
}

async fn spawn_server_in_process(
    mock_server: &MockServer,
    builtins: &[&str],
    data_root: &Path,
    goose_mode: GooseMode,
) -> (
    tokio::io::DuplexStream,
    tokio::io::DuplexStream,
    tokio::task::JoinHandle<()>,
) {
    let api_client = ApiClient::new(
        mock_server.uri(),
        AuthMethod::BearerToken("test-key".to_string()),
    )
    .unwrap();
    let model_config = ModelConfig::new("gpt-5-nano").unwrap();
    let provider = OpenAiProvider::new(api_client, model_config);

    let config = GooseAcpConfig {
        provider: Arc::new(provider),
        builtins: builtins.iter().map(|s| s.to_string()).collect(),
        work_dir: data_root.to_path_buf(),
        data_dir: data_root.to_path_buf(),
        config_dir: data_root.to_path_buf(),
        goose_mode,
    };

    let (client_read, server_write) = tokio::io::duplex(64 * 1024);
    let (server_read, client_write) = tokio::io::duplex(64 * 1024);

    let agent = Arc::new(GooseAcpAgent::with_config(config).await.unwrap());
    let handle = tokio::spawn(async move {
        if let Err(e) = serve(agent, server_read.compat(), server_write.compat_write()).await {
            tracing::error!("ACP server error: {e}");
        }
    });

    (client_read, client_write, handle)
}

async fn run_acp_session<F, Fut>(
    mock_server: &MockServer,
    mcp_servers: Vec<McpServer>,
    builtins: &[&str],
    data_root: &Path,
    mode: GooseMode,
    select: Option<PermissionOptionKind>,
    test_fn: F,
) where
    F: FnOnce(
        JrConnectionCx<ClientToAgent>,
        sacp::schema::SessionId,
        Arc<Mutex<Vec<SessionNotification>>>,
    ) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let (client_read, client_write, _handle) =
        spawn_server_in_process(mock_server, builtins, data_root, mode).await;
    let work_dir = tempfile::tempdir().unwrap();
    let updates = Arc::new(Mutex::new(Vec::new()));

    let transport = sacp::ByteStreams::new(client_write.compat_write(), client_read.compat());

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
            async move |req: RequestPermissionRequest, request_cx, _connection_cx| {
                let response = match select {
                    Some(kind) => {
                        let id = req
                            .options
                            .iter()
                            .find(|o| o.kind == kind)
                            .unwrap()
                            .option_id
                            .clone();
                        RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
                            SelectedPermissionOutcome::new(id),
                        ))
                    }
                    None => RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled),
                };
                request_cx.respond(response)
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
                    .send_request(NewSessionRequest::new(work_dir.path()).mcp_servers(mcp_servers))
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

#[test_case(Some(PermissionOptionKind::AllowAlways), ToolCallStatus::Completed, "user:\n  always_allow:\n  - lookup__get_code\n  ask_before: []\n  never_allow: []\n"; "allow_always")]
#[test_case(Some(PermissionOptionKind::AllowOnce), ToolCallStatus::Completed, ""; "allow_once")]
#[test_case(Some(PermissionOptionKind::RejectAlways), ToolCallStatus::Failed, "user:\n  always_allow: []\n  ask_before: []\n  never_allow:\n  - lookup__get_code\n"; "reject_always")]
#[test_case(Some(PermissionOptionKind::RejectOnce), ToolCallStatus::Failed, ""; "reject_once")]
#[test_case(None, ToolCallStatus::Failed, ""; "cancelled")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_permission_persistence(
    kind: Option<PermissionOptionKind>,
    expected_status: ToolCallStatus,
    expected_yaml: &str,
) {
    let temp_dir = tempfile::tempdir().unwrap();
    let prompt = "Use the get_code tool and output only its result.";
    let (mcp_url, _handle) = spawn_mcp_http_server().await;

    let mock_server = setup_mock_openai(vec![
        (
            format!(r#"</info-msg>\n{prompt}""#),
            include_str!("./test_data/openai_tool_call_response.txt"),
        ),
        (
            format!(r#""content":"{FAKE_CODE}""#),
            include_str!("./test_data/openai_tool_result_response.txt"),
        ),
    ])
    .await;

    run_acp_session(
        &mock_server,
        vec![McpServer::Http(McpServerHttp::new("lookup", mcp_url))],
        &[],
        temp_dir.path(),
        GooseMode::Approve,
        kind,
        |cx, session_id, updates| async move {
            cx.send_request(PromptRequest::new(
                session_id,
                vec![ContentBlock::Text(TextContent::new(prompt))],
            ))
            .block_task()
            .await
            .unwrap();
            wait_for(
                &updates,
                &SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(
                    ToolCallId::new(""),
                    ToolCallUpdateFields::new().status(Some(expected_status)),
                )),
            )
            .await;
        },
    )
    .await;

    assert_eq!(
        fs::read_to_string(temp_dir.path().join("permission.yaml")).unwrap_or_default(),
        expected_yaml
    );
}
