use crate::inputs::inputs::{get_user_input, get_env_value_or_input};
use goose::providers::configs::{
    DatabricksAuth, DatabricksProviderConfig, OpenAiProviderConfig, OllamaProviderConfig, ProviderConfig
};
use goose::providers::factory::ProviderType;
use goose::providers::ollama::OLLAMA_HOST;
use goose::key_manager::{get_api_key_default, save_to_keyring, KeyRetrievalStrategy};
use strum::IntoEnumIterator;

pub const PROVIDER_OPEN_AI: &str = "openai";
pub const PROVIDER_DATABRICKS: &str = "databricks";
pub const PROVIDER_OLLAMA: &str = "ollama";

pub fn select_provider_lists() -> Vec<(&'static str, String, &'static str)> {
    ProviderType::iter()
        .map(|provider| match provider {
            ProviderType::OpenAi => (
                PROVIDER_OPEN_AI,
                PROVIDER_OPEN_AI.to_string(),
                "Recommended",
            ),
            ProviderType::Databricks => (PROVIDER_DATABRICKS, PROVIDER_DATABRICKS.to_string(), ""),
            ProviderType::Ollama => (PROVIDER_OLLAMA, PROVIDER_OLLAMA.to_string(), "")
        })
        .collect()
}

pub fn get_or_set_api_key(provider_name: &str, api_key_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Try to get existing key first from keyring or environment
    if let Ok(key) = get_api_key_default(api_key_name, KeyRetrievalStrategy::Both) {
        return Ok(key);
    }

    // If no key found or error occurred, prompt user for input
    let prompt = format!("Please enter your {} key:", provider_name);
    let api_key = get_env_value_or_input(
        api_key_name,
        &prompt,
        false,
    );
    
    // Check if user wants to save to the system keyring
    let resp = get_user_input("Would you like to save this key to the system keyring? (y/n):", "y")?;
    if resp.eq_ignore_ascii_case("y") {
        match save_to_keyring(api_key_name, &api_key) {
            Ok(_) => println!("Successfully saved key to system keyring"),
            Err(e) => {
                // Log the error but don't fail - the API key is still usable
                println!("Warning: Failed to save key to system keyring: {}", e);
            }
        }
    }
    
    Ok(api_key)
}

pub fn set_provider_config(provider_name: &str, model: String) -> ProviderConfig {
    match provider_name.to_lowercase().as_str() {
        PROVIDER_OPEN_AI => {
            let api_key = get_or_set_api_key(provider_name, "OPENAI_API_KEY")
                .expect("Failed to get OpenAI API key");
            ProviderConfig::OpenAi(OpenAiProviderConfig {
                host: "https://api.openai.com".to_string(),
                api_key,
                model,
                temperature: None,
                max_tokens: None,
            })
        },
        PROVIDER_DATABRICKS => {
            let host = get_env_value_or_input(
                "DATABRICKS_HOST",
                "Please enter your Databricks host:",
                false,
            );
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
        PROVIDER_OLLAMA => ProviderConfig::Ollama(OllamaProviderConfig {
            host: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| String::from(OLLAMA_HOST)),
            model,
            temperature: None,
            max_tokens: None,
        }),
        _ => panic!("Invalid provider name: {}", provider_name),
    }
}