//! Goose integration for the reusable inference operation.

use std::sync::Arc;

use async_stream::try_stream;
use async_trait::async_trait;
use futures::StreamExt;
use goose_agent::inference::InferenceEffect;
pub use goose_agent::inference::InferenceRunner;
use goose_agent::operation::ConversationEffect;
use goose_providers::base::{MessageStream, ModelInfo, Provider};
use goose_providers::conversation::message::Message;
use goose_providers::conversation::token_usage::{ProviderUsage, Usage};
use goose_providers::conversation::Conversation;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

use crate::agents::state_machine::GooseEffect;

pub(super) use goose_agent::inference::{chat_span, record_chat_usage};

pub struct GooseInferenceProvider {
    inner: Arc<dyn Provider>,
}

impl GooseInferenceProvider {
    pub fn new(inner: Arc<dyn Provider>) -> Self {
        Self { inner }
    }
}

impl InferenceEffect for GooseEffect {
    fn record_usage(usage: ProviderUsage) -> Self {
        GooseEffect::RecordUsage(usage)
    }

    fn appended_message(&self) -> Option<&Message> {
        match self {
            GooseEffect::Conversation(ConversationEffect::AppendMessage(message)) => Some(message),
            _ => None,
        }
    }
}

#[async_trait]
impl Provider for GooseInferenceProvider {
    fn get_name(&self) -> &str {
        self.inner.get_name()
    }

    fn provider_session_id(&self) -> Option<String> {
        self.inner.provider_session_id()
    }

    async fn resume(&self, session_id: &str) -> Result<(), ProviderError> {
        self.inner.resume(session_id).await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
    ) -> Result<MessageStream, ProviderError> {
        let (tools, toolshim_tools, system_prompt) =
            crate::agents::reply_parts::prepare_tools_for_provider(
                tools.to_vec(),
                system.to_string(),
                model_config,
            );
        let session_id = crate::session_context::current_session_id().unwrap_or_default();
        let mut stream = crate::agents::reply_parts::stream_response_from_provider(
            self.inner.clone(),
            model_config.clone(),
            &session_id,
            &system_prompt,
            messages,
            &tools,
            &toolshim_tools,
        )
        .await?;
        let model_name = model_config.model_name.clone();
        let messages = messages.to_vec();

        Ok(Box::pin(try_stream! {
            let mut responses = Conversation::empty();
            let mut has_usage = false;
            while let Some(item) = stream.next().await {
                let (message, usage) = item?;
                if let Some(message) = &message {
                    responses.push(message.clone());
                }
                has_usage |= usage.as_ref().is_some_and(|usage| {
                    usage.usage.input_tokens.is_some()
                        || usage.usage.output_tokens.is_some()
                        || usage.usage.total_tokens.is_some()
                });
                yield (message, usage);
            }
            if !has_usage {
                if let Some(response) = responses.last() {
                    let mut usage = ProviderUsage::new(model_name, Usage::default());
                    crate::providers::usage_estimator::ensure_usage_tokens(
                        &mut usage,
                        &system_prompt,
                        &messages,
                        response,
                        &tools,
                    )
                    .await
                    .map_err(|error| ProviderError::UsageError(error.to_string()))?;
                    yield (None, Some(usage));
                }
            }
        }))
    }

    async fn get_context_limit(&self, model_config: &ModelConfig) -> Result<usize, ProviderError> {
        self.inner.get_context_limit(model_config).await
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        self.inner.fetch_model_info(model_name).await
    }
}
