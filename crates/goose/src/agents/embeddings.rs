use crate::model::ModelConfig;
use crate::providers::base::Provider;
use crate::providers::databricks::DatabricksProvider;
use crate::providers::embedding::{EmbeddingCapable, EmbeddingRequest, EmbeddingResponse};
use anyhow::{Context, Result};
use reqwest::Client;
use std::env;
use std::sync::Arc;

pub struct EmbeddingProvider {
    provider: Arc<dyn Provider>,
    client: Client,
    token: String,
    base_url: String,
    model: ModelConfig,
}

impl EmbeddingProvider {
    pub fn new(provider: Arc<dyn Provider>) -> Result<Self> {
        // Get configuration from the provider
        let model_config = provider.get_model_config();
        let config = crate::config::Config::global();

        // Try to use provider's embedding capability if available
        if let Some(embedding_provider) = provider
            .as_ref()
            .as_any()
            .downcast_ref::<DatabricksProvider>()
        {
            eprintln!("Using provider's native embedding capability");
            // For Databricks, we need to use a specific embedding model
            let embedding_model = env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string());
            return Ok(Self {
                provider,
                client: Client::new(),
                token: String::new(), // Not used when using provider's capability
                base_url: String::new(), // Not used when using provider's capability
                model: ModelConfig::new(embedding_model),
            });
        }

        // Check if this is a Databricks provider using the provider's metadata
        let is_databricks = provider.get_name() == "DatabricksProvider";
        eprintln!("Provider name: {}", provider.get_name());
        eprintln!("Is Databricks provider: {}", is_databricks);

        let (base_url, token) = if is_databricks {
            let mut host: Result<String, crate::config::ConfigError> =
                config.get_param("DATABRICKS_HOST");
            if host.is_err() {
                host = config.get_secret("DATABRICKS_HOST");
            }
            let host = host.context("No Databricks host found in config or secrets")?;
            eprintln!("Databricks host: {}", host);

            // Check if this is an internal user
            let is_internal =
                host.as_str() == "https://block-lakehouse-production.cloud.databricks.com";
            eprintln!("Is internal user: {}", is_internal);

            // Get auth token
            let token = if let Ok(api_key) = config.get_secret("DATABRICKS_TOKEN") {
                api_key
            } else {
                std::env::var("DATABRICKS_TOKEN").context(
                    "No API key found for embeddings. Please set DATABRICKS_TOKEN environment variable",
                )?
            };

            if is_internal {
                let model = env::var("EMBEDDING_MODEL")
                    .unwrap_or_else(|_| "text-embedding-3-small".to_string());
                // Use internal Databricks endpoint
                (
                    format!("{}/serving-endpoints/{}/invocations", host, model),
                    token,
                )
            } else {
                // For external Databricks users, use OpenAI endpoint
                let openai_key = std::env::var("OPENAI_API_KEY").context(
                    "No API key found for embeddings. Please set OPENAI_API_KEY environment variable",
                )?;

                (
                    env::var("EMBEDDING_BASE_URL")
                        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                    openai_key,
                )
            }
        } else {
            // For other providers, use OpenAI endpoint
            let token = std::env::var("OPENAI_API_KEY").context(
                "No API key found for embeddings. Please set OPENAI_API_KEY environment variable",
            )?;

            (
                env::var("EMBEDDING_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                token,
            )
        };

        let model =
            env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let log_msg = format!("Using base_url: {}, model: {}", base_url, model);
        eprintln!("{}", log_msg);

        Ok(Self {
            provider,
            client: Client::new(),
            token,
            base_url,
            model: ModelConfig::new(model),
        })
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Try to use provider's embedding capability if available
        if let Some(embedding_provider) = self
            .provider
            .as_ref()
            .as_any()
            .downcast_ref::<DatabricksProvider>()
        {
            return embedding_provider.create_embeddings(texts).await;
        }

        // Fall back to default OpenAI-compatible implementation
        let request = EmbeddingRequest {
            input: texts,
            model: self.model.model_name.clone(),
        };

        // For OpenAI, we need to append /embeddings to the base URL
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send embedding request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Embedding API error: {}", error_text);
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse embedding response")?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }

    pub async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        let embeddings = self.embed(vec![text]).await?;
        embeddings
            .into_iter()
            .next()
            .context("No embedding returned")
    }
}

// Fallback embedding provider that generates random embeddings for testing
pub struct MockEmbeddingProvider;

impl MockEmbeddingProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        Ok(texts
            .into_iter()
            .map(|_| (0..1536).map(|_| rng.gen_range(-1.0..1.0)).collect())
            .collect())
    }

    pub async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        let embeddings = self.embed(vec![text]).await?;
        embeddings
            .into_iter()
            .next()
            .context("No embedding returned")
    }
}

