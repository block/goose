use super::api_client::{ApiClient, AuthMethod};
use super::base::{
    ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata,
    DEFAULT_PROVIDER_TIMEOUT_SECS,
};
use super::errors::ProviderError;
use super::huggingface_auth;
use super::inventory::{default_inventory_identity, InventoryIdentityInput};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::config::{Config, ConfigError};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use anyhow::{anyhow, Result};
use futures::future::BoxFuture;
use rmcp::model::Tool;

pub const HUGGINGFACE_API_HOST: &str = "https://router.huggingface.co/v1";
pub const HUGGINGFACE_DOC_URL: &str = "https://huggingface.co/docs/inference-providers";
pub const HUGGINGFACE_DEFAULT_MODEL: &str = "Qwen/Qwen3-Coder-480B-A35B-Instruct";
pub const HUGGINGFACE_KNOWN_MODELS: &[&str] = &[
    "MiniMaxAI/MiniMax-M2.1",
    "MiniMaxAI/MiniMax-M2.5",
    "MiniMaxAI/MiniMax-M2.7",
    "Qwen/Qwen3-235B-A22B-Thinking",
    "Qwen/Qwen3-Coder-480B-A35B-Instruct",
    "Qwen/Qwen3-Coder-Next",
    "Qwen/Qwen3-Embedding-4B",
    "Qwen/Qwen3-Embedding-8B",
    "Qwen/Qwen3-Next-80B-A3B-Instruct",
    "Qwen/Qwen3-Next-80B-A3B-Thinking",
    "Qwen/Qwen3.5-397B-A17B",
    "XiaomiMiMo/MiMo-V2-Flash",
    "deepseek-ai/DeepSeek-R1",
    "deepseek-ai/DeepSeek-V3.2",
    "deepseek-ai/DeepSeek-V4-Pro",
    "moonshotai/Kimi-K2-Instruct",
    "moonshotai/Kimi-K2-Thinking",
    "moonshotai/Kimi-K2.5",
    "moonshotai/Kimi-K2.6",
    "zai-org/GLM-4.7",
    "zai-org/GLM-4.7-Flash",
    "zai-org/GLM-5",
    "zai-org/GLM-5.1",
];

type QueryParams = Vec<(String, String)>;
type EndpointParts = (String, String, QueryParams);

pub struct HuggingFaceProvider {
    inner: OpenAiCompatibleProvider,
}

impl HuggingFaceProvider {
    pub fn matches_declarative_config(config: &DeclarativeProviderConfig) -> bool {
        config.name == huggingface_auth::HUGGINGFACE_PROVIDER_NAME
            || config.catalog_provider_id.as_deref()
                == Some(huggingface_auth::HUGGINGFACE_PROVIDER_NAME)
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let configured_key = configured_api_key(&config)?;
        let token = huggingface_auth::resolve_token_with_provider_token(configured_key)?
            .ok_or_else(missing_token_error)?;
        let (host, completions_prefix, query_params) =
            openai_compatible_endpoint_parts(&config.base_url, config.base_path.as_deref())?;

        let timeout_secs = config
            .timeout_seconds
            .unwrap_or(DEFAULT_PROVIDER_TIMEOUT_SECS);
        let mut api_client = ApiClient::with_timeout(
            host,
            AuthMethod::BearerToken(token),
            std::time::Duration::from_secs(timeout_secs),
        )?
        .with_query(query_params);

        if let Some(headers) = &config.headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        let model = if let Some(ref fast_model_name) = config.fast_model {
            model.with_fast(fast_model_name, &config.name)?
        } else {
            model
        };

        Ok(Self {
            inner: OpenAiCompatibleProvider::new(
                config.name.clone(),
                api_client,
                model,
                completions_prefix,
            ),
        })
    }

    pub async fn cleanup() -> Result<()> {
        huggingface_auth::clear_oauth_token()
    }
}

#[async_trait::async_trait]
impl Provider for HuggingFaceProvider {
    fn get_name(&self) -> &str {
        self.inner.get_name()
    }

    fn get_model_config(&self) -> ModelConfig {
        self.inner.get_model_config()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        self.inner.fetch_supported_models().await
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
}

impl ProviderDef for HuggingFaceProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            huggingface_auth::HUGGINGFACE_PROVIDER_NAME,
            huggingface_auth::HUGGINGFACE_DISPLAY_NAME,
            "Hugging Face Inference Providers via the Hugging Face Router",
            HUGGINGFACE_DEFAULT_MODEL,
            HUGGINGFACE_KNOWN_MODELS.to_vec(),
            HUGGINGFACE_DOC_URL,
            vec![
                ConfigKey::new(
                    huggingface_auth::HUGGINGFACE_TOKEN_SECRET_KEY,
                    true,
                    true,
                    None,
                    true,
                ),
                ConfigKey::new("HF_HOST", false, false, Some(HUGGINGFACE_API_HOST), false),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(async move {
            let config = Config::global();
            let token = huggingface_auth::resolve_token()?.ok_or_else(missing_token_error)?;
            let host: String = config
                .get_param("HF_HOST")
                .unwrap_or_else(|_| HUGGINGFACE_API_HOST.to_string());
            let api_client = ApiClient::new(host, AuthMethod::BearerToken(token))?;

            Ok(Self {
                inner: OpenAiCompatibleProvider::new(
                    huggingface_auth::HUGGINGFACE_PROVIDER_NAME.to_string(),
                    api_client,
                    model,
                    String::new(),
                ),
            })
        })
    }

