use crate::agents::extension_manager::ExtensionManager;
use crate::conversation::message::Message;
use crate::conversation::{fix_conversation, Conversation};
use std::path::Path;

// Test-only utility. Do not use in production code. No `test` directive due to call outside crate.
thread_local! {
    pub static SKIP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub async fn inject_moim(
    session_id: &str,
    conversation: Conversation,
    extension_manager: &ExtensionManager,
    working_dir: &Path,
) -> Conversation {
    if SKIP.with(|f| f.get()) {
        return conversation;
    }

    if let Some(moim) = extension_manager
        .collect_moim(session_id, working_dir)
        .await
    {
        let mut messages = conversation.messages().clone();
        // Append MOIM at the end to preserve cache prefix
        messages.push(Message::user().with_text(moim));

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        let has_unexpected_issues = issues.iter().any(|issue| {
            !issue.contains("Merged consecutive user messages")
                && !issue.contains("Merged consecutive assistant messages")
        });

        if has_unexpected_issues {
            tracing::warn!("MOIM injection caused unexpected issues: {:?}", issues);
            return conversation;
        }

        return fixed;
    }
    conversation
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParams;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_moim_injection_at_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = PathBuf::from("/test/dir");

        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi"),
            Message::user().with_text("Bye"),
        ]);
        let result = inject_moim("test-session-id", conv, &em, &working_dir).await;
        let msgs = result.messages();

        // MOIM is appended at the end and merged with last user message
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content[0].as_text().unwrap(), "Hello");
        assert_eq!(msgs[1].content[0].as_text().unwrap(), "Hi");

        // Last message should contain both "Bye" and MOIM
        let merged_content = msgs[2]
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("");
        assert!(merged_content.contains("Bye"));
        assert!(merged_content.contains("<info-msg>"));
        assert!(merged_content.contains("Working directory: /test/dir"));
    }

    #[tokio::test]
    async fn test_moim_injection_no_assistant() {
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = PathBuf::from("/test/dir");

        let conv = Conversation::new_unvalidated(vec![Message::user().with_text("Hello")]);
        let result = inject_moim("test-session-id", conv, &em, &working_dir).await;

        assert_eq!(result.messages().len(), 1);

        let merged_content = result.messages()[0]
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("");
        assert!(merged_content.contains("Hello"));
        assert!(merged_content.contains("<info-msg>"));
        assert!(merged_content.contains("Working directory: /test/dir"));
    }

    #[tokio::test]
    async fn test_moim_with_tool_calls() {
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = PathBuf::from("/test/dir");

        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Search for something"),
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: "search".into(),
                        arguments: None,
                    }),
                ),
            Message::user().with_tool_response(
                "search_1",
                Ok(rmcp::model::CallToolResult {
                    content: vec![],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            ),
            Message::assistant()
                .with_text("I need to search more")
                .with_tool_request(
                    "search_2",
                    Ok(CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: "search".into(),
                        arguments: None,
                    }),
                ),
            Message::user().with_tool_response(
                "search_2",
                Ok(rmcp::model::CallToolResult {
                    content: vec![],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            ),
        ]);

        let result = inject_moim("test-session-id", conv, &em, &working_dir).await;
        let msgs = result.messages();

        // MOIM is appended at end (may or may not merge depending on fix_conversation)
        // Original: 5 messages, with MOIM appended = 6, then merged = 5 or 6
        assert!(msgs.len() >= 5);

        // Last message should contain MOIM
        let moim_msg = msgs.last().unwrap();
        let has_moim = moim_msg
            .content
            .iter()
            .any(|c| c.as_text().is_some_and(|t| t.contains("<info-msg>")));

        assert!(has_moim, "MOIM should be appended at the end");
    }
}
