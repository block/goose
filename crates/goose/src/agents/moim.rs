use crate::agents::extension_manager::ExtensionManager;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
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
    conversation: Conversation,
    extension_manager: &ExtensionManager,
) -> Conversation {
    let config = crate::config::Config::global();
    if let Ok(false) = config.get_param::<bool>("GOOSE_MOIM_ENABLED") {
        tracing::debug!("MOIM injection disabled via GOOSE_MOIM_ENABLED=false");
        return conversation;
    }

    let moim_content = match extension_manager.collect_moim().await {
        Some(content) if !content.trim().is_empty() => content,
        _ => {
            tracing::debug!("No MOIM content available");
            return conversation;
        }
    };

    tracing::debug!("Injecting MOIM: {} chars", moim_content.len());

    let messages: Vec<Message> = conversation.into_iter().collect();
    let last_user_index = messages.iter().rposition(|msg| msg.role == Role::User);

    let mut messages_with_moim = messages.clone();

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

    Conversation::new_unvalidated(messages_with_moim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_moim_injection() {
        let em = ExtensionManager::new_without_provider();

        // Test 1: Prepends to last user message
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi"),
            Message::user().with_text("Bye"),
        ]);
        let result = inject_moim(conv, &em).await;
        let msgs = result.messages();
        assert_eq!(msgs.len(), 3);
        assert!(msgs[2].content[0].as_text().unwrap().contains("<info-msg>"));
        assert_eq!(msgs[2].content[1].as_text().unwrap(), "Bye");

        // Test 2: Creates user message when none exist
        let conv = Conversation::new_unvalidated(vec![Message::assistant().with_text("Hello")]);
        let result = inject_moim(conv, &em).await;
        assert_eq!(result.messages().len(), 2);
        assert_eq!(result.messages()[0].role, Role::User);
        assert!(result.messages()[0]
            .id
            .as_ref()
            .unwrap()
            .starts_with("moim_"));
    }

    #[tokio::test]
    async fn test_moim_config_disable() {
        let config = crate::config::Config::global();
        config.set_param("GOOSE_MOIM_ENABLED", false).ok();

        let em = ExtensionManager::new_without_provider();
        let conv = Conversation::new_unvalidated(vec![Message::user().with_text("Hi")]);
        let result = inject_moim(conv.clone(), &em).await;

        assert_eq!(result.messages(), conv.messages()); // Unchanged

        config.delete("GOOSE_MOIM_ENABLED").ok();
    }
}
