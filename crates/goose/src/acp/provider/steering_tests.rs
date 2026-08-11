use super::*;
use agent_client_protocol::schema::v1::{
    Implementation, PermissionOption, PermissionOptionKind, ToolCallId, ToolCallUpdate,
    ToolCallUpdateFields,
};
use futures::StreamExt;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;

const ACP_SESSION_ID: &str = "claude-test-session";
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

fn test_config() -> AcpProviderConfig {
    AcpProviderConfig {
        command: "unused".into(),
        args: vec![],
        env: vec![],
        env_remove: vec![],
        work_dir: ".".into(),
        mcp_servers: vec![],
        session_mode_id: None,
        session_config_options: vec![],
        model_config_option_id: None,
        mode_mapping: HashMap::new(),
        notification_callback: None,
    }
}

fn supported_initialize_response(
    request: InitializeRequest,
    adapter_version: &str,
) -> InitializeResponse {
    let mut meta = serde_json::Map::new();
    meta.insert(
        "steering".to_string(),
        serde_json::json!({ "supported": true }),
    );
    InitializeResponse::new(request.protocol_version)
        .agent_info(Implementation::new("claude-agent-acp", adapter_version))
        .meta(meta)
}

fn permission_request(tool_call_id: &str) -> RequestPermissionRequest {
    RequestPermissionRequest::new(
        ACP_SESSION_ID,
        ToolCallUpdate::new(
            ToolCallId::new(tool_call_id),
            ToolCallUpdateFields::default(),
        ),
        vec![
            PermissionOption::new("allow", "Allow", PermissionOptionKind::AllowOnce),
            PermissionOption::new("reject", "Reject", PermissionOptionKind::RejectOnce),
        ],
    )
}

