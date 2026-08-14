use rmcp::model::ServerNotification;

use crate::conversation::message::{Message, MessageUsage};
use crate::conversation::Conversation;

#[derive(Clone, Debug)]
pub enum AgentEvent {
    Message(Message),
    Usage(crate::providers::base::ProviderUsage),
    MessageUsage {
        message_id: Option<String>,
        usage: MessageUsage,
    },
    McpNotification((String, ServerNotification)),
    HistoryReplaced(Conversation),
}
