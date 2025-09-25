use crate::agents::types::SessionConfig;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::extension_data::ExtensionState;
use crate::session::{self, TodoState};
use chrono::Local;

async fn build_moim_content(session: &Option<SessionConfig>) -> Option<String> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut content = format!("Current date and time: {}\n", timestamp);

    if let Some(todo_content) = get_todo_context(session).await {
        content.push_str("\nCurrent tasks and notes:\n");
        content.push_str(&todo_content);
        content.push('\n');
    }

    Some(content)
}

pub async fn inject_moim_if_enabled(
    messages_for_provider: Conversation,
    session: &Option<SessionConfig>,
) -> Conversation {
    // Check if MOIM is enabled
    let moim_enabled = crate::config::Config::global()
        .get_param::<bool>("goose_moim_enabled")
        .unwrap_or(true);

    if !moim_enabled {
        return messages_for_provider;
    }

    if let Some(moim_content) = build_moim_content(session).await {
        let mut msgs = messages_for_provider.messages().to_vec();

        if !msgs.is_empty() {
            let moim_msg = Message::user().with_text(moim_content);

            // Insert at position -1 (before last message)
            // This ensures MOIM appears just before the latest user query
            let insert_pos = msgs.len().saturating_sub(1);
            msgs.insert(insert_pos, moim_msg);
        }

        Conversation::new_unvalidated(msgs)
    } else {
        messages_for_provider
    }
}

async fn get_todo_context(session: &Option<SessionConfig>) -> Option<String> {
    let session_config = session.as_ref()?;

    match session::storage::get_path(session_config.id.clone()) {
        Ok(path) => match session::storage::read_metadata(&path) {
            Ok(metadata) => TodoState::from_extension_data(&metadata.extension_data)
                .map(|state| state.content)
                .filter(|content| !content.trim().is_empty()),
            Err(e) => {
                tracing::debug!("Could not read session metadata for MOIM: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::debug!("Could not get session path for MOIM: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::conversation::Conversation;

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
        let result = inject_moim_if_enabled(conversation, &None).await;

        // Should have one more message (the MOIM)
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
        let result = inject_moim_if_enabled(conversation, &None).await;

        // Should still be empty (no place to inject)
        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_inject_moim_single_message() {
        // Single message conversation
        let messages = vec![Message::user().with_text("Only message")];
        let conversation = Conversation::new_unvalidated(messages.clone());

        // Inject MOIM
        let result = inject_moim_if_enabled(conversation, &None).await;

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
        std::env::set_var("GOOSE_MOIM_ENABLED", "false");

        let messages = vec![
            Message::user().with_text("Test message"),
            Message::assistant().with_text("Test response"),
        ];
        let conversation = Conversation::new_unvalidated(messages.clone());

        // Try to inject MOIM
        let result = inject_moim_if_enabled(conversation, &None).await;

        // Should be unchanged
        assert_eq!(result.len(), messages.len());
        assert_eq!(result.messages()[0].as_concat_text(), "Test message");
        assert_eq!(result.messages()[1].as_concat_text(), "Test response");

        // Clean up
        std::env::remove_var("GOOSE_MOIM_ENABLED");
    }
}