macro_rules! test_agent {
    ($adapter_version:expr) => {{
        let adapter_version = $adapter_version.to_string();
        agent_client_protocol::Agent
            .builder()
            .on_receive_request(
                async move |request: InitializeRequest, responder, _cx| {
                    responder.respond(supported_initialize_response(request, &adapter_version))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .on_receive_request(
                async |_request: NewSessionRequest, responder, _cx| {
                    responder.respond(NewSessionResponse::new(ACP_SESSION_ID))
                },
                agent_client_protocol::on_receive_request!(),
            )
    }};
}

async fn connect_provider(
    provider_name: &str,
    transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
) -> AcpProvider {
    connect_provider_in_mode(provider_name, GooseMode::Auto, transport).await
}

async fn connect_provider_in_mode(
    provider_name: &str,
    goose_mode: GooseMode,
    transport: impl agent_client_protocol::ConnectTo<Client> + 'static,
) -> AcpProvider {
    AcpProvider::connect_with_transport(
        provider_name.to_string(),
        goose_mode,
        test_config(),
        transport,
    )
    .await
    .unwrap()
}

async fn receive<T>(receiver: &mut mpsc::UnboundedReceiver<T>, expectation: &str) -> T {
    timeout(TEST_TIMEOUT, receiver.recv())
        .await
        .expect(expectation)
        .expect("test channel should remain open")
}

async fn connect_test_provider(
    provider_name: &str,
    adapter_version: &str,
    steer_response: Option<ClaudeSteeringResponse>,
) -> (AcpProvider, mpsc::UnboundedReceiver<serde_json::Value>) {
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let agent = test_agent!(adapter_version).on_receive_request(
        async move |request: ClaudeSteeringRequest, responder, _cx| {
            request_tx
                .send(serde_json::to_value(request).unwrap())
                .unwrap();
            match &steer_response {
                Some(response) => responder.respond(response.clone()),
                None => Err(agent_client_protocol::Error::internal_error()),
            }
        },
        agent_client_protocol::on_receive_request!(),
    );

    let provider = connect_provider(provider_name, agent).await;
    (provider, request_rx)
}

struct PromptTestHarness {
    provider: AcpProvider,
    prompt_started: mpsc::UnboundedReceiver<String>,
    cancellations: mpsc::UnboundedReceiver<CancelNotification>,
    prompt_responded: mpsc::UnboundedReceiver<()>,
    release_prompt: Arc<Notify>,
}

async fn connect_prompt_test_provider(
    provider_name: &str,
    prompt_update: Option<&'static str>,
) -> PromptTestHarness {
    let (prompt_started_tx, prompt_started_rx) = mpsc::unbounded_channel();
    let (cancel_tx, cancel_rx) = mpsc::unbounded_channel();
    let (prompt_responded_tx, prompt_responded_rx) = mpsc::unbounded_channel();
    let release_prompt = Arc::new(Notify::new());
    let agent = test_agent!("0.65.0")
        .on_receive_request(
            {
                let release_prompt = Arc::clone(&release_prompt);
                async move |request: PromptRequest, responder, cx| {
                    let release_prompt = Arc::clone(&release_prompt);
                    let prompt_responded_tx = prompt_responded_tx.clone();
                    let prompt_text = request
                        .prompt
                        .iter()
                        .find_map(|content| match content {
                            ContentBlock::Text(text) => Some(text.text.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    prompt_started_tx.send(prompt_text).unwrap();
                    let prompt_cx = cx.clone();
                    cx.spawn(async move {
                        release_prompt.notified().await;
                        if let Some(text) = prompt_update {
                            prompt_cx.send_notification(SessionNotification::new(
                                ACP_SESSION_ID,
                                SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(text)),
                                )),
                            ))?;
                        }
                        let result = responder.respond(PromptResponse::new(StopReason::EndTurn));
                        let _ = prompt_responded_tx.send(());
                        result
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |notification: CancelNotification, _cx| {
                cancel_tx.send(notification).unwrap();
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async |_request: ClaudeSteeringRequest, responder, _cx| {
                responder.respond(ClaudeSteeringResponse::Injected)
            },
            agent_client_protocol::on_receive_request!(),
        );

    let provider = connect_provider(provider_name, agent).await;

    PromptTestHarness {
        provider,
        prompt_started: prompt_started_rx,
        cancellations: cancel_rx,
        prompt_responded: prompt_responded_rx,
        release_prompt,
    }
}

fn boundary_test_provider() -> (Arc<AcpProvider>, mpsc::Receiver<ClientRequest>) {
    let (tx, rx) = mpsc::channel(2);
    let provider = AcpProvider {
        name: CLAUDE_ACP_PROVIDER_NAME.to_string(),
        supports_native_steering: true,
        assistant_message_boundary_pending: Arc::new(AtomicBool::new(false)),
        goose_mode: Arc::new(Mutex::new(GooseMode::Auto)),
        mode_mapping: HashMap::new(),
        session: Mutex::new(AcpSession {
            id: SessionId::new(ACP_SESSION_ID),
            response: NewSessionResponse::new(ACP_SESSION_ID),
        }),
        pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
        steer_generation: Arc::new(AtomicU64::new(0)),
        pending_tool_updates: Arc::new(Mutex::new(HashMap::new())),
        handoff_context_sent: AtomicBool::new(false),
        context_size: Arc::new(AtomicU64::new(0)),
        session_title_publisher: SessionTitlePublisher::default(),
        model_config_option_id: None,
        applied_model: Arc::new(Mutex::new(None)),
        tx: Some(tx),
        loop_thread: None,
    };
    (Arc::new(provider), rx)
}

async fn start_boundary_test_stream() -> (
    Arc<AcpProvider>,
    mpsc::Receiver<ClientRequest>,
    mpsc::Sender<AcpUpdate>,
    MessageStream,
) {
    let (provider, mut requests) = boundary_test_provider();
    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("start a long task");
    let stream = provider.stream(&model, "", &[prompt], &[]).await.unwrap();
    let response_tx = match timeout(TEST_TIMEOUT, requests.recv())
        .await
        .expect("prompt request should arrive")
        .expect("request channel should remain open")
    {
        ClientRequest::Prompt { response_tx, .. } => response_tx,
        _ => panic!("expected prompt request"),
    };
    (provider, requests, response_tx, stream)
}

async fn send_update_and_message_id(
    response_tx: &mpsc::Sender<AcpUpdate>,
    stream: &mut MessageStream,
    update: AcpUpdate,
) -> String {
    response_tx.send(update).await.unwrap();
    timeout(TEST_TIMEOUT, stream.next())
        .await
        .expect("provider stream should produce a message")
        .expect("provider stream should remain open")
        .expect("provider update should succeed")
        .0
        .expect("provider update should contain a message")
        .id
        .expect("provider message should have an ID")
}

async fn complete_native_steer(
    provider: Arc<AcpProvider>,
    requests: &mut mpsc::Receiver<ClientRequest>,
) -> bool {
    let steer = tokio::spawn(async move {
        provider
            .steer_natively(
                "goose-session",
                &Message::user().with_text("focus on the tests"),
            )
            .await
    });
    match timeout(TEST_TIMEOUT, requests.recv())
        .await
        .expect("steering request should arrive")
        .expect("request channel should remain open")
    {
        ClientRequest::ClaudeSteer {
            assistant_message_boundary_pending,
            response_tx,
            ..
        } => {
            assistant_message_boundary_pending.store(true, Ordering::Release);
            response_tx
                .send(Ok(ClaudeSteeringResponse::Injected))
                .unwrap();
        }
        _ => panic!("expected steering request"),
    }
    steer.await.unwrap().unwrap()
}

#[tokio::test]
async fn injected_response_confirms_delivery_and_message_boundary() {
    let (provider, mut requests) = connect_test_provider(
        CLAUDE_ACP_PROVIDER_NAME,
        "0.65.0",
        Some(ClaudeSteeringResponse::Injected),
    )
    .await;

    let delivered = provider
        .steer_natively(
            "goose-session",
            &Message::user().with_text("focus on tests"),
        )
        .await
        .unwrap();

    assert!(delivered);
    assert!(provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));
    requests
        .recv()
        .await
        .expect("steering request should be sent");
}

#[tokio::test]
async fn injected_steer_cancels_permissions_that_arrive_while_delivery_is_pending() {
    let (permission_started_tx, mut permission_started_rx) = mpsc::unbounded_channel();
    let (permission_cancelled_tx, mut permission_cancelled_rx) = mpsc::unbounded_channel();
    let release_prompt = Arc::new(Notify::new());
    let agent = test_agent!("0.65.0")
        .on_receive_request(
            {
                let release_prompt = Arc::clone(&release_prompt);
                async move |_request: PromptRequest, responder, cx| {
                    let permission_started_tx = permission_started_tx.clone();
                    let permission_cancelled_tx = permission_cancelled_tx.clone();
                    let release_prompt = Arc::clone(&release_prompt);
                    for tool_call_id in ["tool-1", "tool-2"] {
                        let permission_started_tx = permission_started_tx.clone();
                        let permission_cancelled_tx = permission_cancelled_tx.clone();
                        let permission_cx = cx.clone();
                        cx.spawn(async move {
                            let request =
                                permission_cx.send_request(permission_request(tool_call_id));
                            permission_started_tx.send(()).unwrap();
                            let response = request.block_task().await?;
                            permission_cancelled_tx
                                .send(matches!(
                                    response.outcome,
                                    RequestPermissionOutcome::Cancelled
                                ))
                                .unwrap();
                            Ok(())
                        })?;
                    }

                    cx.spawn(async move {
                        release_prompt.notified().await;
                        responder.respond(PromptResponse::new(StopReason::EndTurn))
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async |_request: ClaudeSteeringRequest, responder, _cx| {
                responder.respond(ClaudeSteeringResponse::Injected)
            },
            agent_client_protocol::on_receive_request!(),
        );

    let provider =
        connect_provider_in_mode(CLAUDE_ACP_PROVIDER_NAME, GooseMode::Approve, agent).await;
    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("start a long task");
    let mut stream = provider.stream(&model, "", &[prompt], &[]).await.unwrap();
    let (action_required_tx, mut action_required_rx) = mpsc::unbounded_channel();
    let drain_stream = tokio::spawn(async move {
        while let Some(update) = stream.next().await {
            if update?.0.is_some() {
                let _ = action_required_tx.send(());
            }
        }
        Ok::<_, ProviderError>(())
    });

    receive(&mut permission_started_rx, "first permission should start").await;
    receive(&mut permission_started_rx, "second permission should start").await;
    receive(
        &mut action_required_rx,
        "permission confirmation should be registered",
    )
    .await;
    assert_eq!(provider.pending_confirmations.lock().await.len(), 1);
    assert!(provider
        .steer_natively(
            "goose-session",
            &Message::user().with_text("focus on the tests"),
        )
        .await
        .unwrap());

    assert!(
        receive(
            &mut permission_cancelled_rx,
            "first permission should cancel"
        )
        .await
    );
    assert!(
        receive(
            &mut permission_cancelled_rx,
            "second permission should cancel"
        )
        .await
    );
    assert!(provider.pending_confirmations.lock().await.is_empty());

    release_prompt.notify_one();
    timeout(TEST_TIMEOUT, drain_stream)
        .await
        .expect("prompt stream should finish")
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn stale_permission_marks_the_tool_call_rejected() {
    let (provider, _requests, response_tx, mut stream) = start_boundary_test_stream().await;
    provider.steer_generation.store(1, Ordering::Release);
    let (permission_response_tx, permission_response_rx) = oneshot::channel();
    response_tx
        .send(AcpUpdate::PermissionRequest {
            request: Box::new(permission_request("tool-1")),
            generation: 0,
            response_tx: permission_response_tx,
        })
        .await
        .unwrap();
    response_tx
        .send(AcpUpdate::ToolCallComplete {
            id: "tool-1".to_string(),
            raw_output: None,
            content: None,
            is_error: false,
        })
        .await
        .unwrap();

    let denial = timeout(TEST_TIMEOUT, stream.next())
        .await
        .expect("rejected tool call should produce a response")
        .expect("stream should remain open")
        .expect("tool-call response should succeed")
        .0
        .expect("tool-call response should contain a message");
    assert_eq!(
        denial
            .content
            .first()
            .and_then(|content| content.as_tool_response_text())
            .as_deref(),
        Some("Tool call was denied.")
    );
    let permission_response = permission_response_rx.await.unwrap();
    assert!(matches!(
        permission_response.outcome,
        RequestPermissionOutcome::Cancelled
    ));
}

#[tokio::test]
async fn non_claude_provider_does_not_send_a_steering_request() {
    let (provider, mut requests) = connect_test_provider(
        "other-acp",
        "0.65.0",
        Some(ClaudeSteeringResponse::Injected),
    )
    .await;

    let delivered = provider
        .steer_natively(
            "goose-session",
            &Message::user().with_text("focus on tests"),
        )
        .await
        .unwrap();

    assert!(!delivered);
    assert!(matches!(
        requests.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn protocol_failure_is_returned_as_provider_error() {
    let (provider, mut requests) =
        connect_test_provider(CLAUDE_ACP_PROVIDER_NAME, "0.65.0", None).await;

    let result = provider
        .steer_natively(
            "goose-session",
            &Message::user().with_text("focus on tests"),
        )
        .await;

    assert!(matches!(result, Err(ProviderError::RequestFailed(_))));
    assert!(!provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));
    assert!(requests.recv().await.is_some());
}

#[tokio::test]
async fn dropping_steering_caller_cancels_acp_request_and_releases_loop() {
    let (request_started_tx, mut request_started) = mpsc::unbounded_channel();
    let (request_cancelled_tx, mut request_cancelled) = mpsc::unbounded_channel();
    let steering_requests = Arc::new(AtomicUsize::new(0));
    let agent = test_agent!("0.65.0").on_receive_request(
        {
            let steering_requests = Arc::clone(&steering_requests);
            async move |_request: ClaudeSteeringRequest, responder, cx| {
                if steering_requests.fetch_add(1, Ordering::SeqCst) == 0 {
                    let cancellation = responder.cancellation();
                    let request_cancelled_tx = request_cancelled_tx.clone();
                    request_started_tx.send(()).unwrap();
                    cx.spawn(async move {
                        cancellation.cancelled().await;
                        request_cancelled_tx.send(()).unwrap();
                        responder.respond_with_result(Err(
                            agent_client_protocol::Error::request_cancelled(),
                        ))
                    })
                } else {
                    responder.respond(ClaudeSteeringResponse::PromptRequired)
                }
            }
        },
        agent_client_protocol::on_receive_request!(),
    );

    let provider = Arc::new(connect_provider(CLAUDE_ACP_PROVIDER_NAME, agent).await);
    let first_request = tokio::spawn({
        let provider = Arc::clone(&provider);
        async move {
            provider
                .steer_natively("goose-session", &Message::user().with_text("first steer"))
                .await
        }
    });

    receive(&mut request_started, "steering request should start").await;
    first_request.abort();
    assert!(first_request.await.unwrap_err().is_cancelled());
    receive(
        &mut request_cancelled,
        "dropping the steering caller should cancel the ACP request",
    )
    .await;

    let delivered = timeout(
        TEST_TIMEOUT,
        provider.steer_natively("goose-session", &Message::user().with_text("second steer")),
    )
    .await
    .expect("the ACP request loop should accept another steer")
    .unwrap();
    assert!(!delivered);
    assert_eq!(steering_requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn dropping_prompt_stream_sends_session_cancel() {
    let PromptTestHarness {
        provider,
        mut prompt_started,
        mut cancellations,
        mut prompt_responded,
        release_prompt,
    } = connect_prompt_test_provider("test-acp", None).await;
    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("start a long task");
    let stream = provider.stream(&model, "", &[prompt], &[]).await.unwrap();

    receive(&mut prompt_started, "prompt should start").await;
    drop(stream);

    let cancellation = receive(
        &mut cancellations,
        "dropping the stream should cancel the ACP prompt",
    )
    .await;
    assert_eq!(cancellation.session_id, SessionId::new(ACP_SESSION_ID));

    release_prompt.notify_one();
    receive(&mut prompt_responded, "prompt should respond after release").await;
    assert!(matches!(
        cancellations.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn steering_completes_while_original_prompt_remains_active() {
    let PromptTestHarness {
        provider,
        mut prompt_started,
        cancellations: _cancellations,
        mut prompt_responded,
        release_prompt,
    } = connect_prompt_test_provider(CLAUDE_ACP_PROVIDER_NAME, Some("original prompt update"))
        .await;
    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("start a long task");
    let mut original_stream = provider
        .stream(&model, "", std::slice::from_ref(&prompt), &[])
        .await
        .unwrap();

    receive(&mut prompt_started, "prompt should start").await;

    let delivered = timeout(
        TEST_TIMEOUT,
        provider.steer_natively(
            "goose-session",
            &Message::user().with_text("focus on the tests"),
        ),
    )
    .await
    .expect("steering should complete while the prompt is active")
    .unwrap();
    assert!(delivered);
    assert!(provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));
    assert!(matches!(
        prompt_responded.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));

    let mut second_stream = provider
        .stream(&model, "", std::slice::from_ref(&prompt), &[])
        .await
        .unwrap();
    assert!(matches!(
        prompt_started.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
    assert!(provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));

    release_prompt.notify_one();
    receive(&mut prompt_responded, "prompt should respond after release").await;
    let original_update = timeout(TEST_TIMEOUT, original_stream.next())
        .await
        .expect("original stream should receive its update")
        .expect("original stream should remain open")
        .expect("original stream update should succeed")
        .0
        .expect("original stream update should contain a message");
    assert_eq!(original_update.as_concat_text(), "original prompt update");
    assert!(!provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));
    assert!(timeout(TEST_TIMEOUT, original_stream.next())
        .await
        .expect("original stream should finish")
        .is_none());

    receive(&mut prompt_started, "queued prompt should start").await;
    release_prompt.notify_one();
    receive(&mut prompt_responded, "queued prompt should respond").await;
    let queued_update = timeout(TEST_TIMEOUT, second_stream.next())
        .await
        .expect("queued prompt should receive its update")
        .expect("queued prompt stream should remain open")
        .expect("queued prompt update should succeed")
        .0
        .expect("queued prompt update should contain a message");
    assert_eq!(queued_update.as_concat_text(), "original prompt update");
    assert!(timeout(TEST_TIMEOUT, second_stream.next())
        .await
        .expect("queued prompt should finish")
        .is_none());
}

#[tokio::test]
async fn cancelled_queued_prompt_is_not_sent() {
    let PromptTestHarness {
        provider,
        mut prompt_started,
        cancellations: _cancellations,
        mut prompt_responded,
        release_prompt,
    } = connect_prompt_test_provider("test-acp", None).await;
    let model = ModelConfig::new("test-model");
    let first_prompt = Message::user().with_text("first prompt");
    let mut original_stream = provider
        .stream(&model, "", std::slice::from_ref(&first_prompt), &[])
        .await
        .unwrap();

    assert_eq!(
        receive(&mut prompt_started, "prompt should start").await,
        "first prompt"
    );
    let cancelled_prompt = Message::user().with_text("cancelled prompt");
    let queued_stream = provider
        .stream(&model, "", std::slice::from_ref(&cancelled_prompt), &[])
        .await
        .unwrap();
    drop(queued_stream);
    let final_prompt = Message::user().with_text("final prompt");
    let mut final_stream = provider
        .stream(&model, "", std::slice::from_ref(&final_prompt), &[])
        .await
        .unwrap();

    release_prompt.notify_one();
    receive(&mut prompt_responded, "prompt should respond after release").await;
    assert!(timeout(TEST_TIMEOUT, original_stream.next())
        .await
        .expect("original prompt should finish")
        .is_none());
    assert_eq!(
        receive(&mut prompt_started, "final prompt should start").await,
        "final prompt"
    );
    release_prompt.notify_one();
    receive(&mut prompt_responded, "final prompt should respond").await;
    assert!(timeout(TEST_TIMEOUT, final_stream.next())
        .await
        .expect("final prompt should finish")
        .is_none());
}

#[tokio::test]
async fn injected_steers_start_new_assistant_runs_once() {
    let (provider, mut requests, response_tx, mut stream) = start_boundary_test_stream().await;

    let text_before = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Text(TextContent::new("text before steer")),
    )
    .await;
    let thought_before = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Thought("thought before steer".to_string()),
    )
    .await;

    assert!(complete_native_steer(provider, &mut requests).await);

    let text_after = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Text(TextContent::new("text after steer")),
    )
    .await;
    let text_continued = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Text(TextContent::new("continued text")),
    )
    .await;
    let thought_after = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Thought("thought after steer".to_string()),
    )
    .await;
    let thought_continued = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Thought("continued thought".to_string()),
    )
    .await;

    assert_ne!(text_before, text_after);
    assert_eq!(text_after, text_continued);
    assert_ne!(thought_before, thought_after);
    assert_eq!(thought_after, thought_continued);
}
