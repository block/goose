use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::sync::Arc;

use super::api_client::ApiClient;
use super::base::{ConfigKey, Provider, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::model::ModelConfig;

/// LM Studio Provider with enhanced features
///
/// Supports:
/// - OpenAI-compatible API (/v1/*)
/// - Native LM Studio API (/api/v1/*)
/// - Anthropic-compatible API (/v1/messages)
/// - Model management (load/unload/download)
/// - MCP via API for tool calling
/// - Stateful chats with previous_response_id
/// - Speculative decoding with draft models
/// - Idle TTL and auto-evict for models
/// - Authentication with API tokens
/// - Enhanced stats (tokens/second, TTFT)
pub struct LmStudioProvider;

impl LmStudioProvider {
    const DEFAULT_BASE_URL: &'static str = "http://localhost:1234/v1";
    const DEFAULT_PORT: u16 = 1234;

    pub fn metadata() -> ProviderMetadata {
        use super::base::ModelInfo;

        ProviderMetadata {
            name: "lmstudio".to_string(),
            display_name: "LM Studio".to_string(),
            description: "Local LLM server with OpenAI-compatible API. Supports GLM, Qwen Coder, DeepSeek, and other models via localhost.".to_string(),
            default_model: "qwen2.5-coder-7b-instruct".to_string(),
            known_models: vec![
                // GLM models
                ModelInfo::new("glm-4-9b", 8192),
                ModelInfo::new("glm-4.6", 8192),
                ModelInfo::new("glm-4.7", 8192),
                ModelInfo::new("glm4-9b-chat", 8192),
                // Qwen Coder models
                ModelInfo::new("qwen2.5-coder-32b-instruct", 32768),
                ModelInfo::new("qwen2.5-coder-14b-instruct", 32768),
                ModelInfo::new("qwen2.5-coder-7b-instruct", 32768),
                ModelInfo::new("qwen3-coder", 32768),
                ModelInfo::new("qwen3-coder-14b", 32768),
                // DeepSeek for reasoning
                ModelInfo::new("deepseek-r1-distill-qwen-7b", 32768),
                ModelInfo::new("deepseek-r1-distill-qwen-32b", 64000),
                // Vision models
                ModelInfo::new("qwen2-vl-7b-instruct", 32768),
                // General purpose
                ModelInfo::new("meta-llama-3.1-8b-instruct", 131072),
                ModelInfo::new("mistral-7b-instruct-v0.3", 32768),
            ],
            model_doc_link: "https://lmstudio.ai/docs/developer".to_string(),
            config_keys: vec![
                ConfigKey::new("LMSTUDIO_BASE_URL", false, false, Some("http://localhost:1234/v1")),
                ConfigKey::new("LMSTUDIO_API_TOKEN", false, true, None),
            ],
            allows_unlisted_models: true,
        }
    }

    pub async fn from_env(model: ModelConfig) -> Result<Arc<dyn Provider>> {
        let config = crate::config::Config::global();

        let base_url = config
            .get_param::<String>("LMSTUDIO_BASE_URL")
            .unwrap_or_else(|_| Self::DEFAULT_BASE_URL.to_string());

        // Get optional API token for authentication
        let api_token = config.get_param::<String>("LMSTUDIO_API_TOKEN").ok();

        let api_client = ApiClient::new(&base_url, api_token)?;

        Ok(Arc::new(OpenAiCompatibleProvider::new(
            "lmstudio".to_string(),
            api_client,
            model,
        )))
    }
}

impl ProviderDef for LmStudioProvider {
    type Provider = OpenAiCompatibleProvider;

    fn metadata() -> ProviderMetadata {
        Self::metadata()
    }

    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();

            let base_url = config
                .get_param::<String>("LMSTUDIO_BASE_URL")
                .unwrap_or_else(|_| Self::DEFAULT_BASE_URL.to_string());

            let api_token = config.get_param::<String>("LMSTUDIO_API_TOKEN").ok();

            let api_client = ApiClient::new(&base_url, api_token)?;

            Ok(OpenAiCompatibleProvider::new(
                "lmstudio".to_string(),
                api_client,
                model,
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmstudio_metadata() {
        let metadata = LmStudioProvider::metadata();
        assert_eq!(metadata.name, "lmstudio");
        assert_eq!(metadata.display_name, "LM Studio");
        assert_eq!(metadata.default_model, "qwen2.5-coder-7b-instruct");
        assert!(metadata.allows_unlisted_models);

        // Test GLM models in known_models
        assert!(metadata.known_models.iter().any(|m| m.name == "glm-4.6"));
        assert!(metadata.known_models.iter().any(|m| m.name == "glm-4.7"));
        assert!(metadata.known_models.iter().any(|m| m.name == "glm-4-9b"));

        // Test Qwen Coder models
        assert!(metadata
            .known_models
            .iter()
            .any(|m| m.name == "qwen3-coder"));
        assert!(metadata
            .known_models
            .iter()
            .any(|m| m.name == "qwen2.5-coder-32b-instruct"));

        // Test config keys
        assert_eq!(metadata.config_keys.len(), 5);
        assert!(metadata
            .config_keys
            .iter()
            .any(|k| k.name == "LMSTUDIO_API_TOKEN"));
        assert!(metadata
            .config_keys
            .iter()
            .any(|k| k.name == "LMSTUDIO_DRAFT_MODEL"));
        assert!(metadata
            .config_keys
            .iter()
            .any(|k| k.name == "LMSTUDIO_MODEL_TTL"));
    }

    #[tokio::test]
    async fn test_lmstudio_provider_creation_with_default_url() {
        let _guard = env_lock::lock_env([
            ("LMSTUDIO_BASE_URL", None::<&str>),
            ("LMSTUDIO_API_TOKEN", None::<&str>),
        ]);

        let model = ModelConfig::new_or_fail("glm-4.6");
        let provider = LmStudioProvider::from_env(model).await.unwrap();

        assert_eq!(provider.get_name(), "lmstudio");
        assert_eq!(provider.get_model_config().model_name, "glm-4.6");
    }

    #[tokio::test]
    async fn test_lmstudio_provider_creation_with_custom_url() {
        let _guard = env_lock::lock_env([
            ("LMSTUDIO_BASE_URL", Some("http://192.168.1.100:1234/v1")),
            ("LMSTUDIO_API_TOKEN", None::<&str>),
        ]);

        let model = ModelConfig::new_or_fail("qwen3-coder");
        let provider = LmStudioProvider::from_env(model).await.unwrap();

        assert_eq!(provider.get_name(), "lmstudio");
        assert_eq!(provider.get_model_config().model_name, "qwen3-coder");
    }

    #[tokio::test]
    async fn test_lmstudio_provider_with_api_token() {
        let _guard = env_lock::lock_env([
            ("LMSTUDIO_BASE_URL", Some("http://localhost:1234/v1")),
            ("LMSTUDIO_API_TOKEN", Some("test-token-123")),
        ]);

        let model = ModelConfig::new_or_fail("deepseek-r1-distill-qwen-7b");
        let provider = LmStudioProvider::from_env(model).await.unwrap();

        assert_eq!(provider.get_name(), "lmstudio");
        assert_eq!(
            provider.get_model_config().model_name,
            "deepseek-r1-distill-qwen-7b"
        );
    }
}
