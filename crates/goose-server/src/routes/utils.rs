use serde::{Deserialize, Serialize};
use std::error::Error;
use goose::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyLocation {
    Environment,
    ConfigFile,
    Keychain,
    NotFound
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
pub fn inspect_key(
    key_name: &str,
    is_secret: bool,
) -> Result<KeyInfo, Box<dyn Error>> {
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
        config.get(key_name).map(|v| (v, false))
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
        },
        Err(_) => {
            Ok(KeyInfo {
                name: key_name.to_string(),
                is_set: false,
                location: KeyLocation::NotFound,
                is_secret,
                value: None,
            })
        }
    }
}

/// Inspects multiple keys at once
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