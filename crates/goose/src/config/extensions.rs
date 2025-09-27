use super::base::Config;
use crate::agents::ExtensionConfig;
use crate::goose_apps;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        let mut extensions_map = Config::global()
            .get_param(EXTENSIONS_CONFIG_KEY)
            .unwrap_or_else(|_| HashMap::new());

        // undo this hack when we have the infra (Douwe):
        if !extensions_map.contains_key(goose_apps::mcp_server::EXTENSION_NAME) {
            extensions_map.insert(
                goose_apps::mcp_server::EXTENSION_NAME.to_string(),
                ExtensionEntry {
                    config: ExtensionConfig::Builtin {
                        name: goose_apps::mcp_server::EXTENSION_NAME.to_string(),
                        display_name: Some("Goose Apps".to_string()),
                        description: Some(
                            "Create and edit goose apps through the goose chat interface and share with your friends".to_string()),
                        timeout: Some(300),
                        bundled: Some(true),
                        available_tools: Vec::new(),
                    },
                    enabled: false,
                }
            );
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
