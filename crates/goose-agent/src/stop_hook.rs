use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use goose_provider_types::conversation::message::Message;
use goose_provider_types::conversation::Conversation;
use goose_provider_types::maybe_send::{MaybeSend, MaybeSync};

use crate::hook::{inline_notification, HookDecision};
use crate::operation::{
    applied, ends_turn, messages_since_kickoff, not_applicable, yielded, yielded_with, Emitter,
    Operation, OperationResult,
};

pub const DENIED: &str = "denied";
pub const DEFAULT_DENY_LIMIT: u32 = 8;

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait StopHook<S>: MaybeSend + MaybeSync {
    async fn on_stop(&self, session: &S, conversation: &Conversation) -> HookDecision;
}

pub struct StopHookOperation<S> {
    hooks: Vec<Arc<dyn StopHook<S>>>,
    deny_limit: u32,
    deny_limit_message: Option<String>,
}

impl<S> StopHookOperation<S> {
    pub fn new(hooks: Vec<Arc<dyn StopHook<S>>>) -> Self {
        Self {
            hooks,
            deny_limit: DEFAULT_DENY_LIMIT,
            deny_limit_message: None,
        }
    }

    pub fn with_deny_limit_per_turn(mut self, deny_limit: u32) -> Self {
        self.deny_limit = deny_limit;
        self
    }

    pub fn with_deny_limit_message(mut self, message: impl Into<String>) -> Self {
        self.deny_limit_message = Some(message.into());
        self
    }

    async fn first_denial(&self, session: &S, conversation: &Conversation) -> HookDecision {
        for hook in &self.hooks {
            let decision = hook.on_stop(session, conversation).await;
            if matches!(decision, HookDecision::Deny { .. }) {
                return decision;
            }
        }
        HookDecision::Allow
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl<S, E> Operation<S, E> for StopHookOperation<S>
where
    S: MaybeSend + MaybeSync + 'static,
    E: From<Message> + MaybeSend + 'static,
{
    fn name(&self) -> &'static str {
        "stop_hook"
    }

    async fn run(
        &self,
        session: &S,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<E>> {
        let messages = messages_since_kickoff(conversation)?;
        if !ends_turn(messages) {
            return not_applicable();
        }
        let (reason, user_message) = match self.first_denial(session, conversation).await {
            HookDecision::Allow => return yielded(),
            HookDecision::Deny {
                reason,
                user_message,
            } => (reason, user_message),
        };
        let denials = messages
            .iter()
            .filter(|message| Operation::<S, E>::message_meta(self, message, DENIED).is_some())
            .count() as u32
            + 1;
        if denials > self.deny_limit {
            let message = self
                .deny_limit_message
                .as_deref()
                .map(|message| emit.message(inline_notification(message)).into());
            return yielded_with(message);
        }
        let mut context = Message::user()
            .with_text(reason)
            .with_visibility(false, true);
        Operation::<S, E>::set_message_meta(self, &mut context, DENIED, serde_json::json!(true));
        if let Some(user_message) = user_message {
            emit.message(inline_notification(&user_message));
        }
        applied([context.into()])
    }
}
