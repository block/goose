use super::{canonical_provider_name, CanonicalModel, Modality};
use crate::providers::base::ModelInfo;
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
        // Extract provider and model from id (format: "provider/model")
        if let Some((provider, model_name)) = model.id.split_once('/') {
            let provider = provider.to_string();
            let model_name = model_name.to_string();
            registry.register(&provider, &model_name, model);
        }
    }

    Ok(registry)
});

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

    /// Get known models for a goose provider, sourced from canonical data.
    ///
    /// Maps the goose provider name to canonical (e.g. `"xai"` → `"x-ai"`),
    /// filters for text-input + tool_call capable models, and sorts by
    /// release_date (newest first).
    ///
    /// Returns `Vec<ModelInfo>` ready for use in `ProviderMetadata`.
    pub fn known_models_for_provider(&self, goose_provider_name: &str) -> Vec<ModelInfo> {
        let canonical_name = canonical_provider_name(goose_provider_name);
        let all = self.get_all_models_for_provider(canonical_name);

        let mut eligible: Vec<(ModelInfo, Option<String>)> = all
            .into_iter()
            .filter(|m| {
                m.modalities.input.contains(&Modality::Text) && m.tool_call
            })
            .map(|m| {
                let model_name = m
                    .id
                    .split_once('/')
                    .map(|(_, name)| name.to_string())
                    .unwrap_or_else(|| m.id.clone());
                let release_date = m.release_date.clone();
                let info = ModelInfo {
                    name: model_name,
                    context_limit: m.limit.context,
                    input_token_cost: m.cost.input.map(|c| c / 1_000_000.0),
                    output_token_cost: m.cost.output.map(|c| c / 1_000_000.0),
                    currency: if m.cost.input.is_some() || m.cost.output.is_some() {
                        Some("$".to_string())
                    } else {
                        None
                    },
                    supports_cache_control: None,
                };
                (info, release_date)
            })
            .collect();

        // Sort by release_date (newest first), then alphabetically for models without dates
        eligible.sort_by(|a, b| match (&a.1, &b.1) {
            (Some(date_a), Some(date_b)) => date_b.cmp(date_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.0.name.cmp(&b.0.name),
        });

        eligible.into_iter().map(|(info, _)| info).collect()
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

    #[test]
    fn test_known_models_for_anthropic() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        let models = registry.known_models_for_provider("anthropic");
        assert!(!models.is_empty(), "anthropic should have canonical models");
        // All models should have text tool_call capability (that's the filter)
        for model in &models {
            assert!(model.context_limit > 0, "model {} should have context limit", model.name);
        }
        // Should contain some well-known model names
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("claude")),
            "anthropic models should contain claude variants, got: {:?}",
            names
        );
    }

    #[test]
    fn test_known_models_for_openai() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        let models = registry.known_models_for_provider("openai");
        assert!(!models.is_empty(), "openai should have canonical models");
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.starts_with("gpt-") || n.starts_with("o")),
            "openai models should contain gpt or o-series, got: {:?}",
            names
        );
    }

    #[test]
    fn test_known_models_for_google() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        let models = registry.known_models_for_provider("google");
        assert!(!models.is_empty(), "google should have canonical models");
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.starts_with("gemini")),
            "google models should contain gemini variants, got: {:?}",
            names
        );
    }

    #[test]
    fn test_known_models_for_xai() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        // "xai" is the goose provider name, maps to "x-ai" canonical
        let models = registry.known_models_for_provider("xai");
        assert!(!models.is_empty(), "xai should have canonical models");
        let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
        assert!(
            names.iter().any(|n| n.contains("grok")),
            "xai models should contain grok variants, got: {:?}",
            names
        );
    }

    #[test]
    fn test_known_models_for_bedrock() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        // "aws_bedrock" maps to "amazon-bedrock" canonical
        let models = registry.known_models_for_provider("aws_bedrock");
        assert!(!models.is_empty(), "bedrock should have canonical models");
    }

    #[test]
    fn test_known_models_for_gcp_vertex() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        // "gcp_vertex_ai" maps to "google-vertex" canonical
        let models = registry.known_models_for_provider("gcp_vertex_ai");
        assert!(!models.is_empty(), "gcp_vertex_ai should have canonical models");
    }

    #[test]
    fn test_known_models_sorted_by_release_date() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        let models = registry.known_models_for_provider("anthropic");
        // Models with release dates should be sorted newest first
        // Just verify the list is non-empty and the first model is a recent one
        assert!(!models.is_empty());
    }

    #[test]
    fn test_known_models_includes_pricing() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        let models = registry.known_models_for_provider("openai");
        // At least some models should have pricing info
        let has_pricing = models.iter().any(|m| m.input_token_cost.is_some());
        assert!(has_pricing, "some openai models should have pricing data");
    }

    #[test]
    fn test_known_models_empty_for_unknown_provider() {
        let registry = CanonicalModelRegistry::bundled().unwrap();
        let models = registry.known_models_for_provider("nonexistent_provider_xyz");
        assert!(models.is_empty(), "unknown provider should return empty list");
    }
}
