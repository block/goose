use crate::operation::{messages_since_kickoff, unanswered_tool_requests};
use goose_provider_types::conversation::Conversation;
use goose_provider_types::maybe_send::{MaybeSend, MaybeSync};

#[cfg(not(target_arch = "wasm32"))]
type GateCondition<'a, S> = dyn Fn(&S, &Conversation) -> bool + Send + Sync + 'a;

#[cfg(target_arch = "wasm32")]
type GateCondition<'a, S> = dyn Fn(&S, &Conversation) -> bool + 'a;

pub struct StepGate<'a, S> {
    pub(crate) name: &'static str,
    condition: Box<GateCondition<'a, S>>,
}

impl<'a, S> StepGate<'a, S> {
    pub fn new(
        name: &'static str,
        condition: impl Fn(&S, &Conversation) -> bool + MaybeSend + MaybeSync + 'a,
    ) -> Self {
        Self {
            name,
            condition: Box::new(condition),
        }
    }

    pub(crate) fn is_open(&self, session: &S, conversation: &Conversation) -> bool {
        (self.condition)(session, conversation)
    }
}

pub fn tool_calls_answered<'a, S>() -> StepGate<'a, S> {
    StepGate::new("tool calls answered", |_session, conversation| {
        messages_since_kickoff(conversation)
            .map_or(true, |turn| unanswered_tool_requests(turn).is_empty())
    })
}
