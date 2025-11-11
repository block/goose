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
        models.sort_by(|a, b| a.canonical_slug.cmp(&b.canonical_slug));

        let json = serde_json::to_string_pretty(&models)
            .context("Failed to serialize canonical models")?;

        std::fs::write(path.as_ref(), json)
            .context("Failed to write canonical models file")?;

        Ok(())
    }

    /// Register a canonical model
    pub fn register(&mut self, model: CanonicalModel) {
        self.models.insert(model.canonical_slug.clone(), model);
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

    #[test]
    fn test_registry_operations() {
        let registry = CanonicalModelRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(!registry.contains("test-model"));
        assert!(registry.get("nonexistent").is_none());
    }
}
