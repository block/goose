//! Provider-level compaction: retry a completion that overflowed the context
//! window against a compacted history instead of surfacing the error.

use std::sync::Arc;

use async_trait::async_trait;
use goose_providers::base::{MessageStream, Provider};
use goose_providers::conversation::message::Message;
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;

use crate::model::ProviderModel;
use crate::templates::Templates;

/// Wraps a provider so a `ContextLengthExceeded` response triggers compaction
/// and one retry with the summary standing in for the prior history.
pub struct CompactingProvider {
    inner: Arc<dyn Provider>,
    templates: Templates,
}

impl CompactingProvider {
    pub fn new(inner: Arc<dyn Provider>) -> Self {
        Self {
            inner,
            templates: Templates::default(),
        }
    }

    pub fn with_templates(mut self, templates: Templates) -> Self {
        self.templates = templates;
        self
    }

    async fn compacted_messages(
        &self,
        model_config: &ModelConfig,
        messages: &[Message],
    ) -> Result<Vec<Message>, ProviderError> {
        let model = ProviderModel::new(self.inner.clone(), model_config.clone());
        let summary = crate::summarize(&model, None, &self.templates, messages)
            .await
            .map_err(|error| ProviderError::ExecutionError(error.to_string()))?;
        Ok(vec![summary.message])
    }
}

#[async_trait]
impl Provider for CompactingProvider {
    fn get_name(&self) -> &str {
        self.inner.get_name()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
    ) -> Result<MessageStream, ProviderError> {
        match self
            .inner
            .stream(model_config, system, messages, tools)
            .await
        {
            Err(ProviderError::ContextLengthExceeded(_)) => {
                let compacted = self.compacted_messages(model_config, messages).await?;
                self.inner
                    .stream(model_config, system, &compacted, tools)
                    .await
            }
            other => other,
        }
    }

    async fn complete(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[rmcp::model::Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        match self
            .inner
            .complete(model_config, system, messages, tools)
            .await
        {
            Err(ProviderError::ContextLengthExceeded(_)) => {
                let compacted = self.compacted_messages(model_config, messages).await?;
                self.inner
                    .complete(model_config, system, &compacted, tools)
                    .await
            }
            other => other,
        }
    }

    async fn get_context_limit(&self, model_config: &ModelConfig) -> Result<usize, ProviderError> {
        self.inner.get_context_limit(model_config).await
    }

    fn manages_own_context(&self) -> bool {
        true
    }
}
