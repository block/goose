use super::base::ModelInfo;
use super::provider_registry::ProviderRegistry;
use crate::model::ModelConfig;
use crate::providers::ollama::OllamaProvider;
use crate::providers::openai::OpenAiProvider;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEngine {
    OpenAI,
    Ollama,
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
    // Optional fields for OpenAI-compatible providers
    pub headers: Option<HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
}

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
        }
    }

    Ok(configs)
}

pub fn register_custom_providers(registry: &mut ProviderRegistry, dir: &Path) -> Result<()> {
    for config in load_custom_providers(dir)? {
        match config.engine {
            ProviderEngine::OpenAI => {
                registry.register(move |model: ModelConfig| {
                    OpenAiProvider::from_custom_config(model, config.clone())
                });
            }
            ProviderEngine::Ollama => {
                registry.register(move |model: ModelConfig| {
                    OllamaProvider::from_custom_config(model, config.clone())
                });
            }
        }
    }
    Ok(())
}
