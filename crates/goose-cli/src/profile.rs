use anyhow::Result;
use goose::key_manager::{get_keyring_secret, KeyRetrievalStrategy};
use goose::providers::configs::{
    AnthropicProviderConfig, DatabricksAuth, DatabricksProviderConfig, GoogleProviderConfig,
    GroqProviderConfig, ModelConfig, OllamaProviderConfig, OpenAiProviderConfig, ProviderConfig,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Profile types and structures
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub context_limit: Option<usize>,
    pub max_tokens: Option<i32>,
    pub estimate_factor: Option<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct Profiles {
    pub profile_items: HashMap<String, Profile>,
}

pub fn profile_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Could not determine home directory"))?;
    let config_dir = home_dir.join(".config").join("goose");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir.join("profiles.json"))
}

pub fn load_profiles() -> Result<HashMap<String, Profile>> {
    let path = profile_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path)?;
    let profiles: Profiles = serde_json::from_str(&content)?;
    Ok(profiles.profile_items)
}

pub fn save_profile(name: &str, profile: Profile) -> Result<()> {
    let path = profile_path()?;
    let mut profiles = load_profiles()?;
    profiles.insert(name.to_string(), profile);
    let profiles = Profiles {
        profile_items: profiles,
    };
    let content = serde_json::to_string_pretty(&profiles)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn find_existing_profile(name: &str) -> Option<Profile> {
    match load_profiles() {
        Ok(profiles) => profiles.get(name).cloned(),
        Err(_) => None,
    }
}

pub fn has_no_profiles() -> Result<bool> {
    load_profiles().map(|profiles| Ok(profiles.is_empty()))?
}

pub fn get_provider_config(provider_name: &str, profile: Profile) -> ProviderConfig {
    let model_config = ModelConfig::new(profile.model)
        .with_context_limit(profile.context_limit)
        .with_temperature(profile.temperature)
        .with_max_tokens(profile.max_tokens)
        .with_estimate_factor(profile.estimate_factor);

    match provider_name.to_lowercase().as_str() {
        "openai" => {
            // TODO error propagation throughout the CLI
            let api_key = get_keyring_secret("OPENAI_API_KEY", KeyRetrievalStrategy::Both)
                .expect("OPENAI_API_KEY not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::OpenAi(OpenAiProviderConfig {
                host: "https://api.openai.com".to_string(),
                api_key,
                model: model_config,
            })
        }
        "databricks" => {
            let host = get_keyring_secret("DATABRICKS_HOST", KeyRetrievalStrategy::Both)
                .expect("DATABRICKS_HOST not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Databricks(DatabricksProviderConfig {
                host: host.clone(),
                // TODO revisit configuration
                auth: DatabricksAuth::oauth(host),
                model: model_config,
                image_format: goose::providers::utils::ImageFormat::Anthropic,
            })
        }
        "ollama" => {
            let host = get_keyring_secret("OLLAMA_HOST", KeyRetrievalStrategy::Both)
                .expect("OLLAMA_HOST not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Ollama(OllamaProviderConfig {
                host,
                model: model_config,
            })
        }
        "anthropic" => {
            let api_key = get_keyring_secret("ANTHROPIC_API_KEY", KeyRetrievalStrategy::Both)
                .expect("ANTHROPIC_API_KEY not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Anthropic(AnthropicProviderConfig {
                host: "https://api.anthropic.com".to_string(),
                api_key,
                model: model_config,
            })
        }
        "google" => {
            let api_key = get_keyring_secret("GOOGLE_API_KEY", KeyRetrievalStrategy::Both)
                .expect("GOOGLE_API_KEY not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Google(GoogleProviderConfig {
                host: "https://generativelanguage.googleapis.com".to_string(),
                api_key,
                model: model_config,
            })
        }
        "groq" => {
            let api_key = get_keyring_secret("GROQ_API_KEY", KeyRetrievalStrategy::Both)
                .expect("GROQ_API_KEY not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Groq(GroqProviderConfig {
                host: "https://api.groq.com".to_string(),
                api_key,
                model: model_config,
            })
        }
        _ => panic!("Invalid provider name"),
    }
}

#[cfg(test)]
mod tests {
    use goose::providers::configs::ProviderModelConfig;

    use crate::test_helpers::run_profile_with_tmp_dir;

    use super::*;

    #[test]
    fn test_partial_profile_config() -> Result<()> {
        let profile = r#"
{
    "profile_items": {
        "default": {
            "provider": "databricks",
            "model": "claude-3",
            "temperature": 0.7,
            "context_limit": 50000
        }
    }
}
"#;
        run_profile_with_tmp_dir(profile, || {
            let profiles = load_profiles()?;
            let profile = profiles.get("default").unwrap();

            assert_eq!(profile.temperature, Some(0.7));
            assert_eq!(profile.context_limit, Some(50_000));
            assert_eq!(profile.max_tokens, None);
            assert_eq!(profile.estimate_factor, None);

            let provider_config = get_provider_config(&profile.provider, profile.clone());

            if let ProviderConfig::Databricks(config) = provider_config {
                assert_eq!(config.model_config().estimate_factor(), 0.8);
                assert_eq!(config.model_config().context_limit(), 50_000);
            }
            Ok(())
        })
    }
}
