use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnEffect};
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
        if conversation.last().and_then(|m| m.error_kind()).is_none() {
            return Ok(OperationResult::NotApplicable(emit));
        }

        Ok(OperationResult::Applied(vec![TurnEffect::YieldToClient]))
    }
}
