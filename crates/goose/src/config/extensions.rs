use super::base::Config;
use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;
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

/// Default bundled extensions (builtin MCP servers) that should be available on fresh installs.
/// These are distinct from PLATFORM_EXTENSIONS which run in-process.
pub static BUNDLED_EXTENSIONS: Lazy<Vec<ExtensionEntry>> = Lazy::new(|| {
    vec![
        ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "developer".to_string(),
                display_name: Some("Developer".to_string()),
                description: "General development tools useful for software engineering."
                    .to_string(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: true,
        },
        ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "computercontroller".to_string(),
                display_name: Some("Computer Controller".to_string()),
                description:
                    "General computer control tools that don't require you to be a developer or engineer."
                        .to_string(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        },
        ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "autovisualiser".to_string(),
                display_name: Some("Auto Visualiser".to_string()),
                description: "Data visualization and UI generation tools".to_string(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        },
        ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "memory".to_string(),
                display_name: Some("Memory".to_string()),
                description: "Teach goose your preferences as you go.".to_string(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        },
        ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "tutorial".to_string(),
                display_name: Some("Tutorial".to_string()),
                description: "Access interactive tutorials and guides".to_string(),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        },
    ]
});

pub fn name_to_key(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn get_extensions_map() -> IndexMap<String, ExtensionEntry> {
    let raw: Mapping = Config::global()
        .get_param(EXTENSIONS_CONFIG_KEY)
        .unwrap_or_else(|err| {
            warn!(
                "Failed to load {}: {err}. Falling back to empty object.",
                EXTENSIONS_CONFIG_KEY
            );
            Default::default()
        });

    let mut extensions_map = IndexMap::with_capacity(raw.len());
    for (k, v) in raw {
        match (k, serde_yaml::from_value::<ExtensionEntry>(v)) {
            (serde_yaml::Value::String(key), Ok(entry)) => {
                extensions_map.insert(key, entry);
            }
            (k, v) => {
                warn!(
                    key = ?k,
                    value = ?v,
                    "Skipping malformed extension config entry"
                );
            }
        }
    }

    // Always add platform extensions if they aren't present (including fresh installs)
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
                    enabled: def.default_enabled,
                },
            );
        }
    }

    // Always add bundled extensions if they aren't present (including fresh installs)
    for entry in BUNDLED_EXTENSIONS.iter() {
        let key = entry.config.key();
        if !extensions_map.contains_key(&key) {
            extensions_map.insert(key, entry.clone());
        }
    }

    extensions_map
}

fn save_extensions_map(extensions: IndexMap<String, ExtensionEntry>) {
    let config = Config::global();
    if let Err(e) = config.set_param(EXTENSIONS_CONFIG_KEY, &extensions) {
        // TODO(jack) why is this just a debug statement?
        tracing::debug!("Failed to save extensions config: {}", e);
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
    extensions.shift_remove(key);
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

pub fn get_warnings() -> Vec<String> {
    let raw: Mapping = Config::global()
        .get_param(EXTENSIONS_CONFIG_KEY)
        .unwrap_or_default();

    let mut warnings = Vec::new();
    for (k, v) in raw {
        if let (serde_yaml::Value::String(key), Ok(entry)) =
            (k, serde_yaml::from_value::<ExtensionEntry>(v))
        {
            if matches!(entry.config, ExtensionConfig::Sse { .. }) {
                warnings.push(format!(
                    "'{}': SSE is unsupported, migrate to streamable_http",
                    key
                ));
            }
        }
    }
    warnings
}
