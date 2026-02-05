use anyhow::Result;
use std::sync::Arc;

use super::api_client::ApiClient;
use super::base::{ConfigKey, Provider, ProviderMetadata};
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
        ProviderMetadata {
            name: "lmstudio".to_string(),
            display_name: "LM Studio".to_string(),
            config_keys: vec![
                ConfigKey {
                    name: "LMSTUDIO_BASE_URL".to_string(),
                    description: Some("LM Studio API base URL (default: http://localhost:1234/v1)".to_string()),
                    required: false,
                    secret: false,
                },
                ConfigKey {
                    name: "LMSTUDIO_API_TOKEN".to_string(),
                    description: Some("LM Studio API authentication token (optional)".to_string()),
                    required: false,
                    secret: true,
                },
                ConfigKey {
                    name: "LMSTUDIO_USE_NATIVE_API".to_string(),
                    description: Some("Use native /api/v1/* endpoints instead of OpenAI-compatible (default: false)".to_string()),
                    required: false,
                    secret: false,
                },
                ConfigKey {
                    name: "LMSTUDIO_DRAFT_MODEL".to_string(),
                    description: Some("Draft model for speculative decoding (optional)".to_string()),
                    required: false,
                    secret: false,
                },
                ConfigKey {
                    name: "LMSTUDIO_MODEL_TTL".to_string(),
                    description: Some("Idle TTL in seconds for auto-evict (optional)".to_string()),
                    required: false,
                    secret: false,
                },
            ],
            default_models: vec![
                // GLM models
                "glm-4-9b".to_string(),
                "glm-4.6".to_string(),
                "glm-4.7".to_string(),
                "glm4-9b-chat".to_string(),
                // Qwen Coder models
                "qwen2.5-coder-32b-instruct".to_string(),
                "qwen2.5-coder-14b-instruct".to_string(),
                "qwen2.5-coder-7b-instruct".to_string(),
                "qwen3-coder".to_string(),
                "qwen3-coder-14b".to_string(),
                // DeepSeek for reasoning
                "deepseek-r1-distill-qwen-7b".to_string(),
                "deepseek-r1-distill-qwen-32b".to_string(),
                // Vision models
                "qwen2-vl-7b-instruct".to_string(),
                // General purpose
                "meta-llama-3.1-8b-instruct".to_string(),
                "mistral-7b-instruct-v0.3".to_string(),
            ],
            supports_streaming: true,
            supports_tools: true,
        }
    }

    pub async fn new(model: ModelConfig) -> Result<Arc<dyn Provider>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmstudio_metadata() {
        let metadata = LmStudioProvider::metadata();
        assert_eq!(metadata.name, "lmstudio");
        assert_eq!(metadata.display_name, "LM Studio");
        assert!(metadata.supports_streaming);
        assert!(metadata.supports_tools);

        // Test GLM models
        assert!(metadata.default_models.contains(&"glm-4.6".to_string()));
        assert!(metadata.default_models.contains(&"glm-4.7".to_string()));
        assert!(metadata.default_models.contains(&"glm-4-9b".to_string()));

        // Test Qwen Coder models
        assert!(metadata.default_models.contains(&"qwen3-coder".to_string()));
        assert!(metadata
            .default_models
            .contains(&"qwen2.5-coder-32b-instruct".to_string()));

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
        let _guard =
            env_lock::lock_env([("LMSTUDIO_BASE_URL", None), ("LMSTUDIO_API_TOKEN", None)]);

        let model = ModelConfig::new_or_fail("glm-4.6");
        let provider = LmStudioProvider::new(model).await.unwrap();

        assert_eq!(provider.get_name(), "lmstudio");
        assert_eq!(provider.get_model_config().model_name, "glm-4.6");
    }

    #[tokio::test]
    async fn test_lmstudio_provider_creation_with_custom_url() {
        let _guard = env_lock::lock_env([
            ("LMSTUDIO_BASE_URL", Some("http://192.168.1.100:1234/v1")),
            ("LMSTUDIO_API_TOKEN", None),
        ]);

        let model = ModelConfig::new_or_fail("qwen3-coder");
        let provider = LmStudioProvider::new(model).await.unwrap();

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
        let provider = LmStudioProvider::new(model).await.unwrap();

        assert_eq!(provider.get_name(), "lmstudio");
        assert_eq!(
            provider.get_model_config().model_name,
            "deepseek-r1-distill-qwen-7b"
        );
    }
}
