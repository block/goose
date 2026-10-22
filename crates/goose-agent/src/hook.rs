use goose_provider_types::conversation::message::{Message, SystemNotificationType};

#[derive(Debug)]
pub enum HookDecision {
    Allow,
    Deny {
        reason: String,
        user_message: Option<String>,
    },
}

pub(crate) fn inline_notification(text: &str) -> Message {
    Message::assistant().with_system_notification(SystemNotificationType::InlineMessage, text)
}
