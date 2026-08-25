//! Goose integration for the reusable inference operation.

use std::sync::Arc;

use async_stream::try_stream;
use async_trait::async_trait;
use futures::StreamExt;
use goose_agent::inference::InferenceEffect;
pub use goose_agent::inference::InferenceRunner;
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
}

fn enrich_unclaimed_tool_errors(messages: &[Message], tools: &[rmcp::model::Tool]) -> Vec<Message> {
    let mut available_tools = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();
    available_tools.sort_unstable();
    available_tools.dedup();
    let available_tools = available_tools.join(", ");
    let mut messages = messages.to_vec();
    for message in &mut messages {
        for content in &mut message.content {
            let goose_providers::conversation::message::MessageContent::ToolResponse(response) =
                content
            else {
                continue;
            };
            let Some(metadata) = &mut response.metadata else {
                continue;
            };
            if metadata
                .remove(super::ops_unknown_tool::UNCLAIMED_TOOL_ERROR)
                .is_none()
            {
                continue;
            }
            let Ok(result) = &mut response.tool_result else {
                continue;
            };
            result.content.push(rmcp::model::ContentBlock::text(format!(
                "Available tools: [{available_tools}]."
            )));
        }
    }
    messages
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
        let messages = enrich_unclaimed_tool_errors(messages, tools);
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
            &messages,
            &tools,
            &toolshim_tools,
        )
        .await?;
        let model_name = model_config.model_name.clone();

        Ok(Box::pin(try_stream! {
            let mut responses = Conversation::empty();
            let mut has_usage = false;
            while let Some(item) = stream.next().await {
                let (message, reported_usage) = item?;
                if let Some(message) = &message {
                    responses.push(message.clone());
                }
                let reports_tokens = reported_usage.as_ref().is_some_and(|usage| {
                    usage.usage.input_tokens.is_some()
                        || usage.usage.output_tokens.is_some()
                        || usage.usage.total_tokens.is_some()
                });
                has_usage |= reports_tokens;
                let usage = if reports_tokens || has_usage {
                    reported_usage
                } else if let Some(response) = responses.last() {
                    let mut usage = ProviderUsage::new(model_name.clone(), Usage::default());
                    crate::providers::usage_estimator::ensure_usage_tokens(
                        &mut usage,
                        &system_prompt,
                        &messages,
                        response,
                        &tools,
                    )
                    .await
                    .map_err(|error| ProviderError::UsageError(error.to_string()))?;
                    Some(usage)
                } else {
                    reported_usage
                };
                yield (message, usage);
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use goose_providers::conversation::message::Message;

    struct NoUsageProvider;

    #[async_trait]
    impl Provider for NoUsageProvider {
        fn get_name(&self) -> &str {
            "no-usage"
        }

        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[rmcp::model::Tool],
        ) -> Result<MessageStream, ProviderError> {
            Ok(Box::pin(stream::iter([
                Ok((Some(Message::assistant().with_text("partial")), None)),
                Ok((Some(Message::assistant().with_text(" response")), None)),
            ])))
        }
    }

    #[tokio::test]
    async fn fallback_usage_is_available_before_stream_completion() {
        let provider = GooseInferenceProvider::new(Arc::new(NoUsageProvider));
        let model_config = ModelConfig::new("test-model");
        let messages = [Message::user().with_text("hello")];
        let mut stream = provider
            .stream(&model_config, "system", &messages, &[])
            .await
            .expect("stream");

        let (_, usage) = stream.next().await.expect("first item").expect("chunk");

        let usage = usage.expect("fallback usage must accompany content before EOF");
        assert!(usage.usage.input_tokens.is_some());
        assert!(usage.usage.output_tokens.is_some());
        assert!(usage.usage.total_tokens.is_some());
    }
}
