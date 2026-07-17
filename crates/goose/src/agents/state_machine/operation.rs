use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::AgentEvent;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::session::Session;

pub fn ends_turn(conversation: &Conversation) -> bool {
    conversation.last().is_some_and(|last| {
        last.role == rmcp::model::Role::Assistant
            && last.error_kind().is_none()
            && !last.content.iter().any(|content| {
                matches!(
                    content,
                    MessageContent::ToolRequest(_)
                        | MessageContent::FrontendToolRequest(_)
                        | MessageContent::ActionRequired(_)
                )
            })
    })
}

#[async_trait]
pub trait Operation: Send + Sync {
    fn name(&self) -> &'static str;

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult>;
}

pub type TurnOutcome = Vec<TurnEffect>;

pub enum OperationResult {
    NotApplicable(Emitter),
    Applied(TurnOutcome),
}

pub enum TurnEffect {
    AppendMessage(Message),
    ReplaceConversation(Conversation),
    PatchToolRequestMeta {
        message_id: String,
        tool_call_id: String,
        patch: serde_json::Value,
    },
    SetMessageVisibility {
        message_id: String,
        user_visible: bool,
        agent_visible: bool,
    },
    EmitCurrentHistoryReplaced,
    YieldToClient,
}

impl From<Message> for TurnEffect {
    fn from(message: Message) -> Self {
        TurnEffect::AppendMessage(message)
    }
}

impl From<Conversation> for TurnEffect {
    fn from(conversation: Conversation) -> Self {
        TurnEffect::ReplaceConversation(conversation)
    }
}

pub struct Emitter {
    tx: mpsc::Sender<AgentEvent>,
    cancel: CancellationToken,
}

impl Emitter {
    pub fn new(tx: mpsc::Sender<AgentEvent>, cancel: CancellationToken) -> Self {
        Self { tx, cancel }
    }

    pub async fn emit(&self, event: AgentEvent) {
        let _ = self.tx.send(event).await;
    }

    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    pub async fn cancelled(&self) {
        self.cancel.cancelled().await
    }
}