    fn inventory_identity() -> Result<InventoryIdentityInput> {
        let metadata = Self::metadata();
        Ok(default_inventory_identity(
            &metadata.name,
            &metadata.name,
            &metadata.config_keys,
            Config::global(),
        ))
    }

    fn inventory_configured() -> bool {
        huggingface_auth::usable_oauth_token().is_some()
            || huggingface_auth::hf_token_secret().ok().flatten().is_some()
    }
}

fn missing_token_error() -> anyhow::Error {
    anyhow!(
        "Hugging Face token is not configured. Sign in from Settings > Auth or configure HF_TOKEN."
    )
}

fn configured_api_key(config: &DeclarativeProviderConfig) -> Result<Option<String>> {
    if config.api_key_env.is_empty() {
        return Ok(None);
    }

    match Config::global().get_secret::<String>(&config.api_key_env) {
        Ok(token) => Ok(Some(token)),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn openai_compatible_endpoint_parts(
    base_url: &str,
    base_path: Option<&str>,
) -> Result<EndpointParts> {
    let url =
        url::Url::parse(base_url).map_err(|e| anyhow!("Invalid base URL '{}': {}", base_url, e))?;
    let mut host = if let Some(port) = url.port() {
        format!(
            "{}://{}:{}",
            url.scheme(),
            url.host_str().unwrap_or_default(),
            port
        )
    } else {
        format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default())
    };
    let query_params = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();

    if let Some(path) = base_path {
        return Ok((host, completions_prefix(path), query_params));
    }

    let path = url.path().trim_matches('/');
    if path.is_empty() {
        return Ok((host, String::new(), query_params));
    }

    if let Some(parent) = path
        .strip_suffix("/chat/completions")
        .or_else(|| (path == "chat/completions").then_some(""))
    {
        if !parent.is_empty() {
            host.push('/');
            host.push_str(parent);
        }
        return Ok((host, String::new(), query_params));
    }

    host.push('/');
    host.push_str(path);
    Ok((host, String::new(), query_params))
}

fn completions_prefix(path: &str) -> String {
    let path = path.trim_matches('/');
    if path.is_empty() {
        return String::new();
    }

    let parent = path
        .strip_suffix("/chat/completions")
        .or_else(|| (path == "chat/completions").then_some(""))
        .unwrap_or(path);

    if parent.is_empty() {
        String::new()
    } else {
        format!("{}/", parent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_preserves_huggingface_id_and_token_key() {
        let metadata = HuggingFaceProvider::metadata();
        assert_eq!(metadata.name, "huggingface");
        assert_eq!(metadata.display_name, "Hugging Face");
        assert_eq!(metadata.default_model, HUGGINGFACE_DEFAULT_MODEL);
        assert!(metadata
            .config_keys
            .iter()
            .any(|key| key.name == "HF_TOKEN" && key.secret));
    }

    #[test]
    fn declarative_matching_accepts_name_or_catalog_provider_id() {
        let mut config = test_config();
        assert!(!HuggingFaceProvider::matches_declarative_config(&config));

        config.name = "huggingface".to_string();
        assert!(HuggingFaceProvider::matches_declarative_config(&config));

        config.name = "custom_hugging_face".to_string();
        config.catalog_provider_id = Some("huggingface".to_string());
        assert!(HuggingFaceProvider::matches_declarative_config(&config));
    }

    #[test]
    fn endpoint_parts_use_base_url_path_as_api_host() {
        let (host, prefix, query) =
            openai_compatible_endpoint_parts("https://router.huggingface.co/v1?beta=1", None)
                .unwrap();
        assert_eq!(host, "https://router.huggingface.co/v1");
        assert_eq!(prefix, "");
        assert_eq!(query, vec![("beta".to_string(), "1".to_string())]);
    }

    #[test]
    fn endpoint_parts_strip_chat_completions_suffix() {
        let (host, prefix, query) = openai_compatible_endpoint_parts(
            "https://router.huggingface.co/v1/chat/completions",
            None,
        )
        .unwrap();
        assert_eq!(host, "https://router.huggingface.co/v1");
        assert_eq!(prefix, "");
        assert!(query.is_empty());
    }

    #[test]
    fn endpoint_parts_respect_explicit_base_path() {
        let (host, prefix, query) = openai_compatible_endpoint_parts(
            "https://router.huggingface.co",
            Some("v1/chat/completions"),
        )
        .unwrap();
        assert_eq!(host, "https://router.huggingface.co");
        assert_eq!(prefix, "v1/");
        assert!(query.is_empty());
    }

    fn test_config() -> DeclarativeProviderConfig {
        DeclarativeProviderConfig {
            name: "custom_provider".to_string(),
            engine: crate::config::declarative_providers::ProviderEngine::OpenAI,
            display_name: "Custom Provider".to_string(),
            description: None,
            api_key_env: "CUSTOM_API_KEY".to_string(),
            base_url: HUGGINGFACE_API_HOST.to_string(),
            models: Vec::new(),
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
            requires_auth: true,
            catalog_provider_id: None,
            base_path: None,
            env_vars: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            model_doc_link: None,
            setup_steps: vec![],
            fast_model: None,
            preserves_thinking: true,
        }
    }
}
