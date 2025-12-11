use super::CanonicalModel;
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;

/// Cached bundled canonical model registry
static BUNDLED_REGISTRY: Lazy<Result<CanonicalModelRegistry>> = Lazy::new(|| {
    const CANONICAL_MODELS_JSON: &str = include_str!("data/canonical_models.json");

    let models: Vec<CanonicalModel> = serde_json::from_str(CANONICAL_MODELS_JSON)
        .context("Failed to parse bundled canonical models JSON")?;

    let mut registry = CanonicalModelRegistry::new();
    for model in models {
        registry.register(model);
    }

    Ok(registry)
});

#[derive(Debug, Clone)]
pub struct CanonicalModelRegistry {
    models: HashMap<String, CanonicalModel>,
}

impl CanonicalModelRegistry {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    pub fn bundled() -> Result<&'static Self> {
        BUNDLED_REGISTRY
            .as_ref()
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read canonical models file")?;

        let models: Vec<CanonicalModel> =
            serde_json::from_str(&content).context("Failed to parse canonical models JSON")?;

        let mut registry = Self::new();
        for model in models {
            registry.register(model);
        }

        Ok(registry)
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut models: Vec<&CanonicalModel> = self.models.values().collect();
        models.sort_by(|a, b| a.id.cmp(&b.id));

        let json = serde_json::to_string_pretty(&models)
            .context("Failed to serialize canonical models")?;

        std::fs::write(path.as_ref(), json).context("Failed to write canonical models file")?;

        Ok(())
    }

    pub fn register(&mut self, model: CanonicalModel) {
        self.models.insert(model.id.clone(), model);
    }

    pub fn get(&self, name: &str) -> Option<&CanonicalModel> {
        self.models.get(name)
    }

    pub fn all_models(&self) -> Vec<&CanonicalModel> {
        self.models.values().collect()
    }

    pub fn count(&self) -> usize {
        self.models.len()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }
}

impl Default for CanonicalModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
