//! Extension Catalog - Global registry of available extensions
//!
//! The catalog is loaded from a config file and shared across all sessions.
//! It can be reloaded at runtime to pick up new extensions without restarting.

use std::collections::HashMap;
use std::path::Path;

use tracing::info;

use super::config::{ExtensionConfig, ExtensionKind, ExtensionsConfig};
use crate::{Error, Result};

/// Global catalog of available extensions.
///
/// This is shared across all sessions via `Arc<RwLock<ExtensionCatalog>>`.
/// Sessions read from this to know what extensions can be enabled.
pub struct ExtensionCatalog {
    /// All available extension configs, keyed by name
    configs: HashMap<String, ExtensionConfig>,
}

impl ExtensionCatalog {
    /// Create an empty catalog
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// Load catalog from a TOML config file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!("Failed to read config file '{}': {}", path.display(), e))
        })?;

        Self::from_toml(&content)
    }

    /// Parse catalog from TOML string
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: ExtensionsConfig = toml::from_str(content).map_err(|e| {
            Error::Config(format!("Failed to parse extensions config: {}", e))
        })?;

        let mut catalog = Self::new();

        for (name, entry) in config.extensions {
            let ext_config = entry.into_config(name.clone());
            info!(
                extension = %name,
                kind = ?ext_config.kind,
                "Loaded extension config"
            );
            catalog.configs.insert(name, ext_config);
        }

        // Always register built-in native extensions
        catalog.register_builtins();

        info!(count = catalog.configs.len(), "Extension catalog loaded");
        Ok(catalog)
    }

    /// Reload catalog from a config file
    ///
    /// This preserves any built-in extensions and merges with file config.
    pub fn reload(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!("Failed to read config file '{}': {}", path.display(), e))
        })?;

        let config: ExtensionsConfig = toml::from_str(&content).map_err(|e| {
            Error::Config(format!("Failed to parse extensions config: {}", e))
        })?;

        // Clear non-builtin extensions and reload
        self.configs.retain(|_, c| matches!(c.kind, ExtensionKind::Native));

        for (name, entry) in config.extensions {
            let ext_config = entry.into_config(name.clone());
            self.configs.insert(name, ext_config);
        }

        self.register_builtins();

        info!(count = self.configs.len(), "Extension catalog reloaded");
        Ok(())
    }

    /// Register built-in native extensions
    fn register_builtins(&mut self) {
        // These are always available, even without a config file
        // For now, just a placeholder - we'll add DevelopExtension etc. later
        
        // Example:
        // self.configs.entry("develop".to_string()).or_insert(ExtensionConfig {
        //     name: "develop".to_string(),
        //     description: "Shell, file editing, and codebase exploration tools".to_string(),
        //     kind: ExtensionKind::Native,
        // });
    }

    /// Get config for an extension by name
    pub fn get(&self, name: &str) -> Option<&ExtensionConfig> {
        self.configs.get(name)
    }

    /// Check if an extension exists in the catalog
    pub fn contains(&self, name: &str) -> bool {
        self.configs.contains_key(name)
    }

    /// List all available extensions
    pub fn list(&self) -> impl Iterator<Item = &ExtensionConfig> {
        self.configs.values()
    }

    /// Get all extension names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.configs.keys().map(|s| s.as_str())
    }

    /// Number of available extensions
    pub fn len(&self) -> usize {
        self.configs.len()
    }

    /// Check if catalog is empty
    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }
}

impl Default for ExtensionCatalog {
    fn default() -> Self {
        let mut catalog = Self::new();
        catalog.register_builtins();
        catalog
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_catalog() {
        let catalog = ExtensionCatalog::new();
        assert!(catalog.is_empty());
    }

    #[test]
    fn test_load_from_toml() {
        let toml = r#"
[extensions.browser]
kind = "mcp"
command = "npx"
args = ["-y", "@anthropic/mcp-browser"]
description = "Web browsing tools"

[extensions.memory]
kind = "mcp"
command = "uvx"
args = ["mcp-memory"]
description = "Persistent memory"
"#;

        let catalog = ExtensionCatalog::from_toml(toml).unwrap();
        assert!(catalog.contains("browser"));
        assert!(catalog.contains("memory"));
        assert!(!catalog.contains("nonexistent"));
    }

    #[test]
    fn test_list_extensions() {
        let toml = r#"
[extensions.a]
kind = "native"
description = "Extension A"

[extensions.b]
kind = "native"
description = "Extension B"
"#;

        let catalog = ExtensionCatalog::from_toml(toml).unwrap();
        let names: Vec<_> = catalog.names().collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
    }
}
