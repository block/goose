mod model;
mod name_builder;
mod registry;

pub use model::{CanonicalModel, Pricing};
pub use name_builder::{canonical_name, map_to_canonical_model, strip_version_suffix};
pub use registry::CanonicalModelRegistry;

use std::collections::HashMap;

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

/// Get pricing for a specific provider/model combination
/// Returns (input_cost_per_token, output_cost_per_token, context_length) if found
pub fn get_model_pricing(provider: &str, model: &str) -> Option<(f64, f64, Option<u32>)> {
    let registry = CanonicalModelRegistry::bundled().ok()?;
    registry.get_model_pricing(provider, model)
}

/// Get all pricing data organized by provider
/// Returns HashMap<provider, HashMap<model_name, (input_cost, output_cost, context_length)>>
pub fn get_all_pricing() -> HashMap<String, HashMap<String, (f64, f64, Option<u32>)>> {
    CanonicalModelRegistry::bundled()
        .map(|registry| registry.get_all_pricing())
        .unwrap_or_default()
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
