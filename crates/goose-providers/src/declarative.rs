use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::{
    api_client::TlsConfig,
    base::{ModelInfo, Provider},
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnvVarConfig {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
    /// When true, the field is shown prominently in the UI (not collapsed).
    /// Defaults to the value of `required` if not specified.
    pub primary: Option<bool>,
    pub description: Option<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEngine {
    OpenAI,
    Ollama,
    Anthropic,
}

impl FromStr for ProviderEngine {
    type Err = anyhow::Error;

    fn from_str(engine: &str) -> Result<Self> {
        match engine.trim().to_lowercase().as_str() {
            "openai" | "openai_compatible" => Ok(Self::OpenAI),
            "anthropic" | "anthropic_compatible" => Ok(Self::Anthropic),
            "ollama" | "ollama_compatible" => Ok(Self::Ollama),
            _ => Err(anyhow::anyhow!("Invalid provider type: {}", engine)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeclarativeProviderConfig {
    pub name: String,
    pub engine: ProviderEngine,
    pub display_name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub api_key_env: String,
    pub base_url: String,
    pub models: Vec<ModelInfo>,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub supports_streaming: Option<bool>,
    #[serde(default = "default_requires_auth")]
    pub requires_auth: bool,
    #[serde(default)]
    pub catalog_provider_id: Option<String>,
    #[serde(default)]
    pub base_path: Option<String>,
    #[serde(default)]
    pub env_vars: Option<Vec<EnvVarConfig>>,
    /// Controls whether `fetch_supported_models` calls the provider's `/v1/models`
    /// endpoint or returns the static `models` list directly.
    ///
    /// - `Some(false)` + non-empty `models`: return the static list; no API call.
    ///   Construction fails if `models` is empty.
    /// - `Some(true)` or `None`: try the API; fall back to `models` on 404.
    #[serde(default)]
    pub dynamic_models: Option<bool>,
    #[serde(default)]
    pub skip_canonical_filtering: bool,
    #[serde(default, deserialize_with = "deserialize_non_empty_string")]
    pub model_doc_link: Option<String>,
    #[serde(default)]
    pub setup_steps: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_non_empty_string")]
    pub fast_model: Option<String>,
    #[serde(default)]
    pub preserves_thinking: bool,
}

fn default_requires_auth() -> bool {
    true
}

/// Deserialize an optional string, treating empty/whitespace-only values as None.
fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.filter(|s| !s.trim().is_empty()))
}

impl DeclarativeProviderConfig {
    pub fn id(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }
}

pub trait KeyResolver {
    type Error: std::error::Error + Send + Sync + 'static;

    fn resolve_key(&self, key: &str) -> std::result::Result<String, Self::Error>;
}

pub struct EnvKeyResolver;

impl KeyResolver for EnvKeyResolver {
    type Error = std::env::VarError;

    fn resolve_key(&self, key: &str) -> std::result::Result<String, Self::Error> {
        std::env::var(key)
    }
}

pub fn from_json(
    json: &str,
    tls_config: Option<TlsConfig>,
    key_resolver: impl KeyResolver,
) -> Result<impl Provider> {
    let config: DeclarativeProviderConfig = serde_json::from_str(json)?;

    match config.engine {
        ProviderEngine::OpenAI => {
            crate::openai::from_custom_config(config, tls_config, key_resolver)
        }
        ProviderEngine::Ollama => todo!(),
        ProviderEngine::Anthropic => todo!(),
    }
}
