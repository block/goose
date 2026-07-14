use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Role;

use crate::agents::agent::{
    stop_hook_block_cap_warning, stop_hook_denial_context_message, stop_hook_denial_notification,
};
use crate::agents::state_machine::operation::{Emitter, Operation, OperationResult, TurnEffect};
use crate::agents::{Agent, AgentEvent};
use crate::conversation::message::MessageContent;
use crate::conversation::Conversation;
use crate::hooks::HookDecision;
use crate::session::Session;

/// Gives user-defined `Stop` hooks the last word on ending a turn. Applies
/// when the tail is a completed assistant turn — nothing pending, no error —
/// which is exactly the state where every earlier op passes and the loop would
/// end. On `Deny` it appends the denial-context user message, which re-arms
/// the LLM op on the next iteration; no special control flow needed. On
/// `Allow` (or with no hooks configured) it stays out of the way and the loop
/// ends naturally.
pub struct StopHookOperation<'a> {
    agent: &'a Agent,
    // Denials since this reply started. Like the compaction retry budget,
    // this can't be derived from the conversation: the denial-context
    // messages are user-role and agent-visible, so they look like fresh
    // prompts to any walk-back.
    consecutive_blocks: AtomicU32,
}

impl<'a> StopHookOperation<'a> {
    pub fn new(agent: &'a Agent) -> Self {
        Self {
            agent,
            consecutive_blocks: AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl Operation for StopHookOperation<'_> {
    fn name(&self) -> &'static str {
        "stop_hook"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let Some(last) = conversation.last() else {
            return Ok(OperationResult::NotApplicable(emit));
        };
        let turn_ended = last.role == Role::Assistant
            && last.error_kind().is_none()
            && !last.content.iter().any(|content| {
                matches!(
                    content,
                    MessageContent::ToolRequest(_)
                        | MessageContent::FrontendToolRequest(_)
                        | MessageContent::ActionRequired(_)
                )
            });
        if !turn_ended {
            return Ok(OperationResult::NotApplicable(emit));
        }

        match self
            .agent
            .emit_stop_hook_blocking(&session.id, &last.as_concat_text())
            .await
        {
            HookDecision::Allow => Ok(OperationResult::NotApplicable(emit)),
            HookDecision::Deny { reason, plugin } => {
                let blocks = self.consecutive_blocks.fetch_add(1, Ordering::Relaxed) + 1;
                let cap = self.agent.stop_hook_block_cap();
                if blocks > cap {
                    let warning = stop_hook_block_cap_warning(&plugin, cap);
                    emit.emit(AgentEvent::Message(warning.clone())).await;
                    Ok(OperationResult::Applied(vec![
                        warning.into(),
                        TurnEffect::YieldToClient,
                    ]))
                } else {
                    emit.emit(AgentEvent::Message(stop_hook_denial_notification(&plugin)))
                        .await;
                    Ok(OperationResult::Applied(vec![
                        stop_hook_denial_context_message(&plugin, &reason).into(),
                    ]))
                }
            }
        }
    }
}
