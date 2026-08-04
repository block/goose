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
    let consumed = timeout(
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

    assert!(!consumed);
    assert!(matches!(
        fixture.prompt_started.try_recv(),
        Err(mpsc::error::TryRecvError::Empty)
    ));
}
