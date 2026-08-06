use super::claude_steering_completion_workaround::ClaudeSteeringCompletionWorkaround;
use super::*;
use agent_client_protocol::schema::v1::{ExtNotification, Implementation};
use futures::StreamExt;
use serde_json::value::RawValue;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const ACP_SESSION_ID: &str = "claude-test-session";
const PROMPT_COMMAND_UUID: &str = "prompt-command";
const STEERING_COMMAND_UUID: &str = "steering-command";

struct SteeringFixture {
    provider: AcpProvider,
    prompt_started: mpsc::UnboundedReceiver<()>,
    release_prompt: Arc<Notify>,
    steer_requests: mpsc::UnboundedReceiver<serde_json::Value>,
}

fn supported_workaround() -> ClaudeSteeringCompletionWorkaround {
    let workaround = ClaudeSteeringCompletionWorkaround::default();
    observe_sdk_message(
        &workaround,
        serde_json::json!({
            "type": "system",
            "subtype": "init",
            "capabilities": ["msg_lifecycle_v1"],
        }),
    );
    workaround
}

fn sdk_ext_notification(message: serde_json::Value) -> ExtNotification {
    let params = RawValue::from_string(
        serde_json::json!({
            "sessionId": ACP_SESSION_ID,
            "message": message,
        })
        .to_string(),
    )
    .unwrap();
    ExtNotification::new("_claude/sdkMessage", Arc::from(params))
}

fn sdk_message_notification(message: serde_json::Value) -> AgentNotification {
    AgentNotification::ExtNotification(sdk_ext_notification(message))
}

fn observe_sdk_message(
    workaround: &ClaudeSteeringCompletionWorkaround,
    message: serde_json::Value,
) {
    let notification = sdk_ext_notification(message);
    let normalized = ExtNotification::new("claude/sdkMessage", notification.params);
    let _ = workaround.observe_notification(&normalized);
}

fn boundary_test_provider() -> (Arc<AcpProvider>, mpsc::Receiver<ClientRequest>) {
    boundary_test_provider_with(supported_workaround())
}

