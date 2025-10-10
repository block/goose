use std::collections::HashMap;

use anyhow::Result;

use crate::config::custom_providers::{CustomProviderConfig, ProviderEngine};
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, ModelInfo, ProviderMetadata};
use crate::providers::openai::OpenAiProvider;
use crate::providers::provider_registry::ProviderRegistry;

const PROVIDER_NAME: &str = "mistral";
const DISPLAY_NAME: &str = "Mistral AI";
const DESCRIPTION: &str = "Frontier models from Mistral AI";
const DOC_URL: &str = "https://docs.mistral.ai/";
const DEFAULT_MODEL: &str = "mistral-medium-latest";
const DEFAULT_FAST_MODEL: &str = "mistral-small-2506";
const DEFAULT_HOST: &str = "https://api.mistral.ai";
const DEFAULT_BASE_PATH: &str = "v1/chat/completions";
const DEFAULT_MODELS_PATH: &str = "v1/models";
const KNOWN_MODELS: &[(&str, usize)] = &[
    (DEFAULT_MODEL, 128_000),
    (DEFAULT_FAST_MODEL, 128_000),
    ("mistral-medium-2508", 128_000),
    ("magistral-medium-2509", 128_000),
    ("codestral-2508", 256_000),
    ("pixtral-large-2411", 128_000),
    ("ministral-8b-2410", 128_000),
    ("mistral-medium-2505", 128_000),
    ("ministral-3b-2410", 128_000),
];

pub(super) fn register(registry: &mut ProviderRegistry) -> Result<()> {
    let metadata = ProviderMetadata::with_models(
        PROVIDER_NAME,
        DISPLAY_NAME,
        DESCRIPTION,
        DEFAULT_MODEL,
        models(),
        DOC_URL,
        config_keys(),
    );

    registry.register_with_metadata::<OpenAiProvider, _>(metadata, move |model: ModelConfig| {
        let provider_config = provider_config()?;
        let model_with_fast = model.with_fast(DEFAULT_FAST_MODEL.to_string());
        OpenAiProvider::from_custom_config(model_with_fast, provider_config)
    });

    Ok(())
}

fn models() -> Vec<ModelInfo> {
    KNOWN_MODELS
        .iter()
        .map(|(name, limit)| ModelInfo::new(*name, *limit))
        .collect()
}

fn config_keys() -> Vec<ConfigKey> {
    vec![
        ConfigKey::new("MISTRAL_API_KEY", true, true, None),
        ConfigKey::new("MISTRAL_HOST", false, false, Some(DEFAULT_HOST)),
        ConfigKey::new("MISTRAL_BASE_PATH", false, false, Some(DEFAULT_BASE_PATH)),
        ConfigKey::new(
            "MISTRAL_MODELS_PATH",
            false,
            false,
            Some(DEFAULT_MODELS_PATH),
        ),
        ConfigKey::new("MISTRAL_CUSTOM_HEADERS", false, true, None),
        ConfigKey::new("MISTRAL_TIMEOUT", false, false, Some("600")),
    ]
}

fn provider_config() -> Result<CustomProviderConfig> {
    let config = crate::config::Config::global();
    let host: String = config
        .get_param("MISTRAL_HOST")
        .unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let base_path: String = config
        .get_param("MISTRAL_BASE_PATH")
        .unwrap_or_else(|_| DEFAULT_BASE_PATH.to_string());
    let models_path: String = config
        .get_param("MISTRAL_MODELS_PATH")
        .unwrap_or_else(|_| DEFAULT_MODELS_PATH.to_string());
    let timeout_seconds = config.get_param::<u64>("MISTRAL_TIMEOUT").ok();
    let custom_headers: Option<HashMap<String, String>> = config
        .get_secret::<String>("MISTRAL_CUSTOM_HEADERS")
        .or_else(|_| config.get_param::<String>("MISTRAL_CUSTOM_HEADERS"))
        .ok()
        .and_then(|raw| {
            let parsed = parse_custom_headers(&raw);
            if parsed.is_empty() {
                None
            } else {
                Some(parsed)
            }
        });

    let base_url = format!(
        "{}/{}",
        host.trim_end_matches('/'),
        base_path.trim_start_matches('/')
    );

    Ok(CustomProviderConfig {
        name: PROVIDER_NAME.to_string(),
        engine: ProviderEngine::OpenAI,
        display_name: DISPLAY_NAME.to_string(),
        description: Some(DESCRIPTION.to_string()),
        api_key_env: "MISTRAL_API_KEY".to_string(),
        base_url,
        models: models(),
        models_path: Some(models_path),
        headers: custom_headers,
        timeout_seconds,
        supports_streaming: Some(true),
    })
}

fn parse_custom_headers(value: &str) -> HashMap<String, String> {
    value
        .split(',')
        .filter_map(|header| {
            let mut parts = header.splitn(2, '=');
            let key = parts.next()?.trim();
            let val = parts.next()?.trim();

            if key.is_empty() || val.is_empty() {
                return None;
            }

            Some((key.to_string(), val.to_string()))
        })
        .collect()
}
