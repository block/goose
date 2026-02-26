pub mod nanogpt;

use super::CanonicalModel;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for providers that supply their own model listings.
///
/// When `build_canonical_models` runs, it fetches from models.dev as the
/// baseline, then calls registered loaders to get fresher/more complete data
/// from provider-specific APIs. The loader data replaces the models.dev data
/// for that provider in the final `canonical_models.json`.
#[async_trait]
pub trait CanonicalModelLoader: Send + Sync {
    /// The canonical provider name (as used in the registry, e.g. "nano-gpt")
    fn provider_name(&self) -> &str;

    /// Fetch models from the provider's API and return as CanonicalModels.
    /// Each model's `id` should be in the format "provider/model-name".
    async fn load_models(&self) -> Result<Vec<CanonicalModel>>;
}

/// All registered provider-specific loaders.
/// Add new loaders here as they're implemented.
pub fn all_loaders() -> Vec<Box<dyn CanonicalModelLoader>> {
    vec![Box::new(nanogpt::NanoGptLoader)]
}
