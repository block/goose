use super::base::{Provider, ProviderDef, ProviderMetadata, DEFAULT_PROVIDER_TIMEOUT_SECS};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use anyhow::Result;
use futures::future::BoxFuture;
use futures::stream::{self, StreamExt};
use goose_providers::api_client::{ApiClient, AuthMethod};
use goose_providers::base::{ModelInfo, ProviderDescriptor};
use goose_providers::canonical::{map_to_canonical_model, CanonicalModelRegistry};
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use goose_providers::openai::OpenAiProvider;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tokio::sync::OnceCell;

use crate::conversation::message::Message;
use goose_providers::base::MessageStream;

const OLLAMA_CLOUD_PROVIDER_NAME: &str = "ollama_cloud";
const SHOW_CONCURRENCY: usize = 8;

#[derive(Clone)]
struct CachedShowInfo {
    context_limit: usize,
    tool_call: bool,
    reasoning: bool,
    modified_at: String,
}

static SHOW_INFO_CACHE: LazyLock<Mutex<HashMap<String, CachedShowInfo>>> =
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

        let normalized_base_url = goose_providers::openai::ensure_url_scheme(&config.base_url);
        let url = url::Url::parse(&normalized_base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

        let host = if let Some(port) = url.port() {
            format!(
                "{}://{}:{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                port
            )
        } else {
            format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
        };

        let global_config = crate::config::Config::global();
        let api_key = crate::providers::openai_def::resolve_api_key(&config, &|key| {
            global_config.get_secret(key)
        })?;

        let timeout_secs = config
            .timeout_seconds
            .unwrap_or(DEFAULT_PROVIDER_TIMEOUT_SECS);

        let auth = match api_key {
            Some(key) if !key.is_empty() => AuthMethod::BearerToken(key),
            _ => AuthMethod::NoAuth,
        };

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
                    let response = self
                        .ollama_api_client
                        .request(None, "api/tags")
                        .response_get()
                        .await
                        .map_err(|e| {
                            ProviderError::RequestFailed(format!(
                                "Failed to fetch Ollama Cloud models: {}",
                                e
                            ))
                        })?;

                    if !response.status().is_success() {
                        return Err(ProviderError::RequestFailed(format!(
                            "Failed to fetch models: HTTP {}",
                            response.status()
                        )));
                    }

                    let json: Value = response.json().await.map_err(|e| {
                        ProviderError::RequestFailed(format!("Failed to parse response: {}", e))
                    })?;

                    let mut names: Vec<String> = json
                        .get("models")
                        .and_then(|m| m.as_array())
                        .map(|models| {
                            models
                                .iter()
                                .filter_map(|m| {
                                    m.get("name").and_then(|n| n.as_str()).map(String::from)
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    names.sort();
                    Ok(names)
                })
            })
            .await
            .map(|v| v.to_vec())
    }

    async fn fetch_show_info(&self, model_name: &str) -> Option<CachedShowInfo> {
        let payload = serde_json::json!({ "model": model_name });
        let response = self
            .ollama_api_client
            .request(None, "api/show")
            .response_post(&payload)
            .await
            .ok()?;
        let json: Value = response.json().await.ok()?;

        let capabilities: Vec<String> = json
            .get("capabilities")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let context_limit = json
            .get("model_info")
            .and_then(|mi| mi.as_object())
            .and_then(|obj| {
                obj.iter().find_map(|(key, value)| {
                    key.ends_with(".context_length")
                        .then(|| value.as_u64().map(|v| v as usize))
                        .flatten()
                })
            })
            .unwrap_or(0);

        let modified_at = json
            .get("modified_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some(CachedShowInfo {
            context_limit,
            tool_call: capabilities.iter().any(|c| c == "tools"),
            reasoning: capabilities.iter().any(|c| c == "thinking"),
            modified_at,
        })
    }

    async fn get_or_fetch_show_info(&self, model_name: &str) -> Option<CachedShowInfo> {
        {
            let cache = SHOW_INFO_CACHE.lock().unwrap();
            if let Some(cached) = cache.get(model_name) {
                return Some(cached.clone());
            }
        }

        let info = self.fetch_show_info(model_name).await?;
        SHOW_INFO_CACHE
            .lock()
            .unwrap()
            .insert(model_name.to_string(), info.clone());
        Some(info)
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
            let config = crate::config::Config::global();
            let base_url = config
                .get_param::<String>("OLLAMA_CLOUD_BASE_URL")
                .unwrap_or_else(|_| "https://ollama.com/v1/chat/completions".to_string());

            let declarative_config = DeclarativeProviderConfig {
                name: OLLAMA_CLOUD_PROVIDER_NAME.to_string(),
                display_name: "Ollama Cloud".to_string(),
                description: Some(
                    "Access hosted models on ollama.com via OpenAI-compatible API".to_string(),
                ),
                engine: crate::config::declarative_providers::ProviderEngine::OpenAI,
                api_key_env: "OLLAMA_CLOUD_API_KEY".to_string(),
                base_url,
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
                catalog_provider_id: Some(OLLAMA_CLOUD_PROVIDER_NAME.to_string()),
                model_doc_link: None,
                setup_steps: vec![],
            };

            Self::from_custom_config(declarative_config, tls_config)
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
        if model_config.context_limit.is_some() {
            return Ok(model_config.context_limit.unwrap());
        }

        if let Some(info) = self.get_or_fetch_show_info(&model_config.model_name).await {
            if info.context_limit > 0 {
                return Ok(info.context_limit);
            }
        }

        Ok(model_config.context_limit())
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        self.get_or_fetch_model_names().await
    }

    async fn fetch_recommended_models(&self, toolshim: bool) -> Result<Vec<String>, ProviderError> {
        let all_models = self.fetch_supported_models().await?;

        let registry = CanonicalModelRegistry::bundled().map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to load canonical registry: {}", e))
        })?;

        let provider_name = self.get_name();

        let mut models_with_dates: Vec<(String, String)> = Vec::new();
        let mut unknown_models: Vec<String> = Vec::new();

        for model in &all_models {
            if let Some(canonical_id) = map_to_canonical_model(provider_name, model, registry) {
                if let Some((provider, model_name)) = canonical_id.split_once('/') {
                    if let Some(cm) = registry.get(provider, model_name) {
                        if !cm
                            .modalities
                            .input
                            .contains(&goose_providers::canonical::Modality::Text)
                        {
                            continue;
                        }
                        if !cm.tool_call && !toolshim {
                            continue;
                        }
                        models_with_dates
                            .push((model.clone(), cm.release_date.clone().unwrap_or_default()));
                        continue;
                    }
                }
            }
            unknown_models.push(model.clone());
        }

        if !unknown_models.is_empty() {
            let unknown_infos: Vec<(String, Option<CachedShowInfo>)> = stream::iter(unknown_models)
                .map(|m| async move { (m.clone(), self.get_or_fetch_show_info(&m).await) })
                .buffer_unordered(SHOW_CONCURRENCY)
                .collect()
                .await;

            for (model, info) in unknown_infos {
                let Some(info) = info else { continue };
                if !info.tool_call && !toolshim {
                    continue;
                }
                models_with_dates.push((model, info.modified_at));
            }
        }

        models_with_dates.sort_by(|a, b| b.1.cmp(&a.1));

        if models_with_dates.is_empty() {
            Ok(all_models)
        } else {
            Ok(models_with_dates.into_iter().map(|(m, _)| m).collect())
        }
    }

    async fn fetch_model_info(&self, model_name: &str) -> Result<ModelInfo, ProviderError> {
        let registry = CanonicalModelRegistry::bundled().ok();
        let canonical = registry.as_ref().and_then(|registry| {
            let canonical_id = map_to_canonical_model(self.get_name(), model_name, registry)?;
            let (provider, model) = canonical_id.split_once('/')?;
            registry.get(provider, model)
        });

        if let Some(canonical) = canonical {
            return Ok(ModelInfo {
                name: model_name.to_string(),
                resolved_model: None,
                context_limit: canonical.limit.context,
                input_token_cost: canonical.cost.input,
                output_token_cost: canonical.cost.output,
                currency: None,
                supports_cache_control: None,
                reasoning: canonical.reasoning.unwrap_or(false),
            });
        }

        if let Some(info) = self.get_or_fetch_show_info(model_name).await {
            return Ok(ModelInfo {
                name: model_name.to_string(),
                resolved_model: None,
                context_limit: info.context_limit,
                input_token_cost: None,
                output_token_cost: None,
                currency: None,
                supports_cache_control: None,
                reasoning: info.reasoning,
            });
        }

        Ok(ModelInfo::new(model_name, 0))
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

    async fn post_init(&self) {
        let model_name = self.inner.get_name().to_string();
        let _ = self.get_or_fetch_show_info(&model_name).await;
    }
}

