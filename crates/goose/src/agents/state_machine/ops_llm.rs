//! Goose integration for the reusable inference operation.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
pub use goose_agent::inference::InferenceRunner;
use goose_agent::inference::InferenceRuntime;
use goose_agent::operation::ConversationEffect;
use goose_providers::base::{MessageStream, Provider};
use goose_providers::conversation::message::Message;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::conversation::Conversation;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

use crate::agents::state_machine::ops_unknown_tool::UNCLAIMED_TOOL_ERROR;
use crate::agents::state_machine::GooseEffect;
use crate::session::Session;

pub(super) use goose_agent::inference::{chat_span, record_chat_usage};

pub struct GooseInferenceRuntime;

#[async_trait]
impl InferenceRuntime<Session, GooseEffect> for GooseInferenceRuntime {
    fn session_id<'a>(&self, session: &'a Session) -> &'a str {
        &session.id
    }

    fn status(&self, session: &Session) -> (String, i64, i64) {
        (
            session.goose_mode.to_string(),
            session.usage.total_tokens.unwrap_or(0) as i64,
            session.accumulated_usage.total_tokens.unwrap_or(0) as i64,
        )
    }

    fn effect_from_message(&self, message: Message) -> GooseEffect {
        message.into()
    }
    fn effect_from_conversation(&self, conversation: Conversation) -> GooseEffect {
        conversation.into()
    }
    fn effect_from_conversation_effect(&self, effect: ConversationEffect) -> GooseEffect {
        effect.into()
    }
    fn usage_effect(&self, usage: ProviderUsage) -> GooseEffect {
        GooseEffect::RecordUsage(usage)
    }

    fn appended_message<'a>(&self, effect: &'a GooseEffect) -> Option<&'a Message> {
        match effect {
            GooseEffect::Conversation(ConversationEffect::AppendMessage(message)) => Some(message),
            _ => None,
        }
    }

    fn unclaimed_tool_error_key(&self) -> &'static str {
        UNCLAIMED_TOOL_ERROR
    }

    fn latest_provider_session_id<'a>(
        &self,
        conversation: &'a Conversation,
        provider: &str,
    ) -> Option<&'a str> {
        super::super::latest_provider_session_id(conversation.messages(), provider)
    }

    async fn prepare_tools(
        &self,
        tools: Vec<rmcp::model::Tool>,
        system_prompt: String,
        model: &ModelConfig,
    ) -> (Vec<rmcp::model::Tool>, Vec<rmcp::model::Tool>, String) {
        crate::agents::reply_parts::prepare_tools_for_provider(tools, system_prompt, model)
    }

    #[allow(clippy::too_many_arguments)]
    async fn stream(
        &self,
        provider: Arc<dyn Provider>,
        model: ModelConfig,
        session_id: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
        toolshim_tools: &[rmcp::model::Tool],
    ) -> Result<MessageStream, ProviderError> {
        crate::agents::reply_parts::stream_response_from_provider(
            provider,
            model,
            session_id,
            system_prompt,
            messages,
            tools,
            toolshim_tools,
        )
        .await
    }

    async fn turn_context(
        &self,
        session: &Session,
        context_limit: usize,
        moim_parts: Vec<String>,
        conversation: &Conversation,
    ) -> Option<Message> {
        let turn = goose_agent::operation::messages_since_kickoff(conversation).ok()?;
        let turn_start = turn
            .first()
            .and_then(|message| chrono::DateTime::from_timestamp(message.created, 0))
            .map(|timestamp| timestamp.with_timezone(&chrono::Local))
            .unwrap_or_else(chrono::Local::now);
        let last = turn
            .iter()
            .rev()
            .find(|message| message.is_turn_context())
            .map(Message::as_concat_text);
        crate::agents::moim::turn_context_event(
            &session.working_dir,
            Some(context_limit),
            moim_parts,
            turn_start,
        )
        .filter(|event| Some(event.as_concat_text()) != last)
    }

    async fn ensure_usage(
        &self,
        usage: &mut ProviderUsage,
        system_prompt: &str,
        messages: &[Message],
        response: &Message,
        tools: &[rmcp::model::Tool],
    ) -> Result<()> {
        crate::providers::usage_estimator::ensure_usage_tokens(
            usage,
            system_prompt,
            messages,
            response,
            tools,
        )
        .await
    }

    fn report_error(&self, _error: &ProviderError) {
        #[cfg(feature = "telemetry")]
        crate::posthog::emit_error(_error.telemetry_type(), &_error.to_string());
    }
}
