use super::base::{MessageStream, Provider, ProviderDef, ProviderMetadata};
use crate::config::declarative_providers::{load_provider, DeclarativeProviderConfig};
use anyhow::Result;
use futures::future::BoxFuture;
use goose_providers::api_client::ApiClient;
use goose_providers::base::ProviderDescriptor;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use goose_providers::openai::OpenAiProvider;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::OnceCell;

use crate::conversation::message::Message;

const OLLAMA_CLOUD_PROVIDER_NAME: &str = "ollama_cloud";

static SHOW_INFO_CACHE: LazyLock<Mutex<HashMap<String, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct OllamaCloudProvider {
    inner: OpenAiProvider,
    ollama_api_client: ApiClient,
    model_names: OnceCell<Vec<String>>,
}

impl OllamaCloudProvider {
    pub fn matches_declarative_config(config: &DeclarativeProviderConfig) -> bool {
        config.name == OLLAMA_CLOUD_PROVIDER_NAME
            || config.catalog_provider_id.as_deref() == Some(OLLAMA_CLOUD_PROVIDER_NAME)
    }

    pub fn from_custom_config(
        config: DeclarativeProviderConfig,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> Result<Self> {
        let inner =
            crate::providers::openai_def::from_custom_config(config.clone(), tls_config.clone())?;

        let (host, _, auth, timeout_secs) =
            crate::providers::openai_def::extract_host_and_auth(&config)?;

        let ollama_api_client = ApiClient::with_timeout_and_tls(
            host,
            auth,
            std::time::Duration::from_secs(timeout_secs),
            tls_config,
        )?;

        Ok(Self {
            inner,
            ollama_api_client,
            model_names: OnceCell::new(),
        })
    }

    async fn get_or_fetch_model_names(&self) -> Result<Vec<String>, ProviderError> {
        self.model_names
            .get_or_try_init(|| {
                Box::pin(async {
                    Ok(
                        super::utils::fetch_ollama_model_names(&self.ollama_api_client)
                            .await?
                            .unwrap_or_default(),
                    )
                })
            })
            .await
            .map(|v| v.to_vec())
    }

    async fn fetch_context_limit_from_show(&self, model_name: &str) -> Option<usize> {
        let payload = serde_json::json!({ "model": model_name });
        let response = self
            .ollama_api_client
            .request(None, "api/show")
            .response_post(&payload)
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let json: Value = response.json().await.ok()?;

        json.get("model_info")
            .and_then(|mi| mi.as_object())
            .and_then(|obj| {
                obj.iter().find_map(|(key, value)| {
                    key.ends_with(".context_length")
                        .then(|| value.as_u64().map(|v| v as usize))
                        .flatten()
                })
            })
    }

    async fn get_or_fetch_context_limit(&self, model_name: &str) -> Option<usize> {
        {
            let cache = SHOW_INFO_CACHE.lock().unwrap();
            if let Some(&cached) = cache.get(model_name) {
                return Some(cached);
            }
        }

        let limit = self.fetch_context_limit_from_show(model_name).await;
        if let Some(l) = limit {
            SHOW_INFO_CACHE
                .lock()
                .unwrap()
                .insert(model_name.to_string(), l);
        }
        limit
    }
}

impl ProviderDescriptor for OllamaCloudProvider {
    fn metadata() -> ProviderMetadata {
        OpenAiProvider::metadata()
    }
}

impl ProviderDef for OllamaCloudProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
            let loaded = load_provider(OLLAMA_CLOUD_PROVIDER_NAME)?;
            Self::from_custom_config(loaded.config, tls_config)
        })
    }
}

#[async_trait::async_trait]
impl Provider for OllamaCloudProvider {
    fn get_name(&self) -> &str {
        self.inner.get_name()
    }

    fn retry_config(&self) -> goose_providers::retry::RetryConfig {
        self.inner.retry_config()
    }

    async fn get_context_limit(&self, model_config: &ModelConfig) -> Result<usize, ProviderError> {
        if let Some(limit) = model_config.context_limit {
            return Ok(limit);
        }

        if let Some(limit) = self
            .get_or_fetch_context_limit(&model_config.model_name)
            .await
        {
            if limit > 0 {
                return Ok(limit);
            }
        }

        self.inner.get_context_limit(model_config).await
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        self.get_or_fetch_model_names().await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.inner
            .stream(model_config, session_id, system, messages, tools)
            .await
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.inner.skip_canonical_filtering()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_providers::canonical::{map_to_canonical_model, CanonicalModelRegistry};

    #[test]
    fn test_ollama_cloud_maps_to_canonical() {
        let registry = CanonicalModelRegistry::bundled().expect("Failed to load registry");
        let result = map_to_canonical_model("ollama_cloud", "kimi-k2.6", registry);
        assert!(
            result.is_some(),
            "kimi-k2.6 should map to a canonical model via ollama_cloud"
        );
    }

    #[test]
    fn test_matches_declarative_config_by_name() {
        let config = DeclarativeProviderConfig {
            name: "ollama_cloud".to_string(),
            display_name: "Ollama Cloud".to_string(),
            description: None,
            engine: crate::config::declarative_providers::ProviderEngine::OpenAI,
            api_key_env: "OLLAMA_CLOUD_API_KEY".to_string(),
            base_url: "https://ollama.com/v1/chat/completions".to_string(),
            base_path: None,
            models: vec![],
            dynamic_models: Some(true),
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
            requires_auth: true,
            env_vars: None,
            skip_canonical_filtering: false,
            preserves_thinking: false,
            fast_model: None,
            catalog_provider_id: None,
            model_doc_link: None,
            setup_steps: vec![],
        };
        assert!(OllamaCloudProvider::matches_declarative_config(&config));
    }

    #[test]
    fn test_matches_declarative_config_by_catalog_id() {
        let config = DeclarativeProviderConfig {
            name: "custom_ollama".to_string(),
            display_name: "Custom Ollama".to_string(),
            description: None,
            engine: crate::config::declarative_providers::ProviderEngine::OpenAI,
            api_key_env: "OLLAMA_CLOUD_API_KEY".to_string(),
            base_url: "https://ollama.com/v1/chat/completions".to_string(),
            base_path: None,
            models: vec![],
            dynamic_models: Some(true),
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
            requires_auth: true,
            env_vars: None,
            skip_canonical_filtering: false,
            preserves_thinking: false,
            fast_model: None,
            catalog_provider_id: Some("ollama_cloud".to_string()),
            model_doc_link: None,
            setup_steps: vec![],
        };
        assert!(OllamaCloudProvider::matches_declarative_config(&config));
    }
}
