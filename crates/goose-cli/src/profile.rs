use anyhow::Result;
use goose::key_manager::{get_keyring_secret, KeyRetrievalStrategy};
use goose::providers::configs::{
    AnthropicProviderConfig, DatabricksAuth, DatabricksProviderConfig, OllamaProviderConfig,
    OpenAiProviderConfig, ProviderConfig,
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
    #[serde(default)]
    pub additional_systems: Vec<AdditionalSystem>,
}

#[derive(Serialize, Deserialize)]
pub struct Profiles {
    pub profile_items: HashMap<String, Profile>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdditionalSystem {
    pub name: String,
    pub location: String,
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

pub fn get_provider_config(provider_name: &str, model: String) -> ProviderConfig {
    match provider_name.to_lowercase().as_str() {
        "openai" => {
            // TODO error propagation throughout the CLI
            let api_key = get_keyring_secret("OPENAI_API_KEY", KeyRetrievalStrategy::Both)
                .expect("OPENAI_API_KEY not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::OpenAi(OpenAiProviderConfig {
                host: "https://api.openai.com".to_string(),
                api_key,
                model,
                temperature: None,
                max_tokens: None,
            })
        }
        "databricks" => {
            let host = get_keyring_secret("DATABRICKS_HOST", KeyRetrievalStrategy::Both)
                .expect("DATABRICKS_HOST not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Databricks(DatabricksProviderConfig {
                host: host.clone(),
                // TODO revisit configuration
                auth: DatabricksAuth::oauth(host),
                model,
                temperature: None,
                max_tokens: None,
                image_format: goose::providers::utils::ImageFormat::Anthropic,
            })
        }
        "ollama" => {
            let host = get_keyring_secret("OLLAMA_HOST", KeyRetrievalStrategy::Both)
                .expect("OLLAMA_HOST not available in env or the keychain\nSet an env var or rerun `goose configure`");
            ProviderConfig::Ollama(OllamaProviderConfig {
                host: host.clone(),
                model,
                temperature: None,
                max_tokens: None,
            })
        }
        "anthropic" => {
            let api_key = get_keyring_secret("ANTHROPIC_API_KEY", KeyRetrievalStrategy::Both)
                .expect("ANTHROPIC_API_KEY not available in env or the keychain\nSet an env var or rerun `goose configure`");

            ProviderConfig::Anthropic(AnthropicProviderConfig {
                host: "https://api.anthropic.com".to_string(), // Default Anthropic API endpoint
                api_key,
                model,
                temperature: None,
                max_tokens: None,
            })
        }
        _ => panic!("Invalid provider name"),
    }
}
