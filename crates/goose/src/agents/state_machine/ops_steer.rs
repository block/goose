//! Adds queued user guidance when the agent is between model and tool turns.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{
    applied, ends_turn, last_effective_role, messages_since_kickoff, not_applicable, Emitter,
    Operation, OperationResult,
};
use crate::agents::state_machine::ops_llm::was_native_steer_delivered;
use crate::agents::steering::SteeringQueue;
use crate::conversation::{Conversation, EffectiveRole};
use crate::session::Session;

pub struct SteerOperation {
    queue: Arc<SteeringQueue>,
}

impl SteerOperation {
    pub(crate) fn new(queue: Arc<SteeringQueue>) -> Self {
        Self { queue }
    }
}

#[async_trait]
impl Operation for SteerOperation {
    fn name(&self) -> &'static str {
        "steer"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        let between_turns = ends_turn(messages)
            || last_effective_role(messages)? == EffectiveRole::Tool
            || messages.last().is_some_and(was_native_steer_delivered);
        if !between_turns {
            return not_applicable();
        }

        self.queue
            .wait_until_steer_can_be_used(emit.cancel_token())
            .await;
        let pending = self.queue.drain_available().await;
        if pending.is_empty() {
            return not_applicable();
        }

        let mut effects = Vec::with_capacity(pending.len());
        for message in pending {
            let message = emit.message(message).await;
            effects.push(message.into());
        }
        applied(effects)
    }
}
