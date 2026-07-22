//! Runs a kickoff message beginning with `!` as a direct shell tool call.

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::CallToolRequestParams;

use crate::agents::state_machine::operation::{
    applied, last_effective_role, messages_since_kickoff, not_applicable, Emitter, Operation,
    OperationResult, TurnEffect,
};
use crate::agents::AgentEvent;
use crate::conversation::message::Message;
use crate::conversation::{Conversation, EffectiveRole};
use crate::session::Session;

const SHELL_TOOL_NAME: &str = "developer__shell";

pub struct BangShellOperation;

impl BangShellOperation {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Operation for BangShellOperation {
    fn name(&self) -> &'static str {
        "bang_shell"
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        let Some(kickoff) = messages.first() else {
            return not_applicable(emit);
        };
        let kickoff_text = kickoff.as_concat_text();
        let Some(command) = kickoff_text
            .trim_start()
            .strip_prefix('!')
            .map(str::trim_start)
            .filter(|command| !command.is_empty())
        else {
            return not_applicable(emit);
        };

        if messages.len() > 1 {
            return if last_effective_role(messages)? == EffectiveRole::Tool {
                applied([TurnEffect::YieldToClient])
            } else {
                not_applicable(emit)
            };
        }

        let call = CallToolRequestParams::new(SHELL_TOOL_NAME.to_string()).with_arguments(
            serde_json::Map::from_iter([(
                "command".to_string(),
                serde_json::Value::String(command.to_string()),
            )]),
        );
        let request = Message::assistant()
            .with_generated_id()
            .with_tool_request(format!("bang_shell_{}", uuid::Uuid::now_v7()), Ok(call));
        emit.emit(AgentEvent::Message(request.clone())).await;

        applied([request.into()])
    }
}
