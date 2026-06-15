use goose::config::declarative_providers::{load_provider, LoadedProvider};
use goose::config::Config;
use goose::providers::base::{ConfigKey, ProviderMetadata, ProviderType};
use goose::providers::huggingface_auth;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyLocation {
    Environment,
    ConfigFile,
    Keychain,
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub name: String,
    pub is_set: bool,
    pub location: KeyLocation,
    pub is_secret: bool,
    pub value: Option<String>, // Only populated for non-secret keys that are set
}

/// Inspects a configuration key to determine if it's set, its location, and value (for non-secret keys)
#[allow(dead_code)]
pub fn inspect_key(key_name: &str, is_secret: bool) -> Result<KeyInfo, Box<dyn Error>> {
    let config = Config::global();

    // Check environment variable first
    let env_value = env::var(key_name).ok();

    if let Some(value) = env_value {
        return Ok(KeyInfo {
            name: key_name.to_string(),
            is_set: true,
            location: KeyLocation::Environment,
            is_secret,
            // Only include value for non-secret keys
            value: if !is_secret { Some(value) } else { None },
        });
    }

    // Check config store
    let config_result = if is_secret {
        config.get_secret(key_name).map(|v| (v, true))
    } else {
        config.get_param(key_name).map(|v| (v, false))
    };

    match config_result {
        Ok((value, is_secret_actual)) => {
            // Determine location based on whether it's a secret value
            let location = if is_secret_actual {
                KeyLocation::Keychain
            } else {
                KeyLocation::ConfigFile
            };

            Ok(KeyInfo {
                name: key_name.to_string(),
                is_set: true,
                location,
                is_secret: is_secret_actual,
                // Only include value for non-secret keys
                value: if !is_secret_actual { Some(value) } else { None },
            })
        }
        Err(_) => Ok(KeyInfo {
            name: key_name.to_string(),
            is_set: false,
            location: KeyLocation::NotFound,
            is_secret,
            value: None,
        }),
    }
}

/// Inspects multiple keys at once
#[allow(dead_code)]
pub fn inspect_keys(
    keys: &[(String, bool)], // (name, is_secret) pairs
) -> Result<Vec<KeyInfo>, Box<dyn Error>> {
    let mut results = Vec::new();

    for (key_name, is_secret) in keys {
        let info = inspect_key(key_name, *is_secret)?;
        results.push(info);
    }

    Ok(results)
}

pub fn check_provider_configured(metadata: &ProviderMetadata, provider_type: ProviderType) -> bool {
    check_provider_configured_with_huggingface_oauth(metadata, provider_type, || {
        huggingface_auth::has_usable_or_refreshable_oauth_token()
    })
}

fn check_provider_configured_with_huggingface_oauth(
    metadata: &ProviderMetadata,
    provider_type: ProviderType,
    has_usable_huggingface_oauth_token: impl Fn() -> bool,
) -> bool {
    // Special override
    if metadata.name == "local" {
        return true;
    }

    if accepts_huggingface_oauth(metadata, None, &has_usable_huggingface_oauth_token) {
        return true;
    }

    let config = Config::global();

    if provider_type == ProviderType::Custom || provider_type == ProviderType::Declarative {
        if let Ok(loaded_provider) = load_provider(metadata.name.as_str()) {
            if accepts_huggingface_oauth(
                metadata,
                Some(&loaded_provider),
                &has_usable_huggingface_oauth_token,
            ) {
                return true;
            }

            if !loaded_provider.config.requires_auth {
                return true;
            }

            if !loaded_provider.config.api_key_env.is_empty() {
                let api_key_result =
                    config.get_secret::<String>(&loaded_provider.config.api_key_env);
                if api_key_result.is_ok() {
                    return true;
                }
            }

            // Custom providers with config files are intentionally created
            return provider_type == ProviderType::Custom;
        }
    }

    // OAuth providers: trust the structured configured flag or legacy marker
    let has_oauth_key = metadata.config_keys.iter().any(|key| key.oauth_flow);
    if has_oauth_key {
        if let Some(entry) = goose::config::get_provider_entry(config, &metadata.name) {
            if entry.configured {
                return true;
            }
        }
        let configured_marker = format!("{}_configured", metadata.name);
        if matches!(config.get_param::<bool>(&configured_marker), Ok(true)) {
            return true;
        }
    }

    // Zero-config providers (no config keys): trust structured flag or active status
    if metadata.config_keys.is_empty() {
        if let Some(entry) = goose::config::get_provider_entry(config, &metadata.name) {
            if entry.configured {
                return true;
            }
        }
        let configured_marker = format!("{}_configured", metadata.name);
        if config.get_param::<bool>(&configured_marker).is_ok() {
            return true;
        }
        if let Ok(current) = config.get_goose_provider() {
            if current == metadata.name {
                return true;
            }
        }
        return false;
    }

    // Get all required keys
    let required_keys: Vec<&ConfigKey> = metadata
        .config_keys
        .iter()
        .filter(|key| key.required)
        .collect();

    // Special case: If a provider has exactly one required key and that key
    // has a default value, check if it's explicitly set
    if required_keys.len() == 1 && required_keys[0].default.is_some() {
        let key = &required_keys[0];

        // Check if the key is explicitly set (either in env or config)
        let is_set_in_env = env::var(&key.name).is_ok();
        let is_set_in_config = config.get(&key.name, key.secret).is_ok();

        return is_set_in_env || is_set_in_config;
    }

    // Special case: If a provider has only optional keys with defaults,
    // check if a configuration marker exists
    if required_keys.is_empty() && !metadata.config_keys.is_empty() {
        let all_optional_with_defaults = metadata
            .config_keys
            .iter()
            .all(|key| !key.required && key.default.is_some());

        if all_optional_with_defaults {
            // Check if the provider has been explicitly configured via the UI
            let configured_marker = format!("{}_configured", metadata.name);
            return config.get_param::<bool>(&configured_marker).is_ok();
        }
    }

    // For providers with multiple keys or keys without defaults:
    // Find required keys that don't have default values
    let required_non_default_keys: Vec<&ConfigKey> = required_keys
        .iter()
        .filter(|key| key.default.is_none())
        .cloned()
        .collect();

    // If there are no non-default keys, this provider needs at least one key explicitly set
    if required_non_default_keys.is_empty() {
        return required_keys.iter().any(|key| {
            let is_set_in_env = env::var(&key.name).is_ok();
            let is_set_in_config = config.get(&key.name, key.secret).is_ok();

            is_set_in_env || is_set_in_config
        });
    }

    // Otherwise, all non-default keys must be set
    required_non_default_keys.iter().all(|key| {
        let is_set_in_env = env::var(&key.name).is_ok();
        let is_set_in_config = config.get(&key.name, key.secret).is_ok();

        is_set_in_env || is_set_in_config
    })
}