#[cfg(test)]
mod tests {
    use goose_providers::canonical::{
        map_to_canonical_model, strip_version_suffix, CanonicalModelRegistry,
    };

    #[test]
    fn test_canonical_lookup_for_target_models() {
        let registry = CanonicalModelRegistry::bundled().expect("Failed to load registry");

        let models_to_check = vec![
            "kimi-k2.7-code",
            "glm-5.2",
            "kimi-k2.6",
            "glm-5.1",
            "kimi-k2.5",
            "deepseek-v4-pro",
            "gemma3:4b",
        ];

        for model in &models_to_check {
            let result = map_to_canonical_model("ollama_cloud", model, registry);
            match &result {
                Some(id) => {
                    let (provider, name) = id.split_once('/').unwrap_or(("?", "?"));
                    if let Some(cm) = registry.get(provider, name) {
                        eprintln!(
                            "{:>30} -> {:<40}  tool_call={}  release={:?}  ctx={}",
                            model, id, cm.tool_call, cm.release_date, cm.limit.context
                        );
                    } else {
                        eprintln!(
                            "{:>30} -> {:<40}  (registry lookup failed after mapping)",
                            model, id
                        );
                    }
                }
                None => {
                    eprintln!("{:>30} -> NOT FOUND (will use /api/show)", model);
                }
            }
        }

        eprintln!("\n=== strip_version_suffix ===");
        for model in &models_to_check {
            let stripped = strip_version_suffix(model);
            if stripped != *model {
                eprintln!("{:>30} -> stripped to: '{}'", model, stripped);
            } else {
                eprintln!("{:>30} -> unchanged", model);
            }
        }

        eprintln!("\n=== provider name comparison ===");
        for provider_name in &["ollama_cloud", "ollama-cloud"] {
            let result = map_to_canonical_model(provider_name, "kimi-k2.6", registry);
            eprintln!("{:>15} + kimi-k2.6 -> {:?}", provider_name, result);
        }
    }
}
