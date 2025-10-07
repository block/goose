use crate::agents::extension_manager::ExtensionManager;
use crate::agents::SessionConfig;
use crate::conversation::message::Message;
use uuid::Uuid;

/// Inject MOIM (Minus One Info Message) into conversation.
///
/// MOIM provides ephemeral context that's included in LLM calls
/// as an agent-only message (visible to agent, not user).
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

    let moim_message = Message::user()
        .with_text(moim_content)
        .with_id(format!("moim_{}", Uuid::new_v4()))
        .agent_only();

    let mut messages_with_moim = messages.to_vec();
    messages_with_moim.push(moim_message);

    messages_with_moim
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_moim_appends_to_empty_conversation() {
        let messages = vec![];
        let extension_manager = ExtensionManager::new();

        let result = inject_moim(&messages, &extension_manager, &None).await;

        assert_eq!(result.len(), 1);
        assert!(result[0].id.as_ref().unwrap().starts_with("moim_"));

        let content = result[0].content.first().and_then(|c| c.as_text()).unwrap();
        assert!(content.contains("Current date and time:"));

        assert!(!result[0].is_user_visible());
        assert!(result[0].is_agent_visible());
    }

    #[tokio::test]
    async fn test_moim_appends_to_end() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
        ];
        let extension_manager = ExtensionManager::new();

        let result = inject_moim(&messages, &extension_manager, &None).await;

        assert_eq!(result.len(), 3);

        let moim_msg = &result[2];
        assert!(moim_msg.id.as_ref().unwrap().starts_with("moim_"));

        let content = moim_msg.content.first().and_then(|c| c.as_text()).unwrap();
        assert!(content.contains("Current date and time:"));

        assert!(!moim_msg.is_user_visible());
        assert!(moim_msg.is_agent_visible());

        assert_eq!(result[0].as_concat_text(), "Hello");
        assert_eq!(result[1].as_concat_text(), "Hi there");
    }
}
