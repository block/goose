use goose::action_required_manager::ActionRequiredManager;
use goose::agents::extension_manager::{ExtensionManager, ExtensionManagerCapabilities};
use goose::agents::types::SharedProvider;
use goose::config::{ExtensionConfig, DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::session::SessionManager;
use goose_test_support::McpFixture;
use rmcp::model::{CallToolRequestParams, RawContent};
use rmcp::object;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

fn first_text(result: &rmcp::model::CallToolResult) -> &str {
    match &result.content[0].raw {
        RawContent::Text(text) => &text.text,
        _ => panic!("expected text content"),
    }
}

async fn next_elicitation_message() -> Message {
    timeout(Duration::from_secs(5), async {
        ActionRequiredManager::global()
            .request_rx
            .lock()
            .await
            .recv()
            .await
    })
    .await
    .expect("timed out waiting for elicitation request")
    .expect("expected elicitation request")
}

async fn drain_action_required_queue() {
    let mut rx = ActionRequiredManager::global().request_rx.lock().await;
    while rx.try_recv().is_ok() {}
}

#[tokio::test]
async fn streamable_http_extension_round_trips_elicitation_response() {
    drain_action_required_queue().await;

    let fixture = McpFixture::new(None).await;
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let provider: SharedProvider = Arc::new(tokio::sync::Mutex::new(None));
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let extension_manager = Arc::new(ExtensionManager::new(
        provider,
        session_manager,
        "goose-cli".to_string(),
        ExtensionManagerCapabilities { mcpui: false },
    ));

    extension_manager
        .add_extension(
            ExtensionConfig::StreamableHttp {
                name: "fixture".to_string(),
                description: DEFAULT_EXTENSION_DESCRIPTION.to_string(),
                uri: fixture.url.clone(),
                envs: Default::default(),
                env_keys: vec![],
                headers: Default::default(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: None,
                available_tools: vec![],
            },
            None,
            None,
            None,
        )
        .await
        .expect("add streamable http fixture");

    let tools = extension_manager
        .get_prefixed_tools("test-session-id", None)
        .await
        .expect("list tools");
    assert!(
        tools
            .iter()
            .any(|tool| tool.name == "fixture__request_confirmation"),
        "fixture extension should expose the request_confirmation tool"
    );

    let tool_call = CallToolRequestParams::new("fixture__request_confirmation")
        .with_arguments(object!({ "prompt": "Should goose continue with this release?" }));
    let tool_result = extension_manager
        .dispatch_tool_call("test-session-id", tool_call, None, CancellationToken::new())
        .await
        .expect("dispatch elicitation tool");

    let result_handle = tokio::spawn(tool_result.result);

    let elicitation_message = next_elicitation_message().await;
    let MessageContent::ActionRequired(action_required) = &elicitation_message.content[0] else {
        panic!("expected an action required message");
    };

    let ActionRequiredData::Elicitation {
        id,
        message,
        requested_schema,
    } = &action_required.data
    else {
        panic!("expected an elicitation request");
    };

    assert!(message.contains("Should goose continue with this release?"));
    assert_eq!(
        requested_schema
            .get("properties")
            .and_then(|value| value.get("approved"))
            .and_then(|value| value.get("type"))
            .and_then(|value| value.as_str()),
        Some("boolean")
    );

    ActionRequiredManager::global()
        .submit_response(
            id.clone(),
            json!({
                "approved": true,
                "reason": "release checks look good"
            }),
        )
        .await
        .expect("submit elicitation response");

    let tool_output = timeout(Duration::from_secs(5), result_handle)
        .await
        .expect("timed out waiting for tool result")
        .expect("task join should succeed")
        .expect("tool result should succeed");

    assert_eq!(
        first_text(&tool_output),
        "approved=true; reason=release checks look good"
    );
}
