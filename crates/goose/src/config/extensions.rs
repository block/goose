use super::base::Config;
use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
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

fn get_extensions_map() -> HashMap<String, ExtensionEntry> {
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
                            inner.insert("description".to_string(), Value::String(String::new()));
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
                            display_name: Some(def.display_name.to_string()),
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
    extensions_map
}

fn save_extensions_map(extensions: HashMap<String, ExtensionEntry>) {
    let config = Config::global();
    match serde_json::to_value(extensions) {
        Ok(value) => {
            if let Err(e) = config.set_param(EXTENSIONS_CONFIG_KEY, value) {
                tracing::debug!("Failed to save extensions config: {}", e);
            }
        }
        Err(e) => {
            tracing::debug!("Failed to serialize extensions: {}", e);
        }
    }
}

pub fn get_extension_by_name(name: &str) -> Option<ExtensionConfig> {
    let extensions = get_extensions_map();
    extensions
        .values()
        .find(|entry| entry.config.name() == name)
        .map(|entry| entry.config.clone())
}

pub fn set_extension(entry: ExtensionEntry) {
    let mut extensions = get_extensions_map();
    let key = entry.config.key();
    extensions.insert(key, entry);
    save_extensions_map(extensions);
}

pub fn remove_extension(key: &str) {
    let mut extensions = get_extensions_map();
    extensions.remove(key);
    save_extensions_map(extensions);
}

pub fn set_extension_enabled(key: &str, enabled: bool) {
    let mut extensions = get_extensions_map();
    if let Some(entry) = extensions.get_mut(key) {
        entry.enabled = enabled;
        save_extensions_map(extensions);
    }
}

pub fn get_all_extensions() -> Vec<ExtensionEntry> {
    let extensions = get_extensions_map();
    extensions.into_values().collect()
}

pub fn get_all_extension_names() -> Vec<String> {
    let extensions = get_extensions_map();
    extensions.keys().cloned().collect()
}

pub fn is_extension_enabled(key: &str) -> bool {
    let extensions = get_extensions_map();
    extensions.get(key).map(|e| e.enabled).unwrap_or(false)
}

pub fn get_enabled_extensions() -> Vec<ExtensionConfig> {
    get_all_extensions()
        .into_iter()
        .filter(|ext| ext.enabled)
        .map(|ext| ext.config)
        .collect()
}
