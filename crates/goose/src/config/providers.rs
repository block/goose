use super::base::{Config, ConfigError};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;
use std::env;
use tracing::warn;

const PROVIDERS_CONFIG_KEY: &str = "providers";
const ACTIVE_PROVIDER_KEY: &str = "active_provider";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProviderEntry {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub configured: bool,
}

fn parse_providers_map(raw: Mapping) -> IndexMap<String, ProviderEntry> {
    let mut map = IndexMap::with_capacity(raw.len());
    for (k, v) in raw {
        match (k, serde_yaml::from_value::<ProviderEntry>(v)) {
            (serde_yaml::Value::String(key), Ok(entry)) => {
                map.insert(key, entry);
            }
            (k, v) => {
                warn!(
                    key = ?k,
                    value = ?v,
                    "Skipping malformed provider config entry"
                );
            }
        }
    }
    map
}

fn get_providers_map(config: &Config) -> IndexMap<String, ProviderEntry> {
    let raw: Mapping = config
        .get_param(PROVIDERS_CONFIG_KEY)
        .unwrap_or_else(|_| Default::default());
    parse_providers_map(raw)
}

pub fn get_provider_entry(config: &Config, name: &str) -> Option<ProviderEntry> {
    get_providers_map(config).get(name).cloned()
}

pub fn set_provider_entry(
    config: &Config,
    name: &str,
    entry: &ProviderEntry,
) -> Result<(), ConfigError> {
    let name = name.to_string();
    let entry = entry.clone();
    config.update_param::<Mapping, _, _>(PROVIDERS_CONFIG_KEY, |raw| {
        let mut map = parse_providers_map(raw);
        map.insert(name, entry);
        map
    })
}

pub fn get_active_provider(config: &Config) -> Option<String> {
    if let Ok(val) = env::var("GOOSE_PROVIDER") {
        return Some(val);
    }
    if let Ok(val) = config.get_param::<String>(ACTIVE_PROVIDER_KEY) {
        return Some(val);
    }
    config.get_param::<String>("GOOSE_PROVIDER").ok()
}

pub fn get_active_model(config: &Config) -> Option<String> {
    if let Ok(val) = env::var("GOOSE_MODEL") {
        return Some(val);
    }
    if let Some(provider_name) = get_active_provider(config) {
        if let Some(entry) = get_provider_entry(config, &provider_name) {
            if !entry.model.is_empty() {
                return Some(entry.model);
            }
        }
    }
    config.get_param::<String>("GOOSE_MODEL").ok()
}

pub fn set_active_provider(config: &Config, name: &str, model: &str) -> Result<(), ConfigError> {
    config.set_param(ACTIVE_PROVIDER_KEY, name)?;
    let entry = ProviderEntry {
        enabled: true,
        model: model.to_string(),
        configured: true,
    };
    set_provider_entry(config, name, &entry)
}

pub fn clear_active_provider(config: &Config) -> Result<(), ConfigError> {
    for key in [ACTIVE_PROVIDER_KEY, "GOOSE_PROVIDER", "GOOSE_MODEL"] {
        match config.delete(key) {
            Ok(()) | Err(ConfigError::NotFound(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn new_test_config() -> Config {
        let config_file = NamedTempFile::new().unwrap();
        let secrets_file = NamedTempFile::new().unwrap();
        Config::new_with_file_secrets(config_file.path(), secrets_file.path()).unwrap()
    }

    #[test]
    fn test_set_and_get_provider_entry() {
        let config = new_test_config();
        let entry = ProviderEntry {
            enabled: true,
            model: "gpt-4o".to_string(),
            configured: true,
        };
        set_provider_entry(&config, "openai", &entry).unwrap();

        let loaded = get_provider_entry(&config, "openai").unwrap();
        assert!(loaded.enabled);
        assert_eq!(loaded.model, "gpt-4o");
        assert!(loaded.configured);
    }

    #[test]
    fn test_set_active_provider_writes_structured_keys() {
        let config = new_test_config();
        set_active_provider(&config, "claude-acp", "current").unwrap();

        let active: String = config.get_param(ACTIVE_PROVIDER_KEY).unwrap();
        assert_eq!(active, "claude-acp");

        let entry = get_provider_entry(&config, "claude-acp").unwrap();
        assert!(entry.enabled);
        assert!(entry.configured);
        assert_eq!(entry.model, "current");
    }

    #[test]
    fn test_clear_active_provider_preserves_provider_entries() {
        let config = new_test_config();
        set_active_provider(&config, "openai", "gpt-4o").unwrap();

        clear_active_provider(&config).unwrap();

        assert!(get_active_provider(&config).is_none());
        let entry = get_provider_entry(&config, "openai").unwrap();
        assert_eq!(entry.model, "gpt-4o");
        assert!(entry.configured);
    }

    #[test]
    fn test_clear_active_provider_removes_legacy_keys() {
        let config = new_test_config();
        config.set_param("GOOSE_PROVIDER", "anthropic").unwrap();
        config.set_param("GOOSE_MODEL", "claude").unwrap();

        clear_active_provider(&config).unwrap();

        assert!(get_active_provider(&config).is_none());
        assert!(get_active_model(&config).is_none());
    }

    #[test]
    fn test_get_active_model_from_provider_entry() {
        let config = new_test_config();
        set_active_provider(&config, "openai", "gpt-4o").unwrap();

        let result = get_active_model(&config);
        assert_eq!(result, Some("gpt-4o".to_string()));
    }

    #[test]
    fn test_multiple_providers_preserved() {
        let config = new_test_config();
        set_active_provider(&config, "openai", "gpt-4o").unwrap();
        set_active_provider(&config, "anthropic", "claude-3-opus").unwrap();

        let openai = get_provider_entry(&config, "openai").unwrap();
        assert_eq!(openai.model, "gpt-4o");
        assert!(openai.configured);

        let anthropic = get_provider_entry(&config, "anthropic").unwrap();
        assert_eq!(anthropic.model, "claude-3-opus");
        assert!(anthropic.configured);

        assert_eq!(get_active_provider(&config), Some("anthropic".to_string()));
    }
}
