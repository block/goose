mod model;
mod name_builder;
mod registry;

pub use model::{CanonicalModel, Pricing};
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
    let canonical_id = map_to_canonical_model(provider, model, registry)?;
    registry.get(&canonical_id).cloned()
}
