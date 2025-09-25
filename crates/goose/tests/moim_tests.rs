use goose::agents::moim;
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use mcp_core::ToolCall;
use rmcp::model::Content;
use serial_test::serial;

#[tokio::test]
async fn test_moim_basic_injection() {
    let messages = vec![Message::user().with_text("Only message")];
    let conversation = Conversation::new_unvalidated(messages.clone());

    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Should inject MOIM with timestamp
    assert_eq!(result.len(), 2);
    assert!(result.messages()[0]
        .as_concat_text()
        .contains("Current date and time:"));
}

#[tokio::test]
#[serial]
async fn test_moim_disabled_no_injection() {
    std::env::set_var("GOOSE_MOIM_ENABLED", "false");

    let messages = vec![
        Message::user().with_text("Test message"),
        Message::assistant().with_text("Test response"),
    ];
    let conversation = Conversation::new_unvalidated(messages.clone());

    // Try to inject MOIM
    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Should be unchanged when disabled
    assert_eq!(result.len(), messages.len());
    assert_eq!(result.messages()[0].as_concat_text(), "Test message");
    assert_eq!(result.messages()[1].as_concat_text(), "Test response");

    // Clean up
    std::env::remove_var("GOOSE_MOIM_ENABLED");
}

#[tokio::test]
async fn test_moim_respects_tool_pairs() {
    // Critical test: ensure MOIM doesn't break tool call/response pairs
    let tool_call = Ok(ToolCall::new(
        "test_tool",
        serde_json::json!({"param": "value"}),
    ));
    let tool_result = Ok(vec![Content::text("Tool executed successfully")]);

    let messages = vec![
        Message::user().with_text("Please use the tool"),
        Message::assistant()
            .with_text("I'll use the tool for you")
            .with_tool_request("tool1", tool_call),
        Message::user().with_tool_response("tool1", tool_result),
        Message::assistant().with_text("The tool has been executed"),
        Message::user().with_text("Thank you"),
    ];

    let conversation = Conversation::new_unvalidated(messages.clone());

    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    assert_eq!(result.len(), messages.len() + 1);

    // Verify tool call and response are still adjacent
    let msgs = result.messages();
    for i in 0..msgs.len() - 1 {
        if msgs[i].is_tool_call() {
            assert!(
                msgs[i + 1].is_tool_response()
                    || !msgs[i + 1]
                        .as_concat_text()
                        .contains("Current date and time:"),
                "MOIM should not be inserted between tool call and response"
            );
        }
    }
}

#[test]
fn test_find_safe_insertion_point_ending_with_tool_response() {
    // Critical test: when conversation ends with tool response, don't break the pair
    let tool_call = Ok(ToolCall::new("test_tool", serde_json::json!({})));
    let tool_result = Ok(vec![Content::text("Result")]);

    let messages = vec![
        Message::user().with_text("Do something"),
        Message::assistant().with_tool_request("tool1", tool_call),
        Message::user().with_tool_response("tool1", tool_result),
    ];

    // Should insert before the tool call instead (index 1)
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 1);
}
