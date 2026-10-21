//! Lets stop hooks accept or block a completed assistant turn.

use async_trait::async_trait;
use goose_agent::stop_hook::{StopDecision, StopHook};
use rmcp::model::Role;

use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::hooks::{HookContext, HookDecision, HookEvent, HookManager};
use crate::session::Session;

fn denial_reason(plugin: &str, reason: &str) -> String {
    format!(
        "Stop hook `{plugin}` blocked ending this turn:\n\n\
         {reason}\n\n\
         Address this policy hook denial before trying to stop again."
    )
}

fn denial_notice(plugin: &str) -> String {
    format!("Stop hook `{plugin}` blocked ending this turn.")
}

pub fn deny_limit_warning(limit: u32) -> String {
    format!(
        "Stop hooks blocked the turn from ending more than {limit} consecutive times \
         \u{2014} overriding and ending turn to avoid an infinite loop. Set \
         GOOSE_STOP_HOOK_BLOCK_CAP to raise this limit."
    )
}

pub struct GooseStopHook {
    hook_manager: HookManager,
}

impl GooseStopHook {
    pub fn new(hook_manager: HookManager) -> Self {
        Self { hook_manager }
    }

    fn trailing_assistant_text(messages: &[Message]) -> String {
        messages
            .iter()
            .rev()
            .take_while(|message| message.role == Role::Assistant)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .filter(|message| message.is_user_visible())
            .map(|message| message.user_visible_content().as_concat_text())
            .collect()
    }
}

#[async_trait]
impl StopHook<Session> for GooseStopHook {
    async fn on_stop(&self, session: &Session, conversation: &Conversation) -> StopDecision {
        let context = HookContext::new(HookEvent::Stop, &session.id)
            .with_last_assistant_message(Self::trailing_assistant_text(conversation.messages()))
            .with_working_dir(session.working_dir.to_string_lossy().into_owned());
        match self
            .hook_manager
            .emit_blocking(HookEvent::Stop, context)
            .await
        {
            HookDecision::Allow => StopDecision::Allow,
            HookDecision::Deny { reason, plugin } => StopDecision::Deny {
                reason: denial_reason(&plugin, &reason),
                notice: Some(denial_notice(&plugin)),
            },
        }
    }
}
