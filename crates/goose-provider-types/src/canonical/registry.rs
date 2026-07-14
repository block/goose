use super::dynamic;
use super::CanonicalModel;
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// Cached bundled canonical model registry
static BUNDLED_REGISTRY: Lazy<Result<CanonicalModelRegistry>> = Lazy::new(|| {
    const CANONICAL_MODELS_JSON: &str = include_str!("data/canonical_models.json");

    let models: Vec<CanonicalModel> = serde_json::from_str(CANONICAL_MODELS_JSON)
        .context("Failed to parse bundled canonical models JSON")?;

    let mut registry = CanonicalModelRegistry::new();
    for model in models {
        // Extract provider and model from id (format: "provider/model")
        if let Some((provider, model_name)) = model.id.split_once('/') {
            let provider = provider.to_string();
            let model_name = model_name.to_string();
            registry.register(&provider, &model_name, model);
        }
    }

    Ok(registry)
});

/// The registry goose actually uses at runtime.
///
/// Initialized from the bundled data, then overlaid with any on-disk dynamic
/// cache. When dynamic refresh is enabled and the cache is stale (or absent), a
/// one-shot background refresh is scheduled that hot-swaps the newer data in.
static ACTIVE_REGISTRY: Lazy<RwLock<Arc<CanonicalModelRegistry>>> = Lazy::new(|| {
    let mut registry = BUNDLED_REGISTRY
        .as_ref()
        .map(|r| r.clone())
        .unwrap_or_default();

    let mut stale = true;
    if dynamic::is_enabled() {
        if let Some((cached, cache_stale)) = dynamic::load_cached() {
            registry.merge_from(&cached);
            stale = cache_stale;
        }
    }

    if dynamic::is_enabled() && stale {
        schedule_refresh();
    }

    RwLock::new(Arc::new(registry))
});

static REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Schedule a one-shot background refresh (deduplicated) that fetches the latest
/// inventory, writes the cache, and swaps the fresh data into [`ACTIVE_REGISTRY`].
///
/// Requires a Tokio runtime; if none is running the refresh is skipped and the
/// next access will retry.
fn schedule_refresh() {
    if REFRESH_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        REFRESH_IN_FLIGHT.store(false, Ordering::SeqCst);
        return;
    };

    handle.spawn(async {
        match dynamic::refresh().await {
            Ok(fresh) => {
                let mut merged = BUNDLED_REGISTRY
                    .as_ref()
                    .map(|r| r.clone())
                    .unwrap_or_default();
                merged.merge_from(&fresh);
                if let Ok(mut active) = ACTIVE_REGISTRY.write() {
                    *active = Arc::new(merged);
                }
                tracing::info!("Refreshed canonical model registry from dynamic source");
            }
            Err(e) => tracing::warn!("Canonical model dynamic refresh failed: {e}"),
        }
        REFRESH_IN_FLIGHT.store(false, Ordering::SeqCst);
    });
}

#[derive(Debug, Clone)]
pub struct CanonicalModelRegistry {
    // Key: (provider, model) tuple
    models: HashMap<(String, String), CanonicalModel>,
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

    /// The registry goose should use at runtime.
    ///
    /// This is the bundled registry unless dynamic refresh is enabled, in which
    /// case it is overlaid with the on-disk dynamic cache (and refreshed in the
    /// background when stale). Callers get a cheap `Arc` snapshot; a concurrent
    /// refresh swapping in newer data does not affect an already-held snapshot.
    pub fn active() -> Arc<Self> {
        ACTIVE_REGISTRY
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Overlay every model from `other` onto this registry, replacing existing
    /// entries with the same `(provider, model)` key.
    pub fn merge_from(&mut self, other: &Self) {
        for (key, model) in &other.models {
            self.models.insert(key.clone(), model.clone());
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read canonical models file")?;

        let models: Vec<CanonicalModel> =
            serde_json::from_str(&content).context("Failed to parse canonical models JSON")?;

        let mut registry = Self::new();
        for model in models {
            if let Some((provider, model_name)) = model.id.split_once('/') {
                let provider = provider.to_string();
                let model_name = model_name.to_string();
                registry.register(&provider, &model_name, model);
            }
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

    pub fn register(&mut self, provider: &str, model: &str, canonical_model: CanonicalModel) {
        self.models
            .insert((provider.to_string(), model.to_string()), canonical_model);
    }

    pub fn get(&self, provider: &str, model: &str) -> Option<&CanonicalModel> {
        self.models.get(&(provider.to_string(), model.to_string()))
    }

    pub fn get_all_models_for_provider(&self, provider: &str) -> Vec<CanonicalModel> {
        self.models
            .iter()
            .filter(|((p, _), _)| p == provider)
            .map(|(_, model)| model.clone())
            .collect()
    }

    pub fn all_models(&self) -> Vec<&CanonicalModel> {
        self.models.values().collect()
    }

    pub fn count(&self) -> usize {
        self.models.len()
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
    use crate::canonical::{Limit, Modalities, Pricing};

    fn model(id: &str, reasoning: bool) -> CanonicalModel {
        CanonicalModel {
            id: id.to_string(),
            name: id.to_string(),
            family: None,
            attachment: None,
            reasoning: Some(reasoning),
            thinking_mode: None,
            tool_call: true,
            temperature: None,
            knowledge: None,
            release_date: None,
            last_updated: None,
            modalities: Modalities::default(),
            open_weights: None,
            cost: Pricing::default(),
            limit: Limit::default(),
        }
    }

    #[test]
    fn merge_from_overlays_and_adds() {
        let mut base = CanonicalModelRegistry::new();
        base.register("x-ai", "grok-4", model("x-ai/grok-4", false));

        let mut overlay = CanonicalModelRegistry::new();
        // Replaces the existing entry with new capability data.
        overlay.register("x-ai", "grok-4", model("x-ai/grok-4", true));
        // Adds a brand new model absent from the base.
        overlay.register("x-ai", "grok-4.5", model("x-ai/grok-4.5", true));

        base.merge_from(&overlay);

        assert_eq!(base.count(), 2);
        assert_eq!(base.get("x-ai", "grok-4").unwrap().reasoning, Some(true));
        assert!(base.get("x-ai", "grok-4.5").is_some());
    }
}
