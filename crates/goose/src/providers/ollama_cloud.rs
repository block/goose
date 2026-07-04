use super::base::{MessageStream, Provider, ProviderDef, ProviderMetadata};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::config::Config;
use crate::conversation::message::Message;
use anyhow::Result;
use futures::future::BoxFuture;
use goose_providers::api_client::{ApiClient, AuthMethod, TlsConfig};
use goose_providers::base::ProviderDescriptor;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use goose_providers::ollama::fetch_ollama_model_names;
use goose_providers::openai::OpenAiProvider;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::OnceCell;

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
        tls_config: Option<TlsConfig>,
    ) -> Result<Self> {
        let inner =
            crate::providers::openai_def::from_custom_config(config.clone(), tls_config.clone())?;

        let ollama_api_client = build_ollama_api_client(&config, tls_config)?;

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
                    Ok(fetch_ollama_model_names(&self.ollama_api_client)
                        .await?
                        .unwrap_or_default())
                })
            })
            .await
            .map(|v| v.to_vec())
    }

    async fn fetch_context_limit_from_show(&self, model_name: &str) -> Option<usize> {
        let payload = serde_json::json!({ "model": model_name });
        let response = self
            .ollama_api_client
            .request("api/show")
            .response_post(&payload)
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let json: Value = response.json().await.ok()?;
        json.get("model_info")
            .and_then(|info| info.as_object())
            .and_then(|obj| {
                obj.values().find_map(|v| {
                    v.get("context_length")
                        .and_then(|cl| cl.as_u64())
                        .map(|n| n as usize)
                })
            })
    }
}

fn build_ollama_api_client(
    config: &DeclarativeProviderConfig,
    tls_config: Option<TlsConfig>,
) -> Result<ApiClient> {
    let normalized_base_url = goose_providers::openai::ensure_url_scheme(&config.base_url);
    let url = url::Url::parse(&normalized_base_url)
        .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;
    let host = url[..url::Position::BeforePath].to_string();

    let api_key = crate::providers::openai_def::resolve_api_key(config, &|key| {
        Config::global().get_secret(key)
    })?;

    let timeout_secs = config
        .timeout_seconds
        .unwrap_or(crate::providers::base::DEFAULT_PROVIDER_TIMEOUT_SECS);

    let auth = match api_key {
        Some(key) if !key.is_empty() => AuthMethod::BearerToken(key),
        _ => AuthMethod::NoAuth,
    };

    Ok(ApiClient::with_timeout_and_tls(
        host,
        auth,
        std::time::Duration::from_secs(timeout_secs),
        tls_config,
    )?
    .with_request_builder(crate::session_context::session_id_request_builder()))
}

#[async_trait::async_trait]
impl Provider for OllamaCloudProvider {
    fn get_name(&self) -> &str {
        self.inner.get_name()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        self.inner
            .stream(model_config, system, messages, tools)
            .await
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.inner.skip_canonical_filtering()
    }

    fn retry_config(&self) -> goose_providers::retry::RetryConfig {
        self.inner.retry_config()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        self.get_or_fetch_model_names().await
    }

    async fn get_context_limit(&self, model_config: &ModelConfig) -> Result<usize, ProviderError> {
        if let Some(limit) = model_config.context_limit {
            return Ok(limit);
        }

        if let Some(cached) = SHOW_INFO_CACHE
            .lock()
            .ok()
            .and_then(|cache| cache.get(&model_config.model_name).copied())
        {
            return Ok(cached);
        }

        let limit = self
            .fetch_context_limit_from_show(&model_config.model_name)
            .await
            .unwrap_or_else(|| model_config.context_limit());

        if let Ok(mut cache) = SHOW_INFO_CACHE.lock() {
            cache.insert(model_config.model_name.clone(), limit);
        }

        Ok(limit)
    }
}

impl ProviderDescriptor for OllamaCloudProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OLLAMA_CLOUD_PROVIDER_NAME,
            "Ollama Cloud",
            "Access hosted models on ollama.com via OpenAI-compatible API",
            "qwen3-coder:480b-cloud",
            vec![],
            "https://ollama.com/library",
            vec![],
        )
    }
}

impl ProviderDef for OllamaCloudProvider {
    type Provider = Self;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        _tls_config: Option<TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async {
            anyhow::bail!(
                "Ollama Cloud must be configured as a declarative provider. \
                 Run `goose configure` to set it up."
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::declarative_providers::ProviderEngine;

    #[test]
    fn declarative_matching_accepts_name_or_catalog_provider_id() {
        let mut config = test_config();
        config.name = "custom_ollama".to_string();
        assert!(!OllamaCloudProvider::matches_declarative_config(&config));

        config.name = OLLAMA_CLOUD_PROVIDER_NAME.to_string();
        assert!(OllamaCloudProvider::matches_declarative_config(&config));

        config.name = "custom_ollama".to_string();
        config.catalog_provider_id = Some(OLLAMA_CLOUD_PROVIDER_NAME.to_string());
        assert!(OllamaCloudProvider::matches_declarative_config(&config));
    }

    fn test_config() -> DeclarativeProviderConfig {
        DeclarativeProviderConfig {
            name: OLLAMA_CLOUD_PROVIDER_NAME.to_string(),
            engine: ProviderEngine::OpenAI,
            display_name: "Ollama Cloud".to_string(),
            description: None,
            api_key_env: "OLLAMA_CLOUD_API_KEY".to_string(),
            base_url: "https://ollama.com/v1/chat/completions".to_string(),
            models: Vec::new(),
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
            requires_auth: true,
            catalog_provider_id: None,
            base_path: None,
            env_vars: None,
            dynamic_models: Some(true),
            skip_canonical_filtering: false,
            model_doc_link: None,
            setup_steps: vec![],
            fast_model: None,
            preserves_thinking: true,
        }
    }
}
