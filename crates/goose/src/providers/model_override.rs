use std::sync::Arc;

use async_trait::async_trait;

use super::base::{
    LeadWorkerProviderTrait, MessageStream, Provider, ProviderMetadata, ProviderUsage,
};
use super::errors::ProviderError;
use super::retry::RetryConfig;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

/// A provider wrapper that overrides the model configuration
pub struct ModelOverrideProvider {
    inner: Arc<dyn Provider>,
    model_config: ModelConfig,
}

impl ModelOverrideProvider {
    pub fn new(inner: Arc<dyn Provider>, model_config: ModelConfig) -> Self {
        Self {
            inner,
            model_config,
        }
    }
}

#[async_trait]
impl Provider for ModelOverrideProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        // This won't be called for instances
        unimplemented!("ModelOverrideProvider doesn't have static metadata")
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model_config.clone()
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Use the provided config (for complete_fast, etc.)
        self.inner
            .complete_with_model(model_config, system, messages, tools)
            .await
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Use our override config
        self.inner
            .complete_with_model(&self.model_config, system, messages, tools)
            .await
    }

    // Streaming: Disable for model override cases
    fn supports_streaming(&self) -> bool {
        false // Force non-streaming path for overrides
    }

    async fn stream(
        &self,
        _system: &str,
        _messages: &[Message],
        _tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        Err(ProviderError::NotImplemented(
            "Streaming not supported with model override".to_string(),
        ))
    }

    // Delegate all other methods to inner provider
    async fn complete_fast(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        self.inner.complete_fast(system, messages, tools).await
    }

    fn retry_config(&self) -> RetryConfig {
        self.inner.retry_config()
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        self.inner.fetch_supported_models().await
    }

    fn supports_embeddings(&self) -> bool {
        self.inner.supports_embeddings()
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        self.inner.create_embeddings(texts).await
    }

    fn as_lead_worker(&self) -> Option<&dyn LeadWorkerProviderTrait> {
        None // Override providers are never lead-worker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use async_trait::async_trait;
    use rmcp::model::Tool;

    // Simple mock provider for testing
    struct MockProvider {
        model_config: ModelConfig,
    }

    impl MockProvider {
        fn new(model_name: String) -> Self {
            Self {
                model_config: ModelConfig::new_or_fail(&model_name),
            }
        }
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata
        where
            Self: Sized,
        {
            ProviderMetadata::new(
                "mock",
                "Mock Provider",
                "Test provider",
                "mock-model",
                vec![],
                "https://example.com",
                vec![],
            )
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete_with_model(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock-model".to_string(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn test_model_override_provider() {
        // Create a test provider with a default model
        let inner_provider = Arc::new(MockProvider::new("gpt-4".to_string()));
        let original_config = inner_provider.get_model_config();
        assert_eq!(original_config.model_name, "gpt-4");

        // Create override with different model
        let mut override_config = original_config.clone();
        override_config.model_name = "gpt-3.5-turbo".to_string();
        override_config = override_config.with_temperature(Some(0.7));

        let override_provider = ModelOverrideProvider::new(inner_provider.clone(), override_config);

        // Verify the override config is returned
        let config = override_provider.get_model_config();
        assert_eq!(config.model_name, "gpt-3.5-turbo");
        assert_eq!(config.temperature, Some(0.7));

        // Verify streaming is disabled
        assert!(!override_provider.supports_streaming());
    }

    #[tokio::test]
    async fn test_complete_uses_override() {
        let inner_provider = Arc::new(MockProvider::new("gpt-4".to_string()));

        let mut override_config = inner_provider.get_model_config();
        override_config.model_name = "claude-3".to_string();

        let override_provider = ModelOverrideProvider::new(inner_provider, override_config.clone());

        // When calling complete(), it should use the override config
        let result = override_provider.complete("test", &[], &[]).await;

        assert!(result.is_ok());
        // The MockProvider doesn't actually use the model, but we've verified the flow
    }
}
