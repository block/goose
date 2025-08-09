use crate::providers::base::ModelInfo;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// custom providers
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
}

/// load custom providers
pub fn load_custom_providers(dir: &Path) -> Result<Vec<CustomProviderConfig>> {
    let mut configs = Vec::new();

    if !dir.exists() {
        return Ok(configs);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&path)?;
            let config: CustomProviderConfig = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))?;

            configs.push(config);
        } else {
        }
    }

    Ok(configs)
}

/// register custom providers
pub fn register_custom_providers(
    registry: &mut crate::providers::provider_registry::ProviderRegistry,
    dir: &Path,
) -> Result<()> {
    use crate::model::ModelConfig;
    use crate::providers::{
        anthropic::AnthropicProvider, ollama::OllamaProvider, openai::OpenAiProvider,
    };

    let configs = load_custom_providers(dir)?;

    for config in configs {
        let config_clone = config.clone();

        match config.engine {
            ProviderEngine::OpenAI => {
                let description = config
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("Custom {} provider", config.display_name));
                let default_model = config
                    .models
                    .first()
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                let known_models: Vec<crate::providers::base::ModelInfo> = config
                    .models
                    .iter()
                    .map(|m| crate::providers::base::ModelInfo {
                        name: m.name.clone(),
                        context_limit: m.context_limit as usize,
                        input_token_cost: m.input_token_cost,
                        output_token_cost: m.output_token_cost,
                        currency: m.currency.clone(),
                        supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
                    })
                    .collect();

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
                let description = config
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("Custom {} provider", config.display_name));
                let default_model = config
                    .models
                    .first()
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                let known_models: Vec<crate::providers::base::ModelInfo> = config
                    .models
                    .iter()
                    .map(|m| crate::providers::base::ModelInfo {
                        name: m.name.clone(),
                        context_limit: m.context_limit as usize,
                        input_token_cost: m.input_token_cost,
                        output_token_cost: m.output_token_cost,
                        currency: m.currency.clone(),
                        supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
                    })
                    .collect();

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
                let description = config
                    .description
                    .clone()
                    .unwrap_or_else(|| format!("Custom {} provider", config.display_name));
                let default_model = config
                    .models
                    .first()
                    .map(|m| m.name.clone())
                    .unwrap_or_default();
                let known_models: Vec<crate::providers::base::ModelInfo> = config
                    .models
                    .iter()
                    .map(|m| crate::providers::base::ModelInfo {
                        name: m.name.clone(),
                        context_limit: m.context_limit as usize,
                        input_token_cost: m.input_token_cost,
                        output_token_cost: m.output_token_cost,
                        currency: m.currency.clone(),
                        supports_cache_control: Some(m.supports_cache_control.unwrap_or(false)),
                    })
                    .collect();

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