fn accepts_huggingface_oauth(
    metadata: &ProviderMetadata,
    loaded_provider: Option<&LoadedProvider>,
    has_usable_huggingface_oauth_token: &impl Fn() -> bool,
) -> bool {
    let is_huggingface_provider = metadata.name == huggingface_auth::HUGGINGFACE_PROVIDER_NAME
        || loaded_provider.is_some_and(|provider| {
            provider.config.catalog_provider_id.as_deref()
                == Some(huggingface_auth::HUGGINGFACE_PROVIDER_NAME)
        });

    is_huggingface_provider && has_usable_huggingface_oauth_token()
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::config::declarative_providers::{DeclarativeProviderConfig, ProviderEngine};
    use goose::providers::base::ModelInfo;

    fn huggingface_metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            huggingface_auth::HUGGINGFACE_PROVIDER_NAME,
            huggingface_auth::HUGGINGFACE_DISPLAY_NAME,
            "Hugging Face provider",
            "Qwen/Qwen3-Coder-480B-A35B-Instruct",
            vec![],
            "https://huggingface.co/docs/inference-providers",
            vec![ConfigKey::new(
                huggingface_auth::HUGGINGFACE_TOKEN_SECRET_KEY,
                true,
                true,
                None,
                true,
            )],
        )
    }

    #[test]
    fn huggingface_oauth_token_counts_as_configured_without_hf_token() {
        assert!(check_provider_configured_with_huggingface_oauth(
            &huggingface_metadata(),
            ProviderType::Builtin,
            || true,
        ));
    }

    #[test]
    fn huggingface_catalog_provider_oauth_counts_as_configured() {
        let mut metadata = huggingface_metadata();
        metadata.name = "custom-huggingface".to_string();

        let loaded_provider = LoadedProvider {
            config: DeclarativeProviderConfig {
                name: metadata.name.clone(),
                engine: ProviderEngine::OpenAI,
                display_name: "Custom Hugging Face".to_string(),
                description: None,
                api_key_env: String::new(),
                base_url: "https://router.huggingface.co/v1".to_string(),
                models: vec![ModelInfo::new("test-model", 128_000)],
                headers: None,
                timeout_seconds: None,
                supports_streaming: None,
                requires_auth: true,
                catalog_provider_id: Some(huggingface_auth::HUGGINGFACE_PROVIDER_NAME.to_string()),
                base_path: None,
                env_vars: None,
                dynamic_models: None,
                skip_canonical_filtering: false,
                model_doc_link: None,
                setup_steps: vec![],
                fast_model: None,
                preserves_thinking: false,
            },
            is_editable: false,
        };

        assert!(accepts_huggingface_oauth(
            &metadata,
            Some(&loaded_provider),
            &|| true,
        ));
    }
}
