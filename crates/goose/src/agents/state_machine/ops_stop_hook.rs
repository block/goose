//! Lets stop hooks accept or block a completed assistant turn.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::agents::state_machine::operation::{
    applied, ends_turn, messages_since_kickoff, not_applicable, Emitter, Operation,
    OperationResult, TurnEffect,
};
use crate::agents::AgentEvent;
use crate::conversation::message::{Message, SystemNotificationType};
use crate::conversation::Conversation;
use crate::hooks::{HookContext, HookDecision, HookEvent, HookManager};
use crate::session::Session;

pub(super) const DEFAULT_STOP_HOOK_BLOCK_CAP: u32 = 8;

fn denial_context_message(plugin: &str, reason: &str) -> Message {
    Message::user()
        .with_text(format!(
            "Stop hook `{plugin}` blocked ending this turn:\n\n\
             {reason}\n\n\
             Address this policy hook denial before trying to stop again."
        ))
        .with_visibility(false, true)
}

fn denial_notification(plugin: &str) -> Message {
    Message::assistant().with_system_notification(
        SystemNotificationType::InlineMessage,
        format!("Stop hook `{plugin}` blocked ending this turn."),
    )
}

fn block_cap_warning(plugin: &str, cap: u32) -> Message {
    Message::assistant().with_system_notification(
        SystemNotificationType::InlineMessage,
        format!(
            "Stop hook `{plugin}` blocked the turn from ending more than {cap} consecutive times \
             \u{2014} overriding and ending turn to avoid an infinite loop. Set \
             GOOSE_STOP_HOOK_BLOCK_CAP to raise this limit."
        ),
    )
}

pub struct StopHookOperation {
    hook_manager: HookManager,
    block_cap: u32,
    consecutive_blocks: AtomicU32,
    decided_exit: Arc<AtomicBool>,
}

impl StopHookOperation {
    pub fn new(hook_manager: HookManager, block_cap: u32, decided_exit: Arc<AtomicBool>) -> Self {
        Self {
            hook_manager,
            block_cap,
            consecutive_blocks: AtomicU32::new(0),
            decided_exit,
        }
    }
}

#[async_trait]
impl Operation for StopHookOperation {
    fn name(&self) -> &'static str {
        "stop_hook"
    }

    async fn run(
        &self,
        session: &Session,
        conversation: &Conversation,
        emit: Emitter,
    ) -> Result<OperationResult> {
        let messages = messages_since_kickoff(conversation)?;
        if !ends_turn(messages) {
            return not_applicable(emit);
        }
        let last_assistant_text = conversation
            .last()
            .map(Message::as_concat_text)
            .unwrap_or_default();

        let context = HookContext::new(HookEvent::Stop, &session.id)
            .with_last_assistant_message(last_assistant_text);
        match self
            .hook_manager
            .emit_blocking(HookEvent::Stop, context)
            .await
        {
            HookDecision::Allow => {
                self.decided_exit.store(true, Ordering::Relaxed);
                not_applicable(emit)
            }
            HookDecision::Deny { reason, plugin } => {
                let blocks = self.consecutive_blocks.fetch_add(1, Ordering::Relaxed) + 1;
                if blocks > self.block_cap {
                    self.decided_exit.store(true, Ordering::Relaxed);
                    let warning = block_cap_warning(&plugin, self.block_cap);
                    emit.emit(AgentEvent::Message(warning.clone())).await;
                    applied([warning.into(), TurnEffect::YieldToClient])
                } else {
                    emit.emit(AgentEvent::Message(denial_notification(&plugin)))
                        .await;
                    applied([denial_context_message(&plugin, &reason).into()])
                }
            }
        }
    }
}
