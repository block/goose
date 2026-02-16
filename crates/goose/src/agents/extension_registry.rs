//! Extension Registry — server-level shared singleton for MCP extension storage.
//!
//! `ExtensionRegistry` owns the `HashMap<String, Extension>` that was previously
//! embedded in `ExtensionManager`. By extracting it, multiple agents can share
//! the same set of MCP connections without duplicating them.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────┐
//! │   Server (AppState)         │
//! │   └─ ExtensionRegistry      │  ← ONE shared instance
//! │       └─ extensions: Mutex  │
//! ├─────────────────────────────┤
//! │   Agent A                   │
//! │   └─ ExtensionManager       │──borrows──► Registry
//! │       └─ ToolRegistry       │
//! ├─────────────────────────────┤
//! │   Agent B                   │
//! │   └─ ExtensionManager       │──borrows──► Registry (same!)
//! │       └─ ToolRegistry       │
//! └─────────────────────────────┘
//! ```

use std::collections::HashMap;
use tokio::sync::Mutex;

use super::extension::ExtensionConfig;
use super::extension_manager::Extension;
use crate::config::extensions::name_to_key;

/// Server-level shared registry for MCP extensions.
///
/// Owns the extension storage (`HashMap<String, Extension>`) and provides
/// thread-safe lifecycle operations (insert, remove, list, query).
/// Multiple `ExtensionManager` instances can share a single registry via `Arc`.
pub struct ExtensionRegistry {
    extensions: Mutex<HashMap<String, Extension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: Mutex::new(HashMap::new()),
        }
    }

    /// Get a lock on the extensions map.
    /// Primary access point for ExtensionManager to read/write extensions.
    pub(crate) fn extensions(&self) -> &Mutex<HashMap<String, Extension>> {
        &self.extensions
    }

    /// Insert a pre-built extension into the registry.
    /// Used when ExtensionManager delegates lifecycle to registry (Phase C).
    #[allow(dead_code)]
    pub(crate) async fn insert(&self, name: String, extension: Extension) {
        self.extensions.lock().await.insert(name, extension);
    }

    /// Remove an extension by name. Returns the removed extension if it existed.
    /// Used when ExtensionManager delegates lifecycle to registry (Phase C).
    #[allow(dead_code)]
    pub(crate) async fn remove(&self, name: &str) -> Option<Extension> {
        self.extensions.lock().await.remove(name)
    }

    /// Disconnect (remove) an extension by name. Returns true if it existed.
    /// This is the public API — avoids exposing the private Extension type.
    pub async fn disconnect(&self, name: &str) -> bool {
        self.extensions.lock().await.remove(name).is_some()
    }

    /// Check if an extension exists by normalized name.
    pub async fn contains(&self, name: &str) -> bool {
        self.extensions.lock().await.contains_key(name)
    }

    /// List all extension names.
    pub async fn list_names(&self) -> Vec<String> {
        self.extensions.lock().await.keys().cloned().collect()
    }

    /// Get extension count.
    pub async fn len(&self) -> usize {
        self.extensions.lock().await.len()
    }

    /// Check if empty.
    pub async fn is_empty(&self) -> bool {
        self.extensions.lock().await.is_empty()
    }

    /// Check if an extension is enabled (exists in registry), with name normalization.
    pub async fn is_extension_enabled(&self, name: &str) -> bool {
        let normalized = name_to_key(name);
        self.contains(&normalized).await
    }

    /// Get all extension configs.
    pub async fn get_extension_configs(&self) -> Vec<ExtensionConfig> {
        self.extensions
            .lock()
            .await
            .values()
            .map(|ext| ext.config.clone())
            .collect()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_registry_is_empty() {
        let registry = ExtensionRegistry::new();
        assert!(registry.is_empty().await);
        assert_eq!(registry.len().await, 0);
        assert!(registry.list_names().await.is_empty());
    }

    #[tokio::test]
    async fn test_contains_returns_false_for_missing() {
        let registry = ExtensionRegistry::new();
        assert!(!registry.contains("test").await);
    }

    #[tokio::test]
    async fn test_is_extension_enabled_normalizes_name() {
        let registry = ExtensionRegistry::new();
        assert!(!registry.is_extension_enabled("Some Extension").await);
    }

    #[tokio::test]
    async fn test_default_impl() {
        let registry = ExtensionRegistry::default();
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_remove_on_empty_returns_none() {
        let registry = ExtensionRegistry::new();
        // remove returns Option<Extension> — can't construct Extension in tests,
        // but we can verify the registry remains empty after remove
        let names_before = registry.list_names().await;
        assert!(names_before.is_empty());
        // After remove of nonexistent key, still empty
        let _ = registry.remove("nonexistent").await;
        assert!(registry.is_empty().await);
    }
}
