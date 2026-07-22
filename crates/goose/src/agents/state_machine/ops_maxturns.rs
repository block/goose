//! Ends the turn when the agent has used its autonomous turn budget.

use crate::agents::state_machine::operation::{
    applied, assistant_turn_count, messages_since_kickoff, not_applicable, Emitter, Operation,
    OperationResult, TurnEffect,
};
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::Session;
use anyhow::Result;
use async_trait::async_trait;

pub(super) const DEFAULT_MAX_TURNS: u32 = 1000;

pub struct MaxTurnsOperation {
    max_turns: u32,
}

fn turn_budget_part(turns_taken: u32, max_turns: u32) -> Option<String> {
    if max_turns == 0 || turns_taken.saturating_mul(2) < max_turns {
        return None;
    }

    Some(format!(
        "<turn-budget>{turns_taken}/{max_turns} used</turn-budget>"
    ))
}

impl MaxTurnsOperation {
    pub fn new(max_turns: u32) -> Self {
        Self { max_turns }
    }
}

#[async_trait]
impl Operation for MaxTurnsOperation {
    fn name(&self) -> &'static str {
        "max_turns"
    }

    async fn moim_parts(
        &self,
        _session: &Session,
        conversation: &Conversation,
    ) -> Result<Vec<String>> {
        let turns_taken = assistant_turn_count(messages_since_kickoff(conversation)?);
        Ok(turn_budget_part(turns_taken, self.max_turns)
            .into_iter()
            .collect())
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        if assistant_turn_count(messages) < self.max_turns {
            return not_applicable(emit);
        }

        let message = Message::assistant().with_text(
            "I've reached the maximum number of actions I can do without user input. \
             Would you like me to continue?",
        );
        emit.emit(AgentEvent::Message(message.clone())).await;
        applied([message.into(), TurnEffect::YieldToClient])
    }
}
