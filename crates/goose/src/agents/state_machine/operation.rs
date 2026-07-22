use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agents::AgentEvent;
use crate::conversation::message::{Message, MessageContent, MessageErrorKind};
use crate::conversation::{effective_role, Conversation, EffectiveRole};
use crate::providers::base::ProviderUsage;
use crate::recipe::Recipe;
use crate::session::Session;
use rmcp::model::Tool;

pub type OperationFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct SlashCommand<'a> {
    pub command: &'a str,
    pub params_str: &'a str,
}

pub fn messages_since_kickoff(conversation: &Conversation) -> Result<&[Message]> {
    let messages = conversation.messages();
    let start = messages
        .iter()
        .rposition(|message| {
            message.role == rmcp::model::Role::User
                && message.is_user_visible()
                && !message.is_tool_response()
        })
        .ok_or_else(|| anyhow!("state machine conversation has no kickoff message"))?;
    Ok(&messages[start..])
}

pub fn trailing_error(conversation: &Conversation) -> Option<MessageErrorKind> {
    conversation.last().and_then(Message::error_kind)
}

pub fn last_effective_role(messages: &[Message]) -> Result<EffectiveRole> {
    messages
        .last()
        .map(effective_role)
        .ok_or_else(|| anyhow!("cannot determine the role of an empty conversation"))
}

pub fn assistant_turn_count(messages: &[Message]) -> u32 {
    let mut turns = 0;
    let mut in_assistant_block = false;
    for message in messages.iter().rev() {
        if message.role == rmcp::model::Role::Assistant {
            if !in_assistant_block {
                turns += 1;
                in_assistant_block = true;
            }
        } else {
            in_assistant_block = false;
        }
    }
    turns
}

pub fn ends_turn(messages: &[Message]) -> bool {
    messages.last().is_some_and(|last| {
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

    async fn run_command(
        &self,
        _command: &SlashCommand<'_>,
        _session: &Session,
        _conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        not_applicable(emit)
    }

    async fn inference_tools(&self, _session: &Session) -> Result<Vec<Tool>> {
        Ok(Vec::new())
    }

    async fn prompt_parts(
        &self,
        _session: &Session,
        _conversation: &Conversation,
    ) -> Result<Vec<(String, String)>> {
        Ok(Vec::new())
    }

    async fn moim_parts(
        &self,
        _session: &Session,
        _conversation: &Conversation,
    ) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn run(
        &self,
        _session: &Session,
        _conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        not_applicable(emit)
    }
}

pub struct InferenceInput {
    pub system_prompt: String,
    pub tools: Vec<Tool>,
    pub toolshim_tools: Vec<Tool>,
    pub moim_parts: Vec<String>,
}

#[async_trait]
pub trait Inference: Operation {
    async fn infer(
        &self,
        session: &Session,
        conversation: &Conversation,
        input: InferenceInput,
        emit: Emitter,
    ) -> Result<OperationResult>;
}

pub type TurnOutcome = Vec<TurnEffect>;

pub enum OperationResult {
    NotApplicable(Emitter),
    Applied(TurnOutcome),
}

pub fn not_applicable(emit: Emitter) -> Result<OperationResult> {
    Ok(OperationResult::NotApplicable(emit))
}

pub fn applied(effects: impl IntoIterator<Item = TurnEffect>) -> Result<OperationResult> {
    Ok(OperationResult::Applied(effects.into_iter().collect()))
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
    SetRecipe(Option<Recipe>),
    RecordUsage {
        usage: ProviderUsage,
        is_compaction: bool,
    },
    ResetContextUsage,
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
