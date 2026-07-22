//! Ends the turn when an error remains at the end of the conversation.

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{
    applied, not_applicable, trailing_error, Emitter, Operation, OperationResult, TurnEffect,
};
use crate::conversation::Conversation;
use crate::session::Session;

pub struct ExitOnErrorOperation;

#[async_trait]
impl Operation for ExitOnErrorOperation {
    fn name(&self) -> &'static str {
        "exit_on_error"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if trailing_error(conversation).is_none() {
            return not_applicable(emit);
        }

        applied([TurnEffect::YieldToClient])
    }
}
