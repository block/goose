use super::*;
use agent_client_protocol::schema::v1::Implementation;
use rmcp::model::{Annotations, TextContent as RmcpTextContent};

const ACP_SESSION_ID: &str = "claude-test-session";

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
                let mut meta = serde_json::Map::new();
                meta.insert(
                    "steering".to_string(),
                    serde_json::json!({ "supported": true }),
                );
                responder.respond(
                    InitializeResponse::new(request.protocol_version)
                        .agent_info(Implementation::new(
                            "claude-agent-acp",
                            adapter_version.clone(),
                        ))
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
    assert!(requests.recv().await.is_some());
}
