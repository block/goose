use goose::tool_monitor::RepetitionInspector;
use goose::{
    config::GooseMode,
    conversation::message::{Message, MessageContent, ToolRequest},
    tool_inspection::{InspectionAction, ToolInspector},
};
use rmcp::model::CallToolRequestParams;
use rmcp::object;

// This test targets RepetitionInspector::check_tool_call
// It verifies that:
// - consecutive identical tool calls are allowed up to max_repetitions times
// - the (max_repetitions + 1)th identical call is denied (returns false)
// - changing the parameters resets the repetition count and allows the call
#[test]
fn test_repetition_inspector_denies_after_exceeding_and_resets_on_param_change() {
    // Allow at most 2 consecutive identical calls
    let mut inspector = RepetitionInspector::new(Some(2));

    // First identical call → allowed
    let call_v1 = CallToolRequestParams::new("fetch_user").with_arguments(object!({"id": 123}));
    assert!(inspector.check_tool_call(call_v1.clone()));

    // Second identical call → still allowed (at limit)
    assert!(inspector.check_tool_call(call_v1.clone()));

    // Third identical call → should be denied (exceeds limit)
    assert!(!inspector.check_tool_call(call_v1.clone()));

    // Change parameters; this should reset the consecutive counter
    let call_v2 = CallToolRequestParams::new("fetch_user").with_arguments(object!({"id": 456}));

    assert!(inspector.check_tool_call(call_v2.clone()));

    // Another identical call with new params → allowed (second in a row for this variant)
    assert!(inspector.check_tool_call(call_v2.clone()));

    // One more identical call with new params → denied again
    assert!(!inspector.check_tool_call(call_v2));
}

#[tokio::test]
async fn inspect_persists_repetition_state_between_calls() {
    let inspector = RepetitionInspector::new(Some(2));
    let tool_call =
        CallToolRequestParams::new("developer__shell").with_arguments(object!({"command": "pwd"}));

    assert!(inspector
        .inspect(
            "test-session",
            &[tool_request("call_1", tool_call.clone())],
            &[],
            GooseMode::Auto,
        )
        .await
        .unwrap()
        .is_empty());
    assert!(inspector
        .inspect(
            "test-session",
            &[tool_request("call_2", tool_call.clone())],
            &[],
            GooseMode::Auto,
        )
        .await
        .unwrap()
        .is_empty());

    let results = inspector
        .inspect(
            "test-session",
            &[tool_request("call_3", tool_call)],
            &[],
            GooseMode::Auto,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].tool_request_id, "call_3");
    assert_eq!(results[0].action, InspectionAction::Deny);
    assert_eq!(results[0].inspector_name, "repetition");
}

#[tokio::test]
async fn inspect_resets_repetition_state_for_new_user_input() {
    let inspector = RepetitionInspector::new(Some(2));
    let tool_call =
        CallToolRequestParams::new("developer__shell").with_arguments(object!({"command": "pwd"}));
    let first_turn = [Message::user().with_content(MessageContent::text("check pwd"))];

    assert!(inspector
        .inspect(
            "test-session",
            &[tool_request("call_1", tool_call.clone())],
            &first_turn,
            GooseMode::Auto,
        )
        .await
        .unwrap()
        .is_empty());
    assert!(inspector
        .inspect(
            "test-session",
            &[tool_request("call_2", tool_call.clone())],
            &first_turn,
            GooseMode::Auto,
        )
        .await
        .unwrap()
        .is_empty());

    let second_turn = [
        Message::user().with_content(MessageContent::text("check pwd")),
        Message::assistant().with_content(MessageContent::text("done")),
        Message::user().with_content(MessageContent::text("check pwd again")),
    ];

    assert!(inspector
        .inspect(
            "test-session",
            &[tool_request("call_3", tool_call)],
            &second_turn,
            GooseMode::Auto,
        )
        .await
        .unwrap()
        .is_empty());
}

fn tool_request(id: &str, tool_call: CallToolRequestParams) -> ToolRequest {
    match MessageContent::tool_request(id, Ok(tool_call)) {
        MessageContent::ToolRequest(request) => request,
        _ => unreachable!("tool_request constructor should return ToolRequest content"),
    }
}
