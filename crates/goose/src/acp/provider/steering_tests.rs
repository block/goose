use super::*;
use agent_client_protocol::schema::v1::Implementation;
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const ACP_SESSION_ID: &str = "claude-test-session";

struct SteeringFixture {
    provider: AcpProvider,
    prompt_started: mpsc::UnboundedReceiver<()>,
    release_prompt: Arc<Notify>,
    steer_requests: mpsc::UnboundedReceiver<serde_json::Value>,
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

async fn start_boundary_test_stream() -> (
    Arc<AcpProvider>,
    mpsc::Receiver<ClientRequest>,
    mpsc::Sender<AcpUpdate>,
    MessageStream,
) {
    let (provider, mut requests) = boundary_test_provider();
    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("Start a long task");
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

async fn complete_native_steer_with(
    provider: Arc<AcpProvider>,
    requests: &mut mpsc::Receiver<ClientRequest>,
    response: ClaudeSteeringResponse,
) {
    let steer = tokio::spawn(async move {
        provider
            .steer_natively(
                "goose-session",
                &Message::user().with_text("Focus on the tests"),
            )
            .await
    });
    match timeout(TEST_TIMEOUT, requests.recv())
        .await
        .expect("steering request should arrive")
        .expect("request channel should remain open")
    {
        ClientRequest::ClaudeSteer { response_tx, .. } => {
            response_tx.send(Ok(response)).unwrap();
        }
        _ => panic!("expected steering request"),
    }
    steer.await.unwrap().unwrap();
}

async fn steering_fixture(steer_response: ClaudeSteeringResponse) -> SteeringFixture {
    let (prompt_started_tx, prompt_started) = mpsc::unbounded_channel();
    let release_prompt = Arc::new(Notify::new());
    let (steer_request_tx, steer_requests) = mpsc::unbounded_channel();

    let agent = Agent
        .builder()
        .on_receive_request(
            async |request: InitializeRequest, responder, _cx| {
                let mut meta = serde_json::Map::new();
                meta.insert(
                    "steering".to_string(),
                    serde_json::json!({ "supported": true }),
                );
                responder.respond(
                    InitializeResponse::new(request.protocol_version)
                        .agent_info(Implementation::new("claude-agent-acp", "0.64.0"))
                        .meta(meta),
                )
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
                let release_prompt = release_prompt.clone();
                async move |_request: PromptRequest, responder, cx| {
                    let prompt_started_tx = prompt_started_tx.clone();
                    let release_prompt = release_prompt.clone();
                    prompt_started_tx.send(()).unwrap();
                    cx.spawn(async move {
                        release_prompt.notified().await;
                        responder.respond(PromptResponse::new(StopReason::EndTurn))
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                async move |request: ClaudeSteeringRequest, responder, _cx| {
                    let steer_request_tx = steer_request_tx.clone();
                    let steer_response = steer_response.clone();
                    steer_request_tx
                        .send(serde_json::to_value(request).unwrap())
                        .unwrap();
                    responder.respond(steer_response)
                }
            },
            agent_client_protocol::on_receive_request!(),
        );

    let config = AcpProviderConfig {
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
    };
    let provider = AcpProvider::connect_with_transport(
        CLAUDE_ACP_PROVIDER_NAME.to_string(),
        GooseMode::Auto,
        config,
        agent,
    )
    .await
    .unwrap();

    SteeringFixture {
        provider,
        prompt_started,
        release_prompt,
        steer_requests,
    }
}

#[tokio::test]
async fn sends_claude_steer_while_prompt_is_active() {
    let SteeringFixture {
        provider,
        mut prompt_started,
        release_prompt,
        mut steer_requests,
    } = steering_fixture(ClaudeSteeringResponse::Injected).await;

    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("Start a long task");
    let mut stream = provider.stream(&model, "", &[prompt], &[]).await.unwrap();
    timeout(TEST_TIMEOUT, prompt_started.recv())
        .await
        .expect("prompt should start")
        .expect("prompt channel should remain open");

    let steer = Message::user().with_text("Focus on the tests");
    let steer_result = timeout(
        TEST_TIMEOUT,
        provider.steer_natively("goose-session", &steer),
    )
    .await;
    release_prompt.notify_one();

    let request = timeout(TEST_TIMEOUT, steer_requests.recv())
        .await
        .expect("steering request should arrive")
        .expect("steering request channel should remain open");
    timeout(TEST_TIMEOUT, async {
        while let Some(item) = stream.next().await {
            item.unwrap();
        }
    })
    .await
    .expect("prompt stream should finish");

    assert!(steer_result
        .expect("steering should complete before prompt release")
        .unwrap());
    assert_eq!(
        request,
        serde_json::json!({
            "sessionId": ACP_SESSION_ID,
            "prompt": [{ "type": "text", "text": "Focus on the tests" }],
            "_meta": {
                "steering": {
                    "idleBehavior": "promptRequired"
                }
            }
        })
    );
}

#[tokio::test]
async fn prompt_required_does_not_consume_or_start_prompt() {
    let mut fixture = steering_fixture(ClaudeSteeringResponse::PromptRequired {
        reason: ClaudePromptRequiredReason::NoRunningTurn,
    })
    .await;

    let steer = Message::user().with_text("Focus on the tests");
    let outcome = timeout(
        TEST_TIMEOUT,
        fixture.provider.steer_natively("goose-session", &steer),
    )
    .await
    .expect("steering should complete")
    .unwrap();
    timeout(TEST_TIMEOUT, fixture.steer_requests.recv())
        .await
        .expect("steering request should arrive")
        .expect("steering request channel should remain open");

    assert!(!outcome);
    assert!(matches!(
        fixture.prompt_started.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn injected_steer_starts_new_assistant_runs_once() {
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

    complete_native_steer_with(provider, &mut requests, ClaudeSteeringResponse::Injected).await;

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

    complete_native_steer_with(
        provider,
        &mut requests,
        ClaudeSteeringResponse::PromptRequired {
            reason: ClaudePromptRequiredReason::NoRunningTurn,
        },
    )
    .await;

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
