use super::base::Config;
use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::warn;
use utoipa::ToSchema;

pub const DEFAULT_EXTENSION: &str = "developer";
pub const DEFAULT_EXTENSION_TIMEOUT: u64 = 300;
pub const DEFAULT_EXTENSION_DESCRIPTION: &str = "";
pub const DEFAULT_DISPLAY_NAME: &str = "Developer";
const EXTENSIONS_CONFIG_KEY: &str = "extensions";

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

pub struct ExtensionConfigManager;

impl ExtensionConfigManager {
    fn get_extensions_map() -> Result<HashMap<String, ExtensionEntry>> {
        let raw: Value = Config::global()
            .get_param::<Value>(EXTENSIONS_CONFIG_KEY)
            .unwrap_or_else(|err| {
                warn!(
                    "Failed to load {}: {err}. Falling back to empty object.",
                    EXTENSIONS_CONFIG_KEY
                );
                Value::Object(serde_json::Map::new())
            });

        let mut extensions_map: HashMap<String, ExtensionEntry> = match raw {
            Value::Object(obj) => {
                let mut m = HashMap::with_capacity(obj.len());
                for (k, mut v) in obj {
                    if let Value::Object(ref mut inner) = v {
                        match inner.get("description") {
                            Some(Value::Null) | None => {
                                inner.insert(
                                    "description".to_string(),
                                    Value::String(String::new()),
                                );
                            }
                            _ => {}
                        }
                    }
                    match serde_json::from_value::<ExtensionEntry>(v.clone()) {
                        Ok(entry) => {
                            m.insert(k, entry);
                        }
                        Err(err) => {
                            let bad_json = serde_json::to_string(&v).unwrap_or_else(|e| {
                                format!("<failed to serialize malformed value: {e}>")
                            });
                            warn!(
                                extension = %k,
                                error = %err,
                                bad_json = %bad_json,
                                "Skipping malformed extension"
                            );
                        }
                    }
                }
                m
            }
            other => {
                warn!(
                    "Expected object for {}, got {}. Using empty map.",
                    EXTENSIONS_CONFIG_KEY, other
                );
                HashMap::new()
            }
        };

        if !extensions_map.is_empty() {
            for (name, def) in PLATFORM_EXTENSIONS.iter() {
                if !extensions_map.contains_key(*name) {
                    extensions_map.insert(
                        name.to_string(),
                        ExtensionEntry {
                            config: ExtensionConfig::Platform {
                                name: def.name.to_string(),
                                description: def.description.to_string(),
                                bundled: Some(true),
                                available_tools: Vec::new(),
                            },
                            enabled: true,
                        },
                    );
                }
            }
        }
        Ok(extensions_map)
    }

    fn save_extensions_map(extensions: HashMap<String, ExtensionEntry>) -> Result<()> {
        let config = Config::global();
        config.set_param(EXTENSIONS_CONFIG_KEY, serde_json::to_value(extensions)?)?;
        Ok(())
    }

    pub fn get_config_by_name(name: &str) -> Result<Option<ExtensionConfig>> {
        let extensions = Self::get_extensions_map()?;
        Ok(extensions
            .values()
            .find(|entry| entry.config.name() == name)
            .map(|entry| entry.config.clone()))
    }

    pub fn set(entry: ExtensionEntry) -> Result<()> {
        let mut extensions = Self::get_extensions_map()?;
        let key = entry.config.key();
        extensions.insert(key, entry);
        Self::save_extensions_map(extensions)
    }

    pub fn remove(key: &str) -> Result<()> {
        let mut extensions = Self::get_extensions_map()?;
        extensions.remove(key);
        Self::save_extensions_map(extensions)
    }

    pub fn set_enabled(key: &str, enabled: bool) -> Result<()> {
        let mut extensions = Self::get_extensions_map()?;
        if let Some(entry) = extensions.get_mut(key) {
            entry.enabled = enabled;
            Self::save_extensions_map(extensions)?;
        }
        Ok(())
    }

    pub fn get_all() -> Result<Vec<ExtensionEntry>> {
        let extensions = Self::get_extensions_map()?;
        Ok(extensions.into_values().collect())
    }

    pub fn get_all_names() -> Result<Vec<String>> {
        let extensions = Self::get_extensions_map()?;
        Ok(extensions.keys().cloned().collect())
    }

    pub fn is_enabled(key: &str) -> Result<bool> {
        let extensions = Self::get_extensions_map()?;
        Ok(extensions.get(key).map(|e| e.enabled).unwrap_or(false))
    }
}
