use crate::agents::extension_manager::ExtensionManager;
use crate::conversation::message::Message;
use crate::conversation::{fix_conversation, Conversation};
use rmcp::model::Role;

#[doc(hidden)]
#[cfg(any(test, feature = "test-support"))]
thread_local! {
    pub static SKIP: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

pub async fn inject_moim(
    conversation: Conversation,
    extension_manager: &ExtensionManager,
) -> Conversation {
    #[cfg(any(test, feature = "test-support"))]
    if SKIP.with(|f| f.get()) {
        return conversation;
    }

    if let Some(moim) = extension_manager.collect_moim().await {
        let mut messages = conversation.messages().clone();
        let idx = messages
            .iter()
            .rposition(|m| m.role == Role::Assistant)
            .unwrap_or(messages.len());
        messages.insert(idx, Message::user().with_text(moim));

        let (fixed, _issues) = fix_conversation(Conversation::new_unvalidated(messages));
        return fixed;
    }
    conversation
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParam;

    #[tokio::test]
    async fn test_moim_injection_before_assistant() {
        let em = ExtensionManager::new_without_provider();

        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi"),
            Message::user().with_text("Bye"),
            Message::assistant().with_text("Goodbye"),
        ]);
        let result = inject_moim(conv, &em).await;
        let msgs = result.messages();

        // MOIM gets inserted before last assistant, then merged with "Bye",
        // but then the trailing assistant gets removed by fix_conversation
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].content[0].as_text().unwrap(), "Hello");
        assert_eq!(msgs[1].content[0].as_text().unwrap(), "Hi");

        // The third message should be the merged "Bye" + MOIM
        let merged_content = msgs[2]
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("");
        assert!(merged_content.contains("Bye"));
        assert!(merged_content.contains("<info-msg>"));
    }

    #[tokio::test]
    async fn test_moim_injection_no_assistant() {
        let em = ExtensionManager::new_without_provider();

        let conv = Conversation::new_unvalidated(vec![Message::user().with_text("Hello")]);
        let result = inject_moim(conv, &em).await;

        // MOIM gets merged with the existing user message
        assert_eq!(result.messages().len(), 1);

        let merged_content = result.messages()[0]
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join("");
        assert!(merged_content.contains("Hello"));
        assert!(merged_content.contains("<info-msg>"));
    }

    #[tokio::test]
    async fn test_moim_with_tool_calls() {
        let em = ExtensionManager::new_without_provider();

        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Search for something"),
            Message::assistant()
                .with_text("I'll search for you")
                .with_tool_request(
                    "search_1",
                    Ok(CallToolRequestParam {
                        name: "search".into(),
                        arguments: None,
                    }),
                ),
            Message::user().with_tool_response("search_1", Ok(vec![])),
            Message::assistant().with_text("Found results"),
        ]);

        let result = inject_moim(conv, &em).await;
        let msgs = result.messages();

        // MOIM gets inserted as separate message, trailing assistant removed
        assert_eq!(msgs.len(), 4);

        let tool_request_idx = msgs
            .iter()
            .position(|m| {
                m.content.iter().any(|c| {
                    matches!(
                        c,
                        crate::conversation::message::MessageContent::ToolRequest(_)
                    )
                })
            })
            .unwrap();

        let tool_response_idx = msgs
            .iter()
            .position(|m| {
                m.content.iter().any(|c| {
                    matches!(
                        c,
                        crate::conversation::message::MessageContent::ToolResponse(_)
                    )
                })
            })
            .unwrap();

        // MOIM should be in a separate message after tool response
        let moim_msg = &msgs[3];
        let has_moim = moim_msg
            .content
            .iter()
            .any(|c| c.as_text().map_or(false, |t| t.contains("<info-msg>")));

        assert!(has_moim, "MOIM should be in the last message");
        assert_eq!(
            tool_response_idx,
            tool_request_idx + 1,
            "Tool response should immediately follow tool request"
        );
    }
}
