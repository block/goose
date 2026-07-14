use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::AgentEvent;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::session::Session;

/// True when the conversation tail is a completed assistant turn — no error,
/// nothing pending — the state in which the loop would end. End-of-turn ops
/// (stop hook, retry/goal/final-output) key their applicability on this.
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

/// One step in the agent loop. Each op gets a chance to interpret the
/// conversation state. It either returns the emitter untouched, or streams
/// events and returns effects for the machine to apply.
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

/// One action the machine applies after an operation finishes.
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

/// An op's handle to the machine: emit events the client should see, and
/// observe cancellation. Long-running ops `select!` on [`Emitter::cancelled`];
/// short ops can ignore it entirely.
pub struct Emitter {
    tx: mpsc::Sender<AgentEvent>,
    cancel: CancellationToken,
}

impl Emitter {
    pub fn new(tx: mpsc::Sender<AgentEvent>, cancel: CancellationToken) -> Self {
        Self { tx, cancel }
    }

    /// Drops silently if the receiver is gone (caller cancelled the stream).
    pub async fn emit(&self, event: AgentEvent) {
        let _ = self.tx.send(event).await;
    }

    /// The machine's cancellation token. Ops use it as they need — poll it,
    /// `select!` on it, or hand it to work that observes cancellation itself
    /// (e.g. tool dispatch).
    pub fn cancel_token(&self) -> &CancellationToken {
        &self.cancel
    }

    /// Resolves when cancellation is requested. Convenience for `select!` arms.
    pub async fn cancelled(&self) {
        self.cancel.cancelled().await
    }
}
