use super::CanonicalModel;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Registry for managing canonical models
#[derive(Debug, Clone)]
pub struct CanonicalModelRegistry {
    models: HashMap<String, CanonicalModel>,
}

impl CanonicalModelRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Load registry from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read canonical models file")?;

        let models: Vec<CanonicalModel> = serde_json::from_str(&content)
            .context("Failed to parse canonical models JSON")?;

        let mut registry = Self::new();
        for model in models {
            registry.register(model);
        }

        Ok(registry)
    }

    /// Save registry to a JSON file
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut models: Vec<&CanonicalModel> = self.models.values().collect();
        models.sort_by(|a, b| a.name.cmp(&b.name));

        let json = serde_json::to_string_pretty(&models)
            .context("Failed to serialize canonical models")?;

        std::fs::write(path.as_ref(), json)
            .context("Failed to write canonical models file")?;

        Ok(())
    }

    /// Register a canonical model
    pub fn register(&mut self, model: CanonicalModel) {
        self.models.insert(model.name.clone(), model);
    }

    /// Look up a canonical model by name
    pub fn get(&self, name: &str) -> Option<&CanonicalModel> {
        self.models.get(name)
    }

    /// Get all canonical models
    pub fn all_models(&self) -> Vec<&CanonicalModel> {
        self.models.values().collect()
    }

    /// Get number of registered models
    pub fn count(&self) -> usize {
        self.models.len()
    }

    /// Check if a model exists
    pub fn contains(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }
}

impl Default for CanonicalModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::canonical::ModelType;

    #[test]
    fn test_registry_operations() {
        let mut registry = CanonicalModelRegistry::new();

        let model = CanonicalModel::new("test-model", ModelType::Chat, 8192);
        registry.register(model);

        assert_eq!(registry.count(), 1);
        assert!(registry.contains("test-model"));
        assert!(registry.get("test-model").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_serialization() {
        let mut registry = CanonicalModelRegistry::new();

        let model1 = CanonicalModel::new("model-1", ModelType::Chat, 8192)
            .with_streaming(true);
        let model2 = CanonicalModel::new("model-2", ModelType::Voice, 4096);

        registry.register(model1);
        registry.register(model2);

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_registry.json");

        // Save and load
        registry.to_file(&file_path).unwrap();
        let loaded = CanonicalModelRegistry::from_file(&file_path).unwrap();

        assert_eq!(loaded.count(), 2);
        assert!(loaded.contains("model-1"));
        assert!(loaded.contains("model-2"));

        // Cleanup
        std::fs::remove_file(file_path).ok();
    }
}