fn boundary_test_provider_with(
    workaround: ClaudeSteeringCompletionWorkaround,
) -> (Arc<AcpProvider>, mpsc::Receiver<ClientRequest>) {
    let (tx, rx) = mpsc::channel(2);
    let provider = AcpProvider {
        name: CLAUDE_ACP_PROVIDER_NAME.to_string(),
        supports_native_steering: true,
        claude_steering_workaround: Some(workaround),
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

async fn next_text(stream: &mut MessageStream) -> String {
    let (message, _) = timeout(TEST_TIMEOUT, stream.next())
        .await
        .expect("provider stream should produce text")
        .expect("provider stream should remain open")
        .expect("provider update should succeed");
    message
        .expect("provider update should contain a message")
        .content
        .into_iter()
        .find_map(|content| match content {
            MessageContent::Text(text) => Some(text.text),
            _ => None,
        })
        .expect("provider message should contain text")
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
                    cx.send_notification(sdk_message_notification(serde_json::json!({
                        "type": "user",
                        "uuid": PROMPT_COMMAND_UUID,
                    })))?;
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
                async move |request: ClaudeSteeringRequest, responder, cx| {
                    let steer_request_tx = steer_request_tx.clone();
                    let steer_response = steer_response.clone();
                    steer_request_tx
                        .send(serde_json::to_value(request).unwrap())
                        .unwrap();
                    if matches!(steer_response, ClaudeSteeringResponse::Injected) {
                        cx.send_notification(sdk_message_notification(serde_json::json!({
                            "type": "user",
                            "uuid": STEERING_COMMAND_UUID,
                        })))?;
                        cx.send_notification(sdk_message_notification(serde_json::json!({
                            "type": "command_lifecycle",
                            "command_uuid": STEERING_COMMAND_UUID,
                            "state": "completed",
                        })))?;
                    }
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
    observe_sdk_message(
        provider.claude_steering_workaround.as_ref().unwrap(),
        serde_json::json!({
            "type": "system",
            "subtype": "init",
            "capabilities": ["msg_lifecycle_v1"],
        }),
    );

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
async fn cancelled_claude_steer_releases_client_request_loop() {
    let (steer_responder_tx, mut steer_responders) = mpsc::unbounded_channel();
    let (mode_received_tx, mut mode_received) = mpsc::unbounded_channel();

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
            async move |_request: ClaudeSteeringRequest, responder, _cx| {
                steer_responder_tx.send(responder).unwrap();
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: SetSessionModeRequest, responder, _cx| {
                mode_received_tx.send(()).unwrap();
                responder.respond(SetSessionModeResponse::new())
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
    let provider = Arc::new(
        AcpProvider::connect_with_transport(
            CLAUDE_ACP_PROVIDER_NAME.to_string(),
            GooseMode::Auto,
            config,
            agent,
        )
        .await
        .unwrap(),
    );
    observe_sdk_message(
        provider.claude_steering_workaround.as_ref().unwrap(),
        serde_json::json!({
            "type": "system",
            "subtype": "init",
            "capabilities": ["msg_lifecycle_v1"],
        }),
    );

    let provider_for_steer = Arc::clone(&provider);
    let steer = tokio::spawn(async move {
        provider_for_steer
            .steer_natively(
                "goose-session",
                &Message::user().with_text("Focus on the tests"),
            )
            .await
    });
    let _steer_responder = timeout(TEST_TIMEOUT, steer_responders.recv())
        .await
        .expect("steering request should start")
        .expect("steering request channel should remain open");

    steer.abort();
    assert!(steer.await.unwrap_err().is_cancelled());

    timeout(
        TEST_TIMEOUT,
        provider.send_set_mode("goose-session", "default".to_string()),
    )
    .await
    .expect("the client loop should accept another request")
    .unwrap();
    timeout(TEST_TIMEOUT, mode_received.recv())
        .await
        .expect("mode request should arrive")
        .expect("mode request channel should remain open");
}

#[tokio::test]
async fn keeps_stream_open_until_injected_command_completes() {
    let release_prompt = Arc::new(Notify::new());
    let (prompt_started_tx, mut prompt_started) = mpsc::unbounded_channel();
    let (prompt_responded_tx, mut prompt_responded) = mpsc::unbounded_channel();
    let (agent_connection_tx, mut agent_connection) = mpsc::unbounded_channel();

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
            async |request: NewSessionRequest, responder, _cx| {
                assert_eq!(
                    request
                        .meta
                        .as_ref()
                        .and_then(|meta| meta.get("claudeCode"))
                        .and_then(|value| value.get("emitRawSDKMessages"))
                        .and_then(serde_json::Value::as_array)
                        .map(Vec::len),
                    Some(3)
                );
                responder.respond(NewSessionResponse::new(ACP_SESSION_ID))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let release_prompt = release_prompt.clone();
                async move |_request: PromptRequest, responder, cx| {
                    let release_prompt = release_prompt.clone();
                    let prompt_responded_tx = prompt_responded_tx.clone();
                    cx.send_notification(sdk_message_notification(serde_json::json!({
                        "type": "system",
                        "subtype": "init",
                        "capabilities": ["msg_lifecycle_v1"],
                    })))?;
                    cx.send_notification(sdk_message_notification(serde_json::json!({
                        "type": "user",
                        "uuid": PROMPT_COMMAND_UUID,
                    })))?;
                    cx.send_notification(SessionNotification::new(
                        ACP_SESSION_ID,
                        SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                            TextContent::new("original assistant text"),
                        ))),
                    ))?;
                    prompt_started_tx.send(()).unwrap();
                    cx.spawn(async move {
                        release_prompt.notified().await;
                        let result = responder.respond(PromptResponse::new(StopReason::EndTurn));
                        prompt_responded_tx.send(()).unwrap();
                        result
                    })
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: ClaudeSteeringRequest, responder, cx| {
                cx.send_notification(sdk_message_notification(serde_json::json!({
                    "type": "user",
                    "uuid": STEERING_COMMAND_UUID,
                })))?;
                agent_connection_tx.send(cx).unwrap();
                responder.respond(ClaudeSteeringResponse::Injected)
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

    let model = ModelConfig::new("test-model");
    let prompt = Message::user().with_text("Start a long task");
    let mut stream = provider.stream(&model, "", &[prompt], &[]).await.unwrap();
    timeout(TEST_TIMEOUT, prompt_started.recv())
        .await
        .expect("prompt should start")
        .expect("prompt channel should remain open");
    timeout(TEST_TIMEOUT, async {
        while !provider
            .claude_steering_workaround
            .as_ref()
            .unwrap()
            .native_steering_available()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("lifecycle capability should be observed");
    assert_eq!(next_text(&mut stream).await, "original assistant text");

    assert!(provider
        .steer_natively(
            "goose-session",
            &Message::user().with_text("Focus on the tests"),
        )
        .await
        .unwrap());
    let connection = timeout(TEST_TIMEOUT, agent_connection.recv())
        .await
        .expect("agent connection should arrive")
        .expect("agent connection channel should remain open");

    release_prompt.notify_one();
    timeout(TEST_TIMEOUT, prompt_responded.recv())
        .await
        .expect("original prompt should respond")
        .expect("prompt response channel should remain open");

    connection
        .send_notification(SessionNotification::new(
            ACP_SESSION_ID,
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("steered assistant text"),
            ))),
        ))
        .unwrap();
    assert_eq!(next_text(&mut stream).await, "steered assistant text");

    connection
        .send_notification(sdk_message_notification(serde_json::json!({
            "type": "command_lifecycle",
            "command_uuid": STEERING_COMMAND_UUID,
            "state": "completed",
        })))
        .unwrap();
    assert!(timeout(TEST_TIMEOUT, stream.next())
        .await
        .expect("provider stream should complete after lifecycle completion")
        .is_none());
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
async fn lifecycle_capability_absent_skips_native_steering_request() {
    let (provider, mut requests) =
        boundary_test_provider_with(ClaudeSteeringCompletionWorkaround::default());

    let outcome = provider
        .steer_natively(
            "goose-session",
            &Message::user().with_text("Focus on the tests"),
        )
        .await
        .unwrap();

    assert!(!outcome);
    assert!(matches!(
        requests.try_recv(),
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
