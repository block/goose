use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{ends_turn, Emitter, Operation, OperationResult};
use crate::agents::{Agent, AgentEvent};
use crate::conversation::Conversation;
use crate::session::Session;

pub struct SteerOperation<'a> {
    agent: &'a Agent,
}

impl<'a> SteerOperation<'a> {
    pub fn new(agent: &'a Agent) -> Self {
        Self { agent }
    }
}

#[async_trait]
impl Operation for SteerOperation<'_> {
    fn name(&self) -> &'static str {
        "steer"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let between_turns =
            ends_turn(conversation) || conversation.last().is_some_and(|m| m.is_tool_response());
        if !between_turns || !self.agent.has_pending_steers(&session.id).await {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let mut effects = Vec::new();
        for message in self.agent.drain_pending_steers(&session.id).await {
            self.agent
                .emit_user_prompt_submit_hook(&session.id, &message.as_concat_text())
                .await;
            emit.emit(AgentEvent::Message(message.clone())).await;
            effects.push(message.into());
        }
        if effects.is_empty() {
            return Ok(OperationResult::NotApplicable(emit));
        }
        Ok(OperationResult::Applied(effects))
    }
}
