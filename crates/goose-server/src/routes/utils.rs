use crate::state::AppState;
use goose::config::Config;
use goose::providers::base::{ConfigKey, ProviderMetadata};
use http::{HeaderMap, StatusCode};
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

pub fn verify_secret_key(headers: &HeaderMap, state: &AppState) -> Result<StatusCode, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        Err(StatusCode::UNAUTHORIZED)
    } else {
        Ok(StatusCode::OK)
    }
}

/// Inspects a configuration key to determine if it's set, its location, and value (for non-secret keys)
#[allow(dead_code)]
pub fn inspect_key(key_name: &str, is_secret: bool) -> Result<KeyInfo, Box<dyn Error>> {
    let config = Config::global();

    // Check environment variable first
    let env_value = std::env::var(key_name).ok();

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

pub fn check_provider_configured(metadata: &ProviderMetadata) -> bool {
    let config = Config::global();

    // Special case: Zero-config providers (no config keys)
    if metadata.config_keys.is_empty() {
        // Check if the provider has been explicitly configured via the UI
        let configured_marker = format!("{}_configured", metadata.name);
        return config.get_param::<bool>(&configured_marker).is_ok();
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
