use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::agent::{
    stop_hook_block_cap_warning, stop_hook_denial_context_message, stop_hook_denial_notification,
};
use crate::agents::state_machine::operation::{
    ends_turn, Emitter, Operation, OperationResult, TurnEffect,
};
use crate::agents::{Agent, AgentEvent};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::hooks::HookDecision;
use crate::session::Session;

pub struct StopHookOperation<'a> {
    agent: &'a Agent,
    consecutive_blocks: AtomicU32,
    decided_exit: Arc<AtomicBool>,
}

impl<'a> StopHookOperation<'a> {
    pub fn new(agent: &'a Agent, decided_exit: Arc<AtomicBool>) -> Self {
        Self {
            agent,
            consecutive_blocks: AtomicU32::new(0),
            decided_exit,
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
        if !ends_turn(conversation) {
            return Ok(OperationResult::NotApplicable(emit));
        }
        let last_assistant_text = conversation
            .last()
            .map(Message::as_concat_text)
            .unwrap_or_default();

        match self
            .agent
            .emit_stop_hook_blocking(&session.id, &last_assistant_text)
            .await
        {
            HookDecision::Allow => {
                self.decided_exit.store(true, Ordering::Relaxed);
                Ok(OperationResult::NotApplicable(emit))
            }
            HookDecision::Deny { reason, plugin } => {
                let blocks = self.consecutive_blocks.fetch_add(1, Ordering::Relaxed) + 1;
                let cap = self.agent.stop_hook_block_cap();
                if blocks > cap {
                    self.decided_exit.store(true, Ordering::Relaxed);
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
