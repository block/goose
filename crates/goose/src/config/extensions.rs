use super::base::Config;
use crate::agents::ExtensionConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

pub const DEFAULT_EXTENSION: &str = "developer";
pub const DEFAULT_EXTENSION_TIMEOUT: u64 = 300;
pub const DEFAULT_EXTENSION_DESCRIPTION: &str = "";
pub const DEFAULT_DISPLAY_NAME: &str = "Developer";

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct ExtensionEntry {
    pub enabled: bool,
    #[serde(flatten)]
    pub config: ExtensionConfig,
}

pub fn name_to_key(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

/// Extension configuration management
pub struct ExtensionConfigManager;

impl ExtensionConfigManager {
    /// Get the extension configuration if enabled -- uses key
    pub fn get_config(key: &str) -> Result<Option<ExtensionConfig>> {
        let config = Config::global();

        // Try to get the extension entry
        let extensions: HashMap<String, ExtensionEntry> = match config.get_param("extensions") {
            Ok(exts) => exts,
            Err(super::ConfigError::NotFound(_)) => {
                // Initialize with default developer extension
                let defaults = HashMap::from([(
                    name_to_key(DEFAULT_EXTENSION), // Use key format for top-level key in config
                    ExtensionEntry {
                        enabled: true,
                        config: ExtensionConfig::Builtin {
                            name: DEFAULT_EXTENSION.to_string(),
                            display_name: Some(DEFAULT_DISPLAY_NAME.to_string()),
                            timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                            bundled: Some(true),
                            description: Some(DEFAULT_EXTENSION_DESCRIPTION.to_string()),
                        },
                    },
                )]);
                config.set_param("extensions", serde_json::to_value(&defaults)?)?;
                defaults
            }
            Err(e) => return Err(e.into()),
        };

        Ok(extensions.get(key).and_then(|entry| {
            if entry.enabled {
                Some(entry.config.clone())
            } else {
                None
            }
        }))
    }

    pub fn get_config_by_name(name: &str) -> Result<Option<ExtensionConfig>> {
        let config = Config::global();

        // Try to get the extension entry
        let extensions: HashMap<String, ExtensionEntry> = match config.get_param("extensions") {
            Ok(exts) => exts,
            Err(super::ConfigError::NotFound(_)) => HashMap::new(),
            Err(_) => HashMap::new(),
        };

        Ok(extensions
            .values()
            .find(|entry| entry.config.name() == name)
            .map(|entry| entry.config.clone()))
    }

    /// Set or update an extension configuration
    pub fn set(entry: ExtensionEntry) -> Result<()> {
        let config = Config::global();

        let mut extensions: HashMap<String, ExtensionEntry> = config
            .get_param("extensions")
            .unwrap_or_else(|_| HashMap::new());

        let key = entry.config.key();

        extensions.insert(key, entry);
        config.set_param("extensions", serde_json::to_value(extensions)?)?;
        Ok(())
    }

    /// Remove an extension configuration -- uses the key
    pub fn remove(key: &str) -> Result<()> {
        let config = Config::global();

        let mut extensions: HashMap<String, ExtensionEntry> = config
            .get_param("extensions")
            .unwrap_or_else(|_| HashMap::new());

        extensions.remove(key);
        config.set_param("extensions", serde_json::to_value(extensions)?)?;
        Ok(())
    }

    /// Enable or disable an extension -- uses key
    pub fn set_enabled(key: &str, enabled: bool) -> Result<()> {
        let config = Config::global();

        let mut extensions: HashMap<String, ExtensionEntry> = config
            .get_param("extensions")
            .unwrap_or_else(|_| HashMap::new());

        if let Some(entry) = extensions.get_mut(key) {
            entry.enabled = enabled;
            config.set_param("extensions", serde_json::to_value(extensions)?)?;
        }
        Ok(())
    }

    /// Get all extensions and their configurations
    pub fn get_all() -> Result<Vec<ExtensionEntry>> {
        let config = Config::global();
        let extensions: HashMap<String, ExtensionEntry> = match config.get_param("extensions") {
            Ok(exts) => exts,
            Err(super::ConfigError::NotFound(_)) => HashMap::new(),
            Err(e) => return Err(e.into()),
        };
        Ok(Vec::from_iter(extensions.values().cloned()))
    }

    /// Get all extension names
    pub fn get_all_names() -> Result<Vec<String>> {
        let config = Config::global();
        Ok(config
            .get_param("extensions")
            .unwrap_or_else(|_| get_keys(Default::default())))
    }

    /// Check if an extension is enabled - FIXED to use key
    pub fn is_enabled(key: &str) -> Result<bool> {
        let config = Config::global();
        let extensions: HashMap<String, ExtensionEntry> = config
            .get_param("extensions")
            .unwrap_or_else(|_| HashMap::new());

        Ok(extensions.get(key).map(|e| e.enabled).unwrap_or(false))
    }
}

fn get_keys(entries: HashMap<String, ExtensionEntry>) -> Vec<String> {
    entries.into_keys().collect()
}
