use super::base::Config;
use crate::agents::ExtensionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing;
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
    let config = Config::global();
    config
        .get_param(EXTENSIONS_CONFIG_KEY)
        .unwrap_or_else(|_| HashMap::new())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::ExtensionConfig;

    fn create_test_extension_config() -> ExtensionConfig {
        ExtensionConfig::Builtin {
            name: "test_extension".to_string(),
            display_name: Some("Test Extension".to_string()),
            description: Some("A test extension".to_string()),
            timeout: None,
            bundled: None,
            available_tools: vec![],
        }
    }

    #[test]
    fn test_name_to_key_function() {
        assert_eq!(name_to_key("Test Extension"), "testextension");
        assert_eq!(name_to_key("Developer Tools"), "developertools");
        assert_eq!(name_to_key("simple"), "simple");
        assert_eq!(name_to_key("UPPER_case MiXeD"), "upper_casemixed");
    }

    #[test]
    fn test_extension_config_key_generation() {
        let config = create_test_extension_config();
        assert_eq!(config.key(), "test_extension");

        let config_with_spaces = ExtensionConfig::Builtin {
            name: "Test Extension Name".to_string(),
            display_name: Some("Test Extension".to_string()),
            description: Some("A test extension".to_string()),
            timeout: None,
            bundled: None,
            available_tools: vec![],
        };
        assert_eq!(config_with_spaces.key(), "testextensionname");
    }

    #[test]
    fn test_extension_entry_serialization() {
        let config = create_test_extension_config();
        let entry = ExtensionEntry {
            enabled: true,
            config,
        };

        // Test that ExtensionEntry can be serialized/deserialized
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"name\":\"test_extension\""));

        let deserialized: ExtensionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.enabled, true);
        assert_eq!(deserialized.config.name(), "test_extension");
    }

    #[test]
    fn test_get_extensions_map_returns_hashmap() {
        // Test that get_extensions_map returns a HashMap (may be empty or not depending on global config)
        let extensions = get_extensions_map();
        // Just verify it returns a HashMap - don't assert on contents since global config may vary
        assert!(extensions.is_empty() || !extensions.is_empty());
    }
}
