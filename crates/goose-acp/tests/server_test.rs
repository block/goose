mod common_tests;
use common_tests::fixtures::run_test;
use common_tests::fixtures::server::ClientToAgentSession;
use common_tests::{
    run_basic_completion, run_builtin_and_mcp, run_configured_extension, run_mcp_http_server,
    run_permission_persistence,
};

#[test]
fn test_acp_basic_completion() {
    run_test(async { run_basic_completion::<ClientToAgentSession>().await });
}

#[test]
fn test_acp_with_mcp_http_server() {
    run_test(async { run_mcp_http_server::<ClientToAgentSession>().await });
}

#[test]
fn test_acp_with_builtin_and_mcp() {
    run_test(async { run_builtin_and_mcp::<ClientToAgentSession>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<ClientToAgentSession>().await });
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
    let model_config = ModelConfig::new("gpt-5-nano", "openai").unwrap();
    let provider = OpenAiProvider::new(api_client, model_config);

    let config = GooseAcpConfig {
        provider: Arc::new(provider),
        builtins: builtins.iter().map(|s| s.to_string()).collect(),
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

#[allow(clippy::too_many_arguments)]
async fn run_acp_session<F, Fut>(
    mock_server: &MockServer,
    mcp_servers: Vec<McpServer>,
    builtins: &[&str],
    data_root: &Path,
    mode: GooseMode,
    select: Option<PermissionOptionKind>,
    expected_session_id: ExpectedSessionId,
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
            let expected_session_id = expected_session_id.clone();
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

                expected_session_id.set(&session.session_id);

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
    let expected_session_id = ExpectedSessionId::default();
    let mcp = McpFixture::new(expected_session_id.clone()).await;
    let openai = OpenAiFixture::new(
        vec![
            (
                format!(r#"</info-msg>\n{prompt}""#),
                include_str!("./test_data/openai_tool_call_response.txt"),
            ),
            (
                format!(r#""content":"{FAKE_CODE}""#),
                include_str!("./test_data/openai_tool_result_response.txt"),
            ),
        ],
        expected_session_id.clone(),
    )
    .await;

    run_acp_session(
        &openai.server,
        vec![McpServer::Http(McpServerHttp::new("lookup", &mcp.url))],
        &[],
        temp_dir.path(),
        GooseMode::Approve,
        kind,
        expected_session_id.clone(),
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

    expected_session_id.assert_no_errors();

    assert_eq!(
        fs::read_to_string(temp_dir.path().join("permission.yaml")).unwrap_or_default(),
        expected_yaml
    );
}

}
