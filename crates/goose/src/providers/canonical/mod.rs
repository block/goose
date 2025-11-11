mod model;
mod name_builder;
mod registry;

pub use model::{CanonicalModel, ModelType};
pub use name_builder::{canonical_name, strip_version_suffix};
pub use registry::CanonicalModelRegistry;

/// Represents a mapping from a provider's model name to a canonical model
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelMapping {
    /// The provider's model name/identifier
    pub provider_model: String,
    /// The canonical model name this maps to
    pub canonical_model: String,
    /// Whether this mapping is confirmed/verified
    pub verified: bool,
}

impl ModelMapping {
    pub fn new(provider_model: impl Into<String>, canonical_model: impl Into<String>) -> Self {
        Self {
            provider_model: provider_model.into(),
            canonical_model: canonical_model.into(),
            verified: false,
        }
    }

    pub fn verified(mut self) -> Self {
        self.verified = true;
        self
    }
}
