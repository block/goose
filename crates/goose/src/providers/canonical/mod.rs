pub mod loaders;
mod model;
mod name_builder;
mod registry;

pub use model::{CanonicalModel, Limit, Modalities, Modality, Pricing};
pub use name_builder::{canonical_name, map_to_canonical_model, strip_version_suffix};
pub use registry::CanonicalModelRegistry;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelMapping {
    pub provider_model: String,
    pub canonical_model: String,
}

impl ModelMapping {
    pub fn new(provider_model: impl Into<String>, canonical_model: impl Into<String>) -> Self {
        Self {
            provider_model: provider_model.into(),
            canonical_model: canonical_model.into(),
        }
    }
}

pub fn maybe_get_canonical_model(provider: &str, model: &str) -> Option<CanonicalModel> {
    let registry = CanonicalModelRegistry::bundled().ok()?;

    // map_to_canonical_model returns the canonical ID (provider/model)
    // Parse it to get provider and model parts for registry lookup
    let canonical_id = map_to_canonical_model(provider, model, registry)?;
    if let Some((canon_provider, canon_model)) = canonical_id.split_once('/') {
        // Check runtime overrides first (from provider-specific loaders)
        if let Some(override_model) = loaders::get_override(canon_provider, canon_model) {
            return Some(override_model);
        }
        registry.get(canon_provider, canon_model).cloned()
    } else {
        None
    }
}
