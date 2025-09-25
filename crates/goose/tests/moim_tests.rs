use goose::agents::moim;
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use mcp_core::ToolCall;
use rmcp::model::Content;
use serial_test::serial;

#[tokio::test]
async fn test_inject_moim_preserves_messages() {
    // Create a conversation with multiple messages
    let messages = vec![
        Message::user().with_text("First message"),
        Message::assistant().with_text("First response"),
        Message::user().with_text("Second message"),
    ];
    let conversation = Conversation::new_unvalidated(messages.clone());

    // Inject MOIM (with no session, so only timestamp)
    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Should have one more message (the MOIM with timestamp)
    assert_eq!(result.len(), messages.len() + 1);

    // Original messages should still be present
    assert_eq!(result.messages()[0].as_concat_text(), "First message");
    assert_eq!(result.messages()[1].as_concat_text(), "First response");

    // MOIM should be inserted at a safe position (before last user message)
    let moim_msg = &result.messages()[2];
    assert!(moim_msg.as_concat_text().contains("Current date and time:"));

    // Last message should be the original last message
    assert_eq!(result.messages()[3].as_concat_text(), "Second message");
}

#[tokio::test]
async fn test_inject_moim_empty_conversation() {
    let conversation = Conversation::empty();

    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Empty conversation gets MOIM added
    assert_eq!(result.len(), 1);

    // Should contain timestamp
    assert!(result.messages()[0]
        .as_concat_text()
        .contains("Current date and time:"));
}

#[tokio::test]
async fn test_inject_moim_single_message() {
    // Single message conversation
    let messages = vec![Message::user().with_text("Only message")];
    let conversation = Conversation::new_unvalidated(messages.clone());

    // Inject MOIM
    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Should have MOIM before the single message
    assert_eq!(result.len(), 2);

    // MOIM should be first
    let moim_msg = &result.messages()[0];
    assert!(moim_msg.as_concat_text().contains("Current date and time:"));

    // Original message should be last
    assert_eq!(result.messages()[1].as_concat_text(), "Only message");
}

#[tokio::test]
#[serial]
async fn test_moim_disabled_no_injection() {
    // Temporarily set MOIM to disabled
    // Note: Config uses uppercase for env vars
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
    // Create a conversation with tool call/response pairs
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

    // Should have one more message (the MOIM)
    assert_eq!(result.len(), messages.len() + 1);

    // Verify tool call and response are still adjacent
    let msgs = result.messages();
    for i in 0..msgs.len() - 1 {
        if msgs[i].is_tool_call() {
            // The next message should be the tool response (not MOIM)
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
fn test_find_safe_insertion_point_empty() {
    let messages = vec![];
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 0);
}

#[test]
fn test_find_safe_insertion_point_single_message() {
    let messages = vec![Message::user().with_text("Hello")];
    // Single message, should insert before it
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 0);
}

#[test]
fn test_find_safe_insertion_point_no_tools() {
    let messages = vec![
        Message::user().with_text("Hello"),
        Message::assistant().with_text("Hi there"),
        Message::user().with_text("How are you?"),
    ];
    // Should insert before the last message
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 2);
}

#[test]
fn test_find_safe_insertion_point_with_tool_pair() {
    let tool_call = Ok(ToolCall::new("test_tool", serde_json::json!({})));
    let tool_result = Ok(vec![Content::text("Result")]);

    let messages = vec![
        Message::user().with_text("Do something"),
        Message::assistant().with_tool_request("tool1", tool_call),
        Message::user().with_tool_response("tool1", tool_result),
        Message::assistant().with_text("Done"),
        Message::user().with_text("Thanks"),
    ];

    // Should insert before "Thanks" (index 4), the last message
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 4);
}

#[test]
fn test_find_safe_insertion_point_ending_with_tool_response() {
    let tool_call = Ok(ToolCall::new("test_tool", serde_json::json!({})));
    let tool_result = Ok(vec![Content::text("Result")]);

    let messages = vec![
        Message::user().with_text("Do something"),
        Message::assistant().with_tool_request("tool1", tool_call),
        Message::user().with_tool_response("tool1", tool_result),
    ];

    // Last message is a tool response that pairs with previous tool call
    // Should insert before the tool call instead (index 1)
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 1);
}

#[test]
fn test_find_safe_insertion_point_mixed_content() {
    let messages = vec![
        Message::user().with_text("Start"),
        Message::assistant().with_text("OK"),
        Message::user()
            .with_text("Here's an image")
            .with_image("data", "image/png"),
        Message::assistant().with_text("I see the image"),
        Message::user().with_text("What do you think?"),
    ];

    // Should insert before the last message regardless of content
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 4);
}

#[test]
fn test_find_safe_insertion_point_no_text_only_user_messages() {
    let tool_result = Ok(vec![Content::text("Result")]);
    let messages = vec![
        Message::assistant().with_text("Hello"),
        Message::user().with_tool_response("tool1", tool_result),
    ];

    // Should insert before the tool response, but since previous is not a tool call,
    // it's safe to insert at position 1
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 1);
}

#[test]
fn test_find_safe_insertion_point_tool_response_only() {
    let tool_result = Ok(vec![Content::text("Result")]);
    let messages = vec![
        Message::user().with_text("Start"),
        Message::assistant().with_text("Let me help"),
        Message::user().with_tool_response("tool1", tool_result),
    ];

    // Last message is a tool response but previous is not a tool call
    // Safe to insert at position 2
    assert_eq!(goose::agents::moim::find_safe_insertion_point(&messages), 2);
}
