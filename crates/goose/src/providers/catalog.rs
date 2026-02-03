use super::canonical::{CanonicalModel, CanonicalModelRegistry};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provider metadata embedded in the binary
const PROVIDER_METADATA_JSON: &str =
    include_str!("canonical/data/provider_metadata.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub id: String,
    pub display_name: String,
    pub format: String,
    pub api_url: String,
    pub doc_url: String,
    pub env_var: String,
    pub supports_streaming: bool,
    pub requires_auth: bool,
}

/// Provider metadata map loaded from JSON
static PROVIDER_METADATA: Lazy<HashMap<String, ProviderMetadata>> = Lazy::new(|| {
    serde_json::from_str::<Vec<ProviderMetadata>>(PROVIDER_METADATA_JSON)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse provider metadata: {}", e);
            Vec::new()
        })
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect()
});

/// Engine/format compatibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderFormat {
    OpenAI,
    Anthropic,
    Ollama,
}

impl ProviderFormat {
    pub fn as_str(&self) -> &str {
        match self {
            ProviderFormat::OpenAI => "openai",
            ProviderFormat::Anthropic => "anthropic",
            ProviderFormat::Ollama => "ollama",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "openai_compatible" => Some(ProviderFormat::OpenAI),
            "anthropic" | "anthropic_compatible" => Some(ProviderFormat::Anthropic),
            "ollama" | "ollama_compatible" => Some(ProviderFormat::Ollama),
            _ => None,
        }
    }
}

/// Provider catalog entry for API responses
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProviderCatalogEntry {
    pub id: String,
    pub name: String,
    pub format: String,
    pub api_url: String,
    pub model_count: usize,
    pub doc_url: String,
    pub env_var: String,
}

/// Provider template for auto-filling custom provider form
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProviderTemplate {
    pub id: String,
    pub name: String,
    pub format: String,
    pub api_url: String,
    pub models: Vec<ModelTemplate>,
    pub supports_streaming: bool,
    pub env_var: String,
    pub doc_url: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ModelTemplate {
    pub id: String,
    pub name: String,
    pub context_limit: usize,
    pub capabilities: ModelCapabilities,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ModelCapabilities {
    pub tool_call: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub temperature: bool,
}

/// Get models for a provider from canonical registry
fn get_provider_models(provider_id: &str) -> Vec<CanonicalModel> {
    let registry = match CanonicalModelRegistry::bundled() {
        Ok(reg) => reg,
        Err(e) => {
            eprintln!("Failed to load canonical models: {}", e);
            return Vec::new();
        }
    };

    registry
        .all_models()
        .iter()
        .filter(|m| m.id.starts_with(&format!("{}/", provider_id)))
        .map(|m| (*m).clone())
        .collect()
}

/// Get all providers from catalog filtered by format
pub fn get_providers_by_format(format: ProviderFormat) -> Vec<ProviderCatalogEntry> {
    let mut entries: Vec<ProviderCatalogEntry> = PROVIDER_METADATA
        .values()
        .filter_map(|metadata| {
            // Filter by format
            if metadata.format != format.as_str() {
                return None;
            }

            // Get model count from canonical models
            let models = get_provider_models(&metadata.id);

            // Skip providers with no models
            if models.is_empty() {
                return None;
            }

            Some(ProviderCatalogEntry {
                id: metadata.id.clone(),
                name: metadata.display_name.clone(),
                format: metadata.format.clone(),
                api_url: metadata.api_url.clone(),
                model_count: models.len(),
                doc_url: metadata.doc_url.clone(),
                env_var: metadata.env_var.clone(),
            })
        })
        .collect();

    // Sort by name
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Get provider template by ID for auto-filling form
pub fn get_provider_template(provider_id: &str) -> Option<ProviderTemplate> {
    let metadata = PROVIDER_METADATA.get(provider_id)?;

    // Get models from canonical registry
    let canonical_models = get_provider_models(provider_id);

    if canonical_models.is_empty() {
        return None;
    }

    let models: Vec<ModelTemplate> = canonical_models
        .iter()
        .map(|model| {
            // Extract just the model name (after "provider/")
            let model_name = model
                .id
                .strip_prefix(&format!("{}/", provider_id))
                .unwrap_or(&model.id);

            ModelTemplate {
                id: model_name.to_string(),
                name: model.name.clone(),
                context_limit: model.limit.context,
                capabilities: ModelCapabilities {
                    tool_call: model.tool_call,
                    reasoning: model.reasoning.unwrap_or(false),
                    attachment: model.attachment.unwrap_or(false),
                    temperature: model.temperature.unwrap_or(true),
                },
                deprecated: false, // Canonical models don't track deprecation yet
            }
        })
        .collect();

    Some(ProviderTemplate {
        id: metadata.id.clone(),
        name: metadata.display_name.clone(),
        format: metadata.format.clone(),
        api_url: metadata.api_url.clone(),
        models,
        supports_streaming: metadata.supports_streaming,
        env_var: metadata.env_var.clone(),
        doc_url: metadata.doc_url.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_providers_by_format() {
        let openai_providers = get_providers_by_format(ProviderFormat::OpenAI);
        assert!(!openai_providers.is_empty());

        let anthropic_providers = get_providers_by_format(ProviderFormat::Anthropic);
        assert!(!anthropic_providers.is_empty());
    }

    #[test]
    fn test_get_provider_template() {
        // Test with providers we know exist in canonical models
        let template = get_provider_template("anthropic");
        assert!(template.is_some());

        if let Some(t) = template {
            assert_eq!(t.id, "anthropic");
            assert!(!t.models.is_empty());
            assert!(!t.api_url.is_empty());
        }
    }

    #[test]
    fn test_all_metadata_providers_have_models() {
        // Verify that all providers in metadata have models in canonical registry
        for (provider_id, metadata) in PROVIDER_METADATA.iter() {
            let models = get_provider_models(provider_id);
            assert!(
                !models.is_empty(),
                "Provider {} ({}) has no models in canonical registry",
                provider_id,
                metadata.display_name
            );
        }
    }
}
