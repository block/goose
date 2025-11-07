use crate::agents::extension_manager::ExtensionManager;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use rmcp::model::Role;

/// Inject MOIM (Minus One Info Message) into conversation.
///
/// MOIM provides ephemeral context that's included in LLM calls
/// but never persisted to conversation history.
///
/// The MOIM content is inserted as a separate user message before
/// the latest assistant message to ensure clean tool response handling.
pub async fn inject_moim(
    conversation: Conversation,
    extension_manager: &ExtensionManager,
) -> Conversation {
    let config = crate::config::Config::global();
    if !config.get_param("GOOSE_MOIM_ENABLED").unwrap_or(true) {
        return conversation;
    }

    if let Some(moim) = extension_manager.collect_moim().await {
        let mut messages = conversation.messages().clone();
        let idx = messages
            .iter()
            .rposition(|m| m.role == Role::Assistant)
            .unwrap_or(messages.len());
        messages.insert(idx, Message::user().with_text(moim));
        return Conversation::new_unvalidated(messages);
    }
    conversation
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(msgs.len(), 5); // Original 4 + 1 MOIM
        assert_eq!(msgs[2].content[0].as_text().unwrap(), "Bye");
        assert!(msgs[3].content[0].as_text().unwrap().contains("<info-msg>")); // MOIM before last assistant
        assert_eq!(msgs[4].content[0].as_text().unwrap(), "Goodbye");
    }

    #[tokio::test]
    async fn test_moim_injection_no_assistant() {
        let em = ExtensionManager::new_without_provider();

        let conv = Conversation::new_unvalidated(vec![Message::user().with_text("Hello")]);
        let result = inject_moim(conv, &em).await;
        assert_eq!(result.messages().len(), 2);
        assert_eq!(result.messages()[0].content[0].as_text().unwrap(), "Hello");
        assert!(result.messages()[1].content[0]
            .as_text()
            .unwrap()
            .contains("<info-msg>"));
    }

    #[tokio::test]
    async fn test_moim_config_disable() {
        let config = crate::config::Config::global();
        config.set_param("GOOSE_MOIM_ENABLED", false).ok();

        let em = ExtensionManager::new_without_provider();
        let conv = Conversation::new_unvalidated(vec![Message::user().with_text("Hi")]);
        let result = inject_moim(conv.clone(), &em).await;

        assert_eq!(result.messages(), conv.messages());

        config.delete("GOOSE_MOIM_ENABLED").ok();
    }
}
