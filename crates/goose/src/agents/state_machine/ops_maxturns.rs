use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Role;

use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnEffect};
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::Session;

/// Stops the loop once the agent has taken `max_turns` LLM turns in response to
/// a single user prompt. A "turn" is one assistant message; the current request
/// starts at the last genuine user message (a prompt, not a tool response).
pub struct MaxTurnsOperation {
    max_turns: u32,
}

impl MaxTurnsOperation {
    pub fn new(max_turns: u32) -> Self {
        Self { max_turns }
    }
}

pub(crate) fn turns_taken_this_request(conversation: &Conversation) -> u32 {
    let mut turns = 0u32;
    for message in conversation.messages().iter().rev() {
        // Only a message the user actually typed starts a new request.
        // Machine-generated user messages (goal/grind nudges, stop-hook
        // denial context — user-invisible by construction) keep the loop
        // going and must not reset the budget, or a grind could run forever.
        if message.role == Role::User && !message.is_tool_response() && message.is_user_visible() {
            break;
        }
        if message.role == Role::Assistant {
            turns += 1;
        }
    }
    turns
}

#[async_trait]
impl Operation for MaxTurnsOperation {
    fn name(&self) -> &'static str {
        "max_turns"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        if turns_taken_this_request(conversation) < self.max_turns {
            return Ok(OperationResult::NotApplicable(emit));
        }

        let message = Message::assistant().with_text(
            "I've reached the maximum number of actions I can do without user input. \
             Would you like me to continue?",
        );
        emit.emit(AgentEvent::Message(message.clone())).await;
        Ok(OperationResult::Applied(vec![
            message.into(),
            TurnEffect::YieldToClient,
        ]))
    }
}
