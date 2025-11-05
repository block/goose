use crate::agents::extension_manager::ExtensionManager;
use crate::agents::SessionConfig;
use crate::conversation::message::{Message, MessageContent};
use rmcp::model::{AnnotateAble, RawTextContent, Role};
use uuid::Uuid;

/// Inject MOIM (Minus One Info Message) into conversation.
///
/// MOIM provides ephemeral context that's included in LLM calls
/// but never persisted to conversation history.
///
/// The MOIM content is prepended to the latest user message to ensure
/// it appears in the correct context position.
pub async fn inject_moim(
    messages: &[Message],
    extension_manager: &ExtensionManager,
    _session: &Option<SessionConfig>,
) -> Vec<Message> {
    let moim_content = match extension_manager.collect_moim().await {
        Some(content) if !content.trim().is_empty() => content,
        _ => {
            tracing::debug!("No MOIM content available");
            return messages.to_vec();
        }
    };

    tracing::debug!("Injecting MOIM: {} chars", moim_content.len());

    let last_user_index = messages.iter().rposition(|msg| msg.role == Role::User);

    let mut messages_with_moim = messages.to_vec();

    match last_user_index {
        Some(index) => {
            let original_user_msg = &messages[index];

            let moim_text_content = MessageContent::Text(
                RawTextContent {
                    text: moim_content,
                    meta: None,
                }
                .no_annotation(),
            );

            let mut combined_content = vec![moim_text_content];
            combined_content.extend(original_user_msg.content.clone());

            let modified_user_msg = Message {
                id: original_user_msg.id.clone(),
                role: original_user_msg.role.clone(),
                created: original_user_msg.created,
                content: combined_content,
                metadata: original_user_msg.metadata,
            };

            messages_with_moim[index] = modified_user_msg;
        }
        None => {
            // No user message found, create a standalone MOIM message
            let moim_message = Message::user()
                .with_text(moim_content)
                .with_id(format!("moim_{}", Uuid::new_v4()))
                .agent_only();
            messages_with_moim.insert(0, moim_message);
        }
    }

    messages_with_moim
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{ToolRequest, ToolResponse};
    use rmcp::model::{CallToolRequestParam, Content};
    use rmcp::object;

    #[tokio::test]
    async fn test_moim_injection_empty_conversation() {
        let extension_manager = ExtensionManager::new();

        let messages = vec![];
        let result = inject_moim(&messages, &extension_manager, &None).await;
        assert_eq!(result.len(), 1);
        assert!(result[0].id.as_ref().unwrap().starts_with("moim_"));

        let content = result[0].content.first().and_then(|c| c.as_text()).unwrap();
        assert!(content.contains("<info-msg>"));
        assert!(content.contains("Datetime:"));
        assert!(!result[0].is_user_visible());
        assert!(result[0].is_agent_visible());
    }

    #[tokio::test]
    async fn test_moim_injection_prepends_to_last_user_message() {
        let extension_manager = ExtensionManager::new();

        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
            Message::user().with_text("How are you?"),
        ];
        let result = inject_moim(&messages, &extension_manager, &None).await;

        assert_eq!(result.len(), 3);

        let last_user_msg = &result[2];
        assert_eq!(last_user_msg.role, Role::User);

        let first_content = last_user_msg
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap();
        assert!(first_content.contains("<info-msg>"));
        assert!(first_content.contains("Datetime:"));

        let second_content = last_user_msg
            .content
            .get(1)
            .and_then(|c| c.as_text())
            .unwrap();
        assert_eq!(second_content, "How are you?");
    }

    #[tokio::test]
    async fn test_moim_injection_with_tool_responses() {
        let extension_manager = ExtensionManager::new();

        let tool_request = ToolRequest {
            id: "test_tool_1".to_string(),
            tool_call: Ok(CallToolRequestParam {
                name: "test_tool".into(),
                arguments: Some(object!({"key": "value"})),
            }),
        };

        let tool_response = ToolResponse {
            id: "test_tool_1".to_string(),
            tool_result: Ok(vec![Content::text("Tool executed successfully")]),
        };

        let messages = vec![
            Message::user().with_text("Please use a tool"),
            Message::assistant()
                .with_text("I'll use the tool now.")
                .with_content(MessageContent::ToolRequest(tool_request)),
            Message::user().with_content(MessageContent::ToolResponse(tool_response)),
        ];

        let result = inject_moim(&messages, &extension_manager, &None).await;

        assert_eq!(result.len(), 3);

        let last_user_msg = &result[2];
        assert_eq!(last_user_msg.role, Role::User);

        let first_content = last_user_msg
            .content
            .first()
            .and_then(|c| c.as_text())
            .unwrap();
        assert!(first_content.contains("<info-msg>"));

        assert!(last_user_msg
            .content
            .iter()
            .any(|c| matches!(c, MessageContent::ToolResponse(_))));
    }

    #[tokio::test]
    async fn test_moim_injection_no_user_messages() {
        let extension_manager = ExtensionManager::new();

        let messages = vec![
            Message::assistant().with_text("Hello from assistant"),
            Message::assistant().with_text("Another assistant message"),
        ];

        let result = inject_moim(&messages, &extension_manager, &None).await;

        assert_eq!(result.len(), 3);

        assert_eq!(result[0].role, Role::User);
        assert!(result[0].id.as_ref().unwrap().starts_with("moim_"));

        let content = result[0].content.first().and_then(|c| c.as_text()).unwrap();
        assert!(content.contains("<info-msg>"));

        assert_eq!(result[1].role, Role::Assistant);
        assert_eq!(result[2].role, Role::Assistant);
    }
}
