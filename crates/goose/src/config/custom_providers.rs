use crate::config::paths::Paths;
use crate::config::Config;
use crate::model::ModelConfig;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::base::ModelInfo;
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai::OpenAiProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub fn custom_providers_dir() -> std::path::PathBuf {
    Paths::config_dir().join("custom_providers")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEngine {
    OpenAI,
    Ollama,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    pub name: String,
    pub engine: ProviderEngine,
    pub display_name: String,
    pub description: Option<String>,
    pub api_key_env: String,
    pub base_url: String,
    pub models: Vec<ModelInfo>,
    pub headers: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub supports_streaming: Option<bool>,
}

impl CustomProviderConfig {
    pub fn id(&self) -> &str {
        &self.name
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    pub fn generate_id(display_name: &str) -> String {
        format!("custom_{}", display_name.to_lowercase().replace(' ', "_"))
    }

    pub fn generate_api_key_name(id: &str) -> String {
        format!("{}_API_KEY", id.to_uppercase())
    }

    pub fn create_and_save(
        provider_type: &str,
        display_name: String,
        api_url: String,
        api_key: String,
        models: Vec<String>,
        supports_streaming: Option<bool>,
    ) -> Result<Self> {
        let id = Self::generate_id(&display_name);
        let api_key_name = Self::generate_api_key_name(&id);

        let config = Config::global();
        config.set_secret(&api_key_name, serde_json::Value::String(api_key))?;

        let model_infos: Vec<ModelInfo> = models
            .into_iter()
            .map(|name| ModelInfo::new(name, 128000))
            .collect();

        let provider_config = CustomProviderConfig {
            name: id.clone(),
            engine: match provider_type {
                "openai_compatible" => ProviderEngine::OpenAI,
                "anthropic_compatible" => ProviderEngine::Anthropic,
                "ollama_compatible" => ProviderEngine::Ollama,
                _ => return Err(anyhow::anyhow!("Invalid provider type: {}", provider_type)),
            },
            display_name: display_name.clone(),
            description: Some(format!("Custom {} provider", display_name)),
            api_key_env: api_key_name,
            base_url: api_url,
            models: model_infos,
            headers: None,
            timeout_seconds: None,
            supports_streaming,
        };

        // save to JSON file
        let custom_providers_dir = custom_providers_dir();
        std::fs::create_dir_all(&custom_providers_dir)?;

        let json_content = serde_json::to_string_pretty(&provider_config)?;
        let file_path = custom_providers_dir.join(format!("{}.json", id));
        std::fs::write(file_path, json_content)?;

        Ok(provider_config)
    }

    pub fn remove(id: &str) -> Result<()> {
        let config = Config::global();
        let api_key_name = Self::generate_api_key_name(id);
        let _ = config.delete_secret(&api_key_name);

        let custom_providers_dir = custom_providers_dir();
        let file_path = custom_providers_dir.join(format!("{}.json", id));

        if file_path.exists() {
            std::fs::remove_file(file_path)?;
        }

        Ok(())
    }
}

pub fn load_custom_providers(dir: &Path) -> Result<Vec<CustomProviderConfig>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then_some(path)
        })
        .map(|path| {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))
        })
        .collect()
}

pub fn register_custom_providers(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    dir: &Path,
) -> Result<()> {
    let configs = load_custom_providers(dir)?;

    for config in configs {
        let config_clone = config.clone();
        let description = config
            .description
            .clone()
            .unwrap_or_else(|| format!("Custom {} provider", config.display_name));
        let default_model = config
            .models
            .first()
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let known_models: Vec<ModelInfo> = config
            .models
            .iter()
            .map(|m| ModelInfo {
                name: m.name.clone(),
                context_limit: m.context_limit,
                input_token_cost: m.input_token_cost,
                output_token_cost: m.output_token_cost,
                currency: m.currency.clone(),
                supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
            })
            .collect();

        match config.engine {
            ProviderEngine::OpenAI => {
                registry.register_with_name::<OpenAiProvider, _>(
                    config.name.clone(),
                    config.display_name.clone(),
                    description,
                    default_model,
                    known_models,
                    move |model: ModelConfig| {
                        OpenAiProvider::from_custom_config(model, config_clone.clone())
                    },
                );
            }
            ProviderEngine::Ollama => {
                registry.register_with_name::<OllamaProvider, _>(
                    config.name.clone(),
                    config.display_name.clone(),
                    description,
                    default_model,
                    known_models,
                    move |model: ModelConfig| {
                        OllamaProvider::from_custom_config(model, config_clone.clone())
                    },
                );
            }
            ProviderEngine::Anthropic => {
                registry.register_with_name::<AnthropicProvider, _>(
                    config.name.clone(),
                    config.display_name.clone(),
                    description,
                    default_model,
                    known_models,
                    move |model: ModelConfig| {
                        AnthropicProvider::from_custom_config(model, config_clone.clone())
                    },
                );
            }
        }
    }
    Ok(())
}
