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
        headers: Option<HashMap<String, String>>,
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
            headers,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_custom_provider_config_with_headers() {
        let temp_dir = TempDir::new().unwrap();
        let provider_dir = temp_dir.path().to_path_buf();

        // Create test headers
        let mut headers = HashMap::new();
        headers.insert(
            "x-origin-client-id".to_string(),
            "test-client-id".to_string(),
        );
        headers.insert("x-origin-secret".to_string(), "test-secret".to_string());

        // Create a custom provider config with headers
        let config = CustomProviderConfig {
            name: "test_provider".to_string(),
            engine: ProviderEngine::OpenAI,
            display_name: "Test Provider".to_string(),
            description: Some("Test provider with custom headers".to_string()),
            api_key_env: "TEST_PROVIDER_API_KEY".to_string(),
            base_url: "https://api.test.com/v1/chat/completions".to_string(),
            models: vec![
                ModelInfo::new("test-model-1", 128000),
                ModelInfo::new("test-model-2", 65536),
            ],
            headers: Some(headers.clone()),
            timeout_seconds: Some(300),
            supports_streaming: Some(true),
        };

        // Save the config to a file
        let json_content = serde_json::to_string_pretty(&config).unwrap();
        let file_path = provider_dir.join("test_provider.json");
        std::fs::write(&file_path, json_content).unwrap();

        // Load the config back and verify
        let loaded_configs = load_custom_providers(&provider_dir).unwrap();
        assert_eq!(loaded_configs.len(), 1);

        let loaded_config = &loaded_configs[0];
        assert_eq!(loaded_config.name, "test_provider");
        assert_eq!(loaded_config.display_name, "Test Provider");
        assert_eq!(
            loaded_config.base_url,
            "https://api.test.com/v1/chat/completions"
        );

        // Verify headers are loaded correctly
        assert!(loaded_config.headers.is_some());
        let loaded_headers = loaded_config.headers.as_ref().unwrap();
        assert_eq!(loaded_headers.len(), 2);
        assert_eq!(
            loaded_headers.get("x-origin-client-id").unwrap(),
            "test-client-id"
        );
        assert_eq!(
            loaded_headers.get("x-origin-secret").unwrap(),
            "test-secret"
        );
    }

    #[test]
    fn test_custom_provider_without_headers() {
        let temp_dir = TempDir::new().unwrap();
        let provider_dir = temp_dir.path().to_path_buf();

        // Create a custom provider config without headers
        let config = CustomProviderConfig {
            name: "test_provider_no_headers".to_string(),
            engine: ProviderEngine::OpenAI,
            display_name: "Test Provider No Headers".to_string(),
            description: Some("Test provider without custom headers".to_string()),
            api_key_env: "TEST_PROVIDER_API_KEY".to_string(),
            base_url: "https://api.test.com/v1/chat/completions".to_string(),
            models: vec![ModelInfo::new("test-model", 128000)],
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
        };

        // Save and load the config
        let json_content = serde_json::to_string_pretty(&config).unwrap();
        let file_path = provider_dir.join("test_provider_no_headers.json");
        std::fs::write(&file_path, json_content).unwrap();

        let loaded_configs = load_custom_providers(&provider_dir).unwrap();
        assert_eq!(loaded_configs.len(), 1);

        let loaded_config = &loaded_configs[0];
        assert!(loaded_config.headers.is_none());
    }

    #[test]
    fn test_generate_id() {
        assert_eq!(
            CustomProviderConfig::generate_id("My Custom Provider"),
            "custom_my_custom_provider"
        );
        assert_eq!(
            CustomProviderConfig::generate_id("Test API"),
            "custom_test_api"
        );
    }

    #[test]
    fn test_generate_api_key_name() {
        assert_eq!(
            CustomProviderConfig::generate_api_key_name("custom_test_provider"),
            "CUSTOM_TEST_PROVIDER_API_KEY"
        );
    }

    #[test]
    fn test_provider_engine_serialization() {
        // Test OpenAI engine
        let openai_engine = ProviderEngine::OpenAI;
        let serialized = serde_json::to_string(&openai_engine).unwrap();
        assert_eq!(serialized, "\"openai\"");

        let deserialized: ProviderEngine = serde_json::from_str(&serialized).unwrap();
        matches!(deserialized, ProviderEngine::OpenAI);

        // Test Anthropic engine
        let anthropic_engine = ProviderEngine::Anthropic;
        let serialized = serde_json::to_string(&anthropic_engine).unwrap();
        assert_eq!(serialized, "\"anthropic\"");

        // Test Ollama engine
        let ollama_engine = ProviderEngine::Ollama;
        let serialized = serde_json::to_string(&ollama_engine).unwrap();
        assert_eq!(serialized, "\"ollama\"");
    }
}
