use super::*;
use agent_client_protocol::schema::v1::Implementation;
use futures::StreamExt;
use rmcp::model::{Annotations, TextContent as RmcpTextContent};
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

async fn connect_test_provider(
    provider_name: &str,
    adapter_version: &str,
    steer_response: Option<ClaudeSteeringResponse>,
) -> (AcpProvider, mpsc::UnboundedReceiver<serde_json::Value>) {
    let (request_tx, request_rx) = mpsc::unbounded_channel();
    let adapter_version = adapter_version.to_string();
    let agent = Agent
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
        .on_receive_request(
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

    let provider = AcpProvider::connect_with_transport(
        provider_name.to_string(),
        GooseMode::Auto,
        test_config(),
        agent,
    )
    .await
    .unwrap();
    (provider, request_rx)
}

fn boundary_test_provider() -> (Arc<AcpProvider>, mpsc::Receiver<ClientRequest>) {
    let (tx, rx) = mpsc::channel(2);
    let provider = AcpProvider {
        name: CLAUDE_ACP_PROVIDER_NAME.to_string(),
        supports_native_steering: true,
        assistant_message_boundary_pending: Arc::new(AtomicBool::new(false)),
        goose_mode: Arc::new(Mutex::new(GooseMode::Auto)),
        mode_mapping: HashMap::new(),
        session: AcpSession {
            id: SessionId::new(ACP_SESSION_ID),
            response: NewSessionResponse::new(ACP_SESSION_ID),
        },
        pending_confirmations: Arc::new(TokioMutex::new(HashMap::new())),
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

async fn complete_native_steer_with(
    provider: Arc<AcpProvider>,
    requests: &mut mpsc::Receiver<ClientRequest>,
    response: ClaudeSteeringResponse,
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
            if claude_steering::delivery_confirmed(&response) {
                assistant_message_boundary_pending.store(true, Ordering::Release);
            }
            response_tx.send(Ok(response)).unwrap();
        }
        _ => panic!("expected steering request"),
    }
    steer.await.unwrap().unwrap()
}

#[tokio::test]
async fn injected_response_confirms_delivery_and_preserves_request_shape() {
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
    assert_eq!(
        requests.recv().await.unwrap(),
        serde_json::json!({
            "sessionId": ACP_SESSION_ID,
            "prompt": [{ "type": "text", "text": "focus on tests" }],
            "_meta": {
                "steering": {
                    "idleBehavior": "promptRequired"
                }
            }
        })
    );
}

#[tokio::test]
async fn prompt_required_returns_fallback() {
    let (provider, mut requests) = connect_test_provider(
        CLAUDE_ACP_PROVIDER_NAME,
        "0.65.0",
        Some(ClaudeSteeringResponse::PromptRequired),
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
    assert!(!provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));
    assert!(requests.recv().await.is_some());
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
async fn old_claude_adapter_does_not_send_a_steering_request() {
    let (provider, mut requests) = connect_test_provider(
        CLAUDE_ACP_PROVIDER_NAME,
        "0.64.2",
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
async fn empty_or_non_agent_visible_content_is_not_sent() {
    let (provider, mut requests) = connect_test_provider(
        CLAUDE_ACP_PROVIDER_NAME,
        "0.65.0",
        Some(ClaudeSteeringResponse::Injected),
    )
    .await;

    let empty = provider
        .steer_natively("goose-session", &Message::user())
        .await
        .unwrap();
    let user_only = MessageContent::Text(
        RmcpTextContent::new("hidden")
            .with_annotations(Annotations::default().with_audience(vec![Role::User])),
    );
    let non_agent_visible = provider
        .steer_natively("goose-session", &Message::user().with_content(user_only))
        .await
        .unwrap();

    assert!(!empty);
    assert!(!non_agent_visible);
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
async fn steering_completes_while_original_prompt_remains_active() {
    let (prompt_started_tx, mut prompt_started) = mpsc::unbounded_channel();
    let (prompt_responded_tx, mut prompt_responded) = mpsc::unbounded_channel();
    let release_prompt = Arc::new(Notify::new());
    let adapter_version = "0.65.0".to_string();
    let agent = Agent
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
        .on_receive_request(
            {
                let release_prompt = Arc::clone(&release_prompt);
                async move |_request: PromptRequest, responder, cx| {
                    let release_prompt = Arc::clone(&release_prompt);
                    let prompt_responded_tx = prompt_responded_tx.clone();
                    prompt_started_tx.send(()).unwrap();
                    let prompt_cx = cx.clone();
                    cx.spawn(async move {
                        release_prompt.notified().await;
                        prompt_cx.send_notification(SessionNotification::new(
                            ACP_SESSION_ID,
                            SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                ContentBlock::Text(TextContent::new("original prompt update")),
                            )),
                        ))?;
                        let result = responder.respond(PromptResponse::new(StopReason::EndTurn));
                        prompt_responded_tx.send(()).unwrap();
                        result
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

    let provider = AcpProvider::connect_with_transport(
        CLAUDE_ACP_PROVIDER_NAME.to_string(),
        GooseMode::Auto,
        test_config(),
        agent,
    )
    .await
    .unwrap();
    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("start a long task");
    let mut original_stream = provider
        .stream(&model, "", std::slice::from_ref(&prompt), &[])
        .await
        .unwrap();

    timeout(TEST_TIMEOUT, prompt_started.recv())
        .await
        .expect("prompt should start")
        .expect("prompt-start channel should remain open");

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
    let second_result = timeout(TEST_TIMEOUT, second_stream.next())
        .await
        .expect("second prompt should be rejected")
        .expect("second prompt should produce an error");
    assert!(matches!(
        second_result,
        Err(ProviderError::RequestFailed(message))
            if message == "ACP prompt already in progress"
    ));
    assert!(provider
        .assistant_message_boundary_pending
        .load(Ordering::Acquire));

    release_prompt.notify_one();
    timeout(TEST_TIMEOUT, prompt_responded.recv())
        .await
        .expect("prompt should respond after release")
        .expect("prompt-response channel should remain open");
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

    assert!(
        complete_native_steer_with(
            Arc::clone(&provider),
            &mut requests,
            ClaudeSteeringResponse::Injected,
        )
        .await
    );

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

    assert!(
        complete_native_steer_with(provider, &mut requests, ClaudeSteeringResponse::Injected,)
            .await
    );
    let text_after_second = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Text(TextContent::new("text after second steer")),
    )
    .await;
    let thought_after_second = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Thought("thought after second steer".to_string()),
    )
    .await;

    assert_ne!(text_after, text_after_second);
    assert_ne!(thought_after, thought_after_second);
}

#[tokio::test]
async fn prompt_required_preserves_assistant_runs() {
    let (provider, mut requests, response_tx, mut stream) = start_boundary_test_stream().await;
    let text_before = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Text(TextContent::new("text before prompt required")),
    )
    .await;
    let thought_before = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Thought("thought before prompt required".to_string()),
    )
    .await;

    assert!(
        !complete_native_steer_with(
            provider,
            &mut requests,
            ClaudeSteeringResponse::PromptRequired,
        )
        .await
    );

    let text_after = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Text(TextContent::new("text after prompt required")),
    )
    .await;
    let thought_after = send_update_and_message_id(
        &response_tx,
        &mut stream,
        AcpUpdate::Thought("thought after prompt required".to_string()),
    )
    .await;

    assert_eq!(text_before, text_after);
    assert_eq!(thought_before, thought_after);
}
