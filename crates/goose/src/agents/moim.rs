use crate::agents::extension_manager::ExtensionManager;
use crate::conversation::message::Message;
use crate::conversation::{fix_conversation, Conversation};
use rmcp::model::Role;
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
        let last_assistant_idx = messages.iter().rposition(|m| m.role == Role::Assistant);

        // If the conversation ends with an assistant message (text-only, no tool
        // call), inserting MOIM *before* it would leave a trailing assistant
        // message, which violates the API constraint that conversations must end
        // with a user message.  Insert after it instead so the MOIM becomes the
        // final (user) message.
        let idx = match last_assistant_idx {
            Some(i) if i == messages.len() - 1 => messages.len(),
            Some(i) => i,
            None => 0,
        };
        messages.insert(idx, Message::user().with_text(moim));

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
    async fn test_moim_injection_before_assistant() {
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

        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content[0].as_text().unwrap(), "Hello");
        assert_eq!(msgs[1].content[0].as_text().unwrap(), "Hi");

        let merged_content = msgs[0]
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

        assert_eq!(msgs.len(), 6);

        let moim_msg = &msgs[3];
        let has_moim = moim_msg
            .content
            .iter()
            .any(|c| c.as_text().is_some_and(|t| t.contains("<info-msg>")));

        assert!(
            has_moim,
            "MOIM should be in message before latest assistant message"
        );
    }

    #[tokio::test]
    async fn test_moim_trailing_assistant_text_only() {
        // Regression test for issue #7425: when the conversation ends with a
        // text-only assistant message (no tool call), MOIM must be inserted
        // *after* it so the conversation ends with a user message.
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = PathBuf::from("/test/dir");

        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("clean up comments in check.sh"),
            Message::assistant()
                .with_text("Let me look at the file")
                .with_tool_request(
                    "tool_1",
                    Ok(CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: "text_editor".into(),
                        arguments: None,
                    }),
                ),
            Message::user().with_tool_response(
                "tool_1",
                Ok(rmcp::model::CallToolResult {
                    content: vec![],
                    structured_content: None,
                    is_error: Some(false),
                    meta: None,
                }),
            ),
            // Trailing assistant message with text only — no tool call
            Message::assistant().with_text("Now I'll clean this up..."),
        ]);

        let result = inject_moim("test-session-id", conv, &em, &working_dir).await;
        let msgs = result.messages();

        // The conversation must end with a user message (the MOIM)
        let last = msgs.last().expect("should have messages");
        assert_eq!(
            last.role,
            Role::User,
            "conversation must end with a user message after MOIM injection"
        );

        // The MOIM should be present
        let has_moim = last
            .content
            .iter()
            .any(|c| c.as_text().is_some_and(|t| t.contains("<info-msg>")));
        assert!(has_moim, "last message should contain MOIM");

        // The trailing assistant message should still be present (not dropped)
        let has_trailing_assistant = msgs.iter().any(|m| {
            m.role == Role::Assistant
                && m.content.iter().any(|c| {
                    c.as_text()
                        .is_some_and(|t| t.contains("Now I'll clean this up"))
                })
        });
        assert!(
            has_trailing_assistant,
            "trailing assistant message should be preserved"
        );
    }
}
