use goose::agents::moim;
use goose::conversation::message::Message;
use goose::conversation::Conversation;

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

    // MOIM should be at position -1 (before last user message)
    let moim_msg = &result.messages()[2];
    assert!(moim_msg.as_concat_text().contains("Current date and time:"));

    // Last message should be the original last message
    assert_eq!(result.messages()[3].as_concat_text(), "Second message");
}

#[tokio::test]
async fn test_inject_moim_empty_conversation() {
    // Empty conversation
    let conversation = Conversation::empty();

    // Inject MOIM
    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Should have one message now (the MOIM with timestamp)
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
async fn test_moim_always_includes_timestamp() {
    // Even without session or TODO content, MOIM should include timestamp
    let messages = vec![Message::user().with_text("Test")];
    let conversation = Conversation::new_unvalidated(messages);

    let result = moim::inject_moim_if_enabled(conversation, &None).await;

    // Should have MOIM with timestamp
    assert_eq!(result.len(), 2);
    let moim_content = result.messages()[0].as_concat_text();
    assert!(moim_content.contains("Current date and time:"));
    // Should have format like "2025-09-25 18:19:30"
    assert!(moim_content.contains("202")); // Year
    assert!(moim_content.contains(":")); // Time separator
}
