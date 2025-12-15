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

/// Parse OpenRouter-style model ID into (provider, model) components
/// e.g., "anthropic/claude-sonnet-4-20250514" -> ("anthropic", "claude-sonnet-4-20250514")
pub fn parse_model_id(model_id: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = model_id.splitn(2, '/').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}
