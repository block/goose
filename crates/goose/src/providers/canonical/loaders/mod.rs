pub mod nanogpt;

use super::CanonicalModel;
use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Trait for providers that supply their own up-to-date model listings.
///
/// Providers can implement this to fetch model metadata from their own API,
/// overriding the bundled canonical models with fresher data.
#[async_trait]
pub trait CanonicalModelLoader: Send + Sync {
    /// The canonical provider name (as used in the registry, e.g. "nano-gpt")
    fn provider_name(&self) -> &str;

    /// Fetch models from the provider's API and return as CanonicalModels.
    /// Each model's `id` should be in the format "provider/model-name".
    async fn load_models(&self) -> Result<Vec<CanonicalModel>>;
}

/// Runtime storage for provider-loaded models that override bundled ones.
/// Outer key: provider name, Inner key: model name (without provider prefix)
static PROVIDER_OVERRIDES: Lazy<RwLock<HashMap<String, HashMap<String, CanonicalModel>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Get the loader for a given provider, if one exists.
pub fn get_loader(provider: &str) -> Option<Box<dyn CanonicalModelLoader>> {
    match provider {
        "nano-gpt" => Some(Box::new(nanogpt::NanoGptLoader)),
        _ => None,
    }
}

/// Load models from a provider's API and store them as runtime overrides.
/// These override the bundled canonical models for that provider.
/// Returns the number of models loaded, or 0 if no loader exists.
pub async fn load_provider_models(provider: &str) -> Result<usize> {
    let loader = match get_loader(provider) {
        Some(l) => l,
        None => return Ok(0),
    };

    let models = loader.load_models().await?;
    let count = models.len();

    let mut overrides = PROVIDER_OVERRIDES
        .write()
        .map_err(|e| anyhow::anyhow!("Failed to acquire write lock: {}", e))?;
    let provider_models = overrides
        .entry(provider.to_string())
        .or_insert_with(HashMap::new);

    for model in models {
        let model_name = model
            .id
            .strip_prefix(&format!("{}/", provider))
            .unwrap_or(&model.id)
            .to_string();
        provider_models.insert(model_name, model);
    }

    tracing::info!(
        "Loaded {} canonical models from {} API",
        count,
        provider
    );

    Ok(count)
}

/// Look up a single model from the provider overrides.
pub fn get_override(provider: &str, model: &str) -> Option<CanonicalModel> {
    let overrides = PROVIDER_OVERRIDES.read().ok()?;
    overrides.get(provider)?.get(model).cloned()
}

/// Get all override models for a provider.
pub fn get_all_overrides_for_provider(provider: &str) -> Vec<CanonicalModel> {
    PROVIDER_OVERRIDES
        .read()
        .ok()
        .and_then(|overrides| {
            overrides
                .get(provider)
                .map(|m| m.values().cloned().collect())
        })
        .unwrap_or_default()
}

/// Check if overrides have been loaded for a provider.
pub fn has_overrides(provider: &str) -> bool {
    PROVIDER_OVERRIDES
        .read()
        .ok()
        .map(|overrides| overrides.contains_key(provider))
        .unwrap_or(false)
}

/// Clear all overrides (useful for testing).
#[cfg(test)]
pub fn clear_overrides() {
    if let Ok(mut overrides) = PROVIDER_OVERRIDES.write() {
        overrides.clear();
    }
}