// Factory function to create appropriate embedding provider
pub async fn create_embedding_provider(
    provider: Arc<dyn Provider>,
) -> Box<dyn EmbeddingProviderTrait> {
    eprintln!("Attempting to create embedding provider...");

    match EmbeddingProvider::new(provider) {
        Ok(provider) => {
            eprintln!("Successfully created real embedding provider");
            Box::new(provider)
        }
        Err(e) => {
            eprintln!(
                "Failed to create embedding provider: {}. Using mock provider.",
                e
            );
            eprintln!("Initializing mock embedding provider as fallback");
            Box::new(MockEmbeddingProvider::new())
        }
    }
}

#[async_trait::async_trait]
pub trait EmbeddingProviderTrait: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
    async fn embed_single(&self, text: String) -> Result<Vec<f32>>;
}

#[async_trait::async_trait]
impl EmbeddingProviderTrait for EmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed(texts).await
    }

    async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        self.embed_single(text).await
    }
}

#[async_trait::async_trait]
impl EmbeddingProviderTrait for MockEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed(texts).await
    }

    async fn embed_single(&self, text: String) -> Result<Vec<f32>> {
        self.embed_single(text).await
    }
}

// Mock provider for testing
#[derive(Debug)]
#[allow(dead_code)]
struct MockProvider;

#[async_trait::async_trait]
impl Provider for MockProvider {
    fn metadata() -> crate::providers::base::ProviderMetadata {
        crate::providers::base::ProviderMetadata::new(
            "mock",
            "Mock Provider",
            "Mock provider for testing",
            "mock-model",
            vec![],
            "",
            vec![],
        )
    }

    fn get_model_config(&self) -> crate::model::ModelConfig {
        crate::model::ModelConfig::new("mock-model".to_string())
    }

    async fn complete(
        &self,
        _system: &str,
        _messages: &[crate::message::Message],
        _tools: &[mcp_core::tool::Tool],
    ) -> Result<
        (
            crate::message::Message,
            crate::providers::base::ProviderUsage,
        ),
        crate::providers::errors::ProviderError,
    > {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new();

        // Test single embedding
        let text = "Test text for embedding".to_string();
        let embedding = provider.embed_single(text).await.unwrap();

        // Check dimensions
        assert_eq!(embedding.len(), 1536);

        // Check values are within expected range (-1.0 to 1.0)
        for value in embedding {
            assert!(value >= -1.0 && value <= 1.0);
        }

        // Test batch embedding
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];
        let embeddings = provider.embed(texts).await.unwrap();

        // Check batch results
        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 1536);
            for value in embedding {
                assert!(value >= -1.0 && value <= 1.0);
            }
        }
    }

    #[tokio::test]
    async fn test_empty_input_mock_provider() {
        let provider = MockEmbeddingProvider::new();
        let empty_texts: Vec<String> = vec![];
        let result = provider.embed(empty_texts).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_embedding_provider_creation() {
        // Test without API key
        env::remove_var("OPENAI_API_KEY");
        assert!(EmbeddingProvider::new(Arc::new(MockProvider)).is_err());

        // Test with API key
        env::set_var("OPENAI_API_KEY", "test_key");
        let provider = EmbeddingProvider::new(Arc::new(MockProvider)).unwrap();
        assert_eq!(provider.token, "test_key");
        assert_eq!(provider.model, "mock-model");
        assert_eq!(provider.base_url, "https://api.openai.com/v1");

        // Test with custom configuration
        env::set_var("EMBEDDING_MODEL", "custom-model");
        env::set_var("EMBEDDING_BASE_URL", "https://custom.api.com");
        let provider = EmbeddingProvider::new(Arc::new(MockProvider)).unwrap();
        assert_eq!(provider.model, "custom-model");
        assert_eq!(provider.base_url, "https://custom.api.com");

        // Cleanup
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("EMBEDDING_MODEL");
        env::remove_var("EMBEDDING_BASE_URL");
    }

    #[tokio::test]
    async fn test_create_embedding_provider_fallback() {
        // Remove API key to force fallback to mock provider
        env::remove_var("OPENAI_API_KEY");

        let provider = create_embedding_provider(Arc::new(MockProvider)).await;

        // Test that we get a working provider (mock in this case)
        let text = "Test text".to_string();
        let embedding = provider.embed_single(text).await.unwrap();
        assert_eq!(embedding.len(), 1536);
    }

    #[tokio::test]
    async fn test_mock_embedding_consistency() {
        let provider = MockEmbeddingProvider::new();

        // Test that different texts get different embeddings
        let text1 = "First text".to_string();
        let text2 = "Second text".to_string();

        let embedding1 = provider.embed_single(text1.clone()).await.unwrap();
        let embedding2 = provider.embed_single(text2.clone()).await.unwrap();

        // Verify embeddings are different (random values should make this extremely likely)
        assert!(embedding1 != embedding2);

        // Verify same text gets different embeddings (mock doesn't cache)
        let embedding1_repeat = provider.embed_single(text1).await.unwrap();
        assert!(embedding1 != embedding1_repeat);
    }
}
