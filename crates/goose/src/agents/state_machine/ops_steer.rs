//! Adds queued user guidance when the agent is between model and tool turns.

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{
    applied, ends_turn, last_effective_role, messages_since_kickoff, not_applicable, Emitter,
    Operation, OperationResult,
};
use crate::agents::steering::PendingSteers;
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::{Conversation, EffectiveRole};
use crate::hooks::{HookContext, HookEvent, HookManager};
use crate::session::Session;

pub struct SteerOperation<'a> {
    pending_steers: &'a PendingSteers,
    hook_manager: HookManager,
}

impl<'a> SteerOperation<'a> {
    pub(super) fn new(pending_steers: &'a PendingSteers, hook_manager: HookManager) -> Self {
        Self {
            pending_steers,
            hook_manager,
        }
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
        let messages = messages_since_kickoff(conversation)?;
        let between_turns =
            ends_turn(messages) || last_effective_role(messages)? == EffectiveRole::Tool;
        if !between_turns {
            return not_applicable(emit);
        }

        let mut effects = Vec::new();
        for message in self.pending_steers.drain(&session.id).await {
            let context = HookContext::new(HookEvent::UserPromptSubmit, &session.id)
                .with_message(message.as_concat_text());
            self.hook_manager
                .emit(HookEvent::UserPromptSubmit, context)
                .await;
            emit.emit(AgentEvent::Message(message.clone())).await;
            effects.push(message.into());
        }
        if effects.is_empty() {
            return not_applicable(emit);
        }
        applied(effects)
    }
}
