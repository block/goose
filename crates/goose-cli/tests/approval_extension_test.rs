use goose::action_required_manager::ActionRequiredManager;
use goose::agents::extension_manager::{ExtensionManager, ExtensionManagerCapabilities};
use goose::agents::types::SharedProvider;
use goose::builtin_extension::register_builtin_extensions;
use goose::config::{ExtensionConfig, DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::session::SessionManager;
use goose_mcp::BUILTIN_EXTENSIONS;
use rmcp::model::{CallToolRequestParams, RawContent};
use rmcp::object;
use serde_json::json;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

fn first_text(message: &rmcp::model::CallToolResult) -> &str {
    match &message.content[0].raw {
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
async fn builtin_approval_extension_round_trips_elicitation_response() {
    register_builtin_extensions(BUILTIN_EXTENSIONS.clone());
    drain_action_required_queue().await;

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
            ExtensionConfig::Builtin {
                name: "approval".to_string(),
                display_name: Some("Approval".to_string()),
                description: DEFAULT_EXTENSION_DESCRIPTION.to_string(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: None,
                available_tools: vec![],
            },
            None,
            None,
            None,
        )
        .await
        .expect("add approval extension");

    let tools = extension_manager
        .get_prefixed_tools("test-session-id", None)
        .await
        .expect("list tools");
    assert!(
        tools
            .iter()
            .any(|tool| tool.name == "approval__request_approval"),
        "approval builtin should expose the request_approval tool"
    );

    let tool_call = CallToolRequestParams::new("approval__request_approval")
        .with_arguments(object!({ "action_summary": "Deploy database migration v2" }));
    let tool_result = extension_manager
        .dispatch_tool_call("test-session-id", tool_call, None, CancellationToken::new())
        .await
        .expect("dispatch approval tool");

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

    assert!(message.contains("Deploy database migration v2"));
    assert_eq!(
        requested_schema
            .get("properties")
            .and_then(|value| value.get("approved"))
            .and_then(|value| value.get("type"))
            .and_then(|value| value.as_str()),
        Some("boolean")
    );
    assert_eq!(
        requested_schema
            .get("properties")
            .and_then(|value| value.get("reason"))
            .and_then(|value| value.get("type"))
            .and_then(|value| value.as_str()),
        Some("string")
    );

    ActionRequiredManager::global()
        .submit_response(
            id.clone(),
            json!({
                "approved": true,
                "reason": "Reviewed and safe to proceed"
            }),
        )
        .await
        .expect("submit approval response");

    let tool_output = timeout(Duration::from_secs(5), result_handle)
        .await
        .expect("timed out waiting for approval tool result")
        .expect("task join should succeed")
        .expect("tool call should succeed");

    assert_eq!(tool_output.is_error, Some(false));
    assert_eq!(
        first_text(&tool_output),
        "APPROVED: Reviewed and safe to proceed"
    );
    assert_eq!(
        tool_output.structured_content,
        Some(json!({
            "action_summary": "Deploy database migration v2",
            "status": "approved",
            "approved": true,
            "reason": "Reviewed and safe to proceed"
        }))
    );
}
