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
        let idx = messages
            .iter()
            .rposition(|m| m.role == Role::Assistant)
            .unwrap_or(0);
        messages.insert(idx, Message::user().with_text(moim));

        let (fixed, issues) = fix_conversation(Conversation::new_unvalidated(messages));

        let has_unexpected_issues = issues.iter().any(|issue| {
            !issue.contains("Merged consecutive user messages")
                && !issue.contains("Merged consecutive assistant messages")
                && !issue.contains("Merged text content")
                && !issue.contains("Removed orphaned tool response")
                && !issue.contains("Removed orphaned tool request")
                && !issue.contains("Removed empty message")
                && !issue.contains("Removed leading assistant message")
                && !issue.contains("Removed trailing assistant message")
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

    // After the allowlist fix: MOIM now applies the fix when fix_conversation detects an
    // orphaned tool request, removing it from the conversation.
    #[tokio::test]
    async fn test_moim_fixes_orphaned_tool_request() {
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = PathBuf::from("/test/dir");

        let broken_conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Do something"),
            Message::assistant()
                .with_text("I'll call a tool")
                .with_tool_request(
                    "orphan_tool_1",
                    Ok(CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: "some_tool".into(),
                        arguments: None,
                    }),
                ),
        ]);

        // Control: fix_conversation alone correctly removes the orphaned tool request.
        let (_, issues) = fix_conversation(Conversation::new_unvalidated(
            broken_conv.messages().clone(),
        ));
        assert!(
            issues
                .iter()
                .any(|i| i.contains("Removed orphaned tool request")),
            "fix_conversation alone should detect the orphan, but issues were: {:?}",
            issues
        );

        // inject_moim now applies the fix (orphan removal is in the allowlist).
        let result = inject_moim("test-session-id", broken_conv.clone(), &em, &working_dir).await;
        let msgs = result.messages();

        // The orphaned ToolRequest should be removed.
        let has_orphan = msgs.iter().any(|m| {
            m.content.iter().any(|c| {
                matches!(c, crate::conversation::message::MessageContent::ToolRequest(tr) if tr.id == "orphan_tool_1")
            })
        });
        assert!(
            !has_orphan,
            "Orphaned tool request should have been removed by MOIM's fix_conversation"
        );

        // MOIM should have been injected (conversation was fixed).
        let has_moim = msgs.iter().any(|m| {
            m.content
                .iter()
                .any(|c| c.as_text().is_some_and(|t| t.contains("<info-msg>")))
        });
        assert!(
            has_moim,
            "MOIM should have been injected after fixing the orphan"
        );
    }

    // After the allowlist fix: MOIM now fixes the cancellation scenario — removes the
    // orphaned tool request and empty user message, preventing the Anthropic 400 error.
    #[tokio::test]
    async fn test_moim_fixes_cancellation_orphan() {
        let temp_dir = tempfile::tempdir().unwrap();
        let em = ExtensionManager::new_without_provider(temp_dir.path().to_path_buf());
        let working_dir = PathBuf::from("/test/dir");

        // Simulates what the agent produces after cancellation:
        // - Valid prior exchange
        // - Assistant issues a tool call
        // - Pre-allocated user message was never populated (cancellation fired first)
        let broken_conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Search for something"),
            Message::assistant().with_text("I searched and found results"),
            Message::user().with_text("Now do something else"),
            Message::assistant()
                .with_text("I'll call a tool")
                .with_tool_request(
                    "cancelled_tool",
                    Ok(CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: "some_tool".into(),
                        arguments: None,
                    }),
                ),
            // Pre-allocated Message::user() never populated due to cancellation.
            Message::user(),
        ]);

        let result = inject_moim("test-session-id", broken_conv.clone(), &em, &working_dir).await;
        let msgs = result.messages();

        // The orphaned ToolRequest should be removed.
        let has_orphan = msgs.iter().any(|m| {
            m.content.iter().any(|c| {
                matches!(c, crate::conversation::message::MessageContent::ToolRequest(tr) if tr.id == "cancelled_tool")
            })
        });
        assert!(
            !has_orphan,
            "Orphaned tool request should have been removed by MOIM's fix_conversation"
        );

        // The empty user message should be removed.
        let has_empty = msgs.iter().any(|m| m.content.is_empty());
        assert!(
            !has_empty,
            "Empty user message should have been removed by MOIM's fix_conversation"
        );

        // MOIM should have been injected.
        let has_moim = msgs.iter().any(|m| {
            m.content
                .iter()
                .any(|c| c.as_text().is_some_and(|t| t.contains("<info-msg>")))
        });
        assert!(
            has_moim,
            "MOIM should have been injected after fixing the cancellation orphan"
        );
    }
}
