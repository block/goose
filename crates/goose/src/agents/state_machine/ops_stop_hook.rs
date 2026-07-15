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
    // Set when the blocking hook decided the exit (Allow or cap override).
    // The machine fires the non-blocking Stop hook at stream end when it
    // didn't — max-turns, approval waits, errors, cancellation — mirroring
    // the old loop's stop_hook_handled_for_exit.
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
