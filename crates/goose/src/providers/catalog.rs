use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::canonical::CanonicalModelRegistry;

/// Provider metadata embedded in the binary (generated from models.dev)
const PROVIDER_METADATA_JSON: &str =
    include_str!("canonical/data/provider_metadata.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderMetadataEntry {
    pub id: String,
    pub display_name: String,
    pub npm: Option<String>,
    pub api: Option<String>,
    pub doc: Option<String>,
    pub env: Vec<String>,
}

/// Provider metadata loaded from generated JSON
static PROVIDER_METADATA: Lazy<HashMap<String, ProviderMetadataEntry>> = Lazy::new(|| {
    serde_json::from_str::<Vec<ProviderMetadataEntry>>(PROVIDER_METADATA_JSON)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse provider metadata: {}", e);
            Vec::new()
        })
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect()
});

/// Canonical model registry (loaded once, lazily)
static CANONICAL_REGISTRY: Lazy<Option<CanonicalModelRegistry>> = Lazy::new(|| {
    CanonicalModelRegistry::bundled().ok().cloned()
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

/// Detect format from npm package name
fn detect_format_from_npm(npm: &str) -> Option<ProviderFormat> {
    if npm.contains("openai") {
        Some(ProviderFormat::OpenAI)
    } else if npm.contains("anthropic") {
        Some(ProviderFormat::Anthropic)
    } else if npm.contains("ollama") {
        Some(ProviderFormat::Ollama)
    } else {
        None
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

/// Get all providers from catalog filtered by format
pub fn get_providers_by_format(format: ProviderFormat) -> Vec<ProviderCatalogEntry> {
    let registry = CANONICAL_REGISTRY.as_ref();

    let mut entries: Vec<ProviderCatalogEntry> = PROVIDER_METADATA
        .values()
        .filter_map(|metadata| {
            // Filter by npm package format
            let npm = metadata.npm.as_ref()?;
            let detected_format = detect_format_from_npm(npm)?;

            if detected_format != format {
                return None;
            }

            // Get API URL - skip if missing
            let api_url = metadata.api.as_ref()?.clone();

            // Count models for this provider in canonical registry (if available)
            let model_count = registry
                .and_then(|r| Some(r.get_all_models_for_provider(&metadata.id).len()))
                .unwrap_or(0);

            // Get env var (first one or generate default)
            let env_var = metadata.env.first()
                .cloned()
                .unwrap_or_else(|| format!("{}_API_KEY", metadata.id.to_uppercase().replace('-', "_")));

            Some(ProviderCatalogEntry {
                id: metadata.id.clone(),
                name: metadata.display_name.clone(),
                format: detected_format.as_str().to_string(),
                api_url,
                model_count,
                doc_url: metadata.doc.clone().unwrap_or_default(),
                env_var,
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

    // Get npm package and detect format
    let npm = metadata.npm.as_ref()?;
    let format = detect_format_from_npm(npm)?;

    // Get API URL
    let api_url = metadata.api.as_ref()?.clone();

    // Get all models for this provider from canonical registry (if available)
    let models: Vec<ModelTemplate> = CANONICAL_REGISTRY
        .as_ref()
        .map(|registry| {
            registry.get_all_models_for_provider(provider_id)
                .into_iter()
                .map(|model| {
                    // Extract just the model ID (without provider prefix)
                    let model_id = model.id
                        .strip_prefix(&format!("{}/", provider_id))
                        .unwrap_or(&model.id)
                        .to_string();

                    ModelTemplate {
                        id: model_id,
                        name: model.name.clone(),
                        context_limit: model.limit.context,
                        capabilities: ModelCapabilities {
                            tool_call: model.tool_call,
                            reasoning: model.reasoning.unwrap_or(false),
                            attachment: model.attachment.unwrap_or(false),
                            temperature: model.temperature.unwrap_or(false),
                        },
                        deprecated: false, // Canonical models don't have deprecated flag
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Get env var (first one or generate default)
    let env_var = metadata.env.first()
        .cloned()
        .unwrap_or_else(|| format!("{}_API_KEY", provider_id.to_uppercase().replace('-', "_")));

    Some(ProviderTemplate {
        id: metadata.id.clone(),
        name: metadata.display_name.clone(),
        format: format.as_str().to_string(),
        api_url,
        models,
        supports_streaming: true, // Default to true
        env_var,
        doc_url: metadata.doc.clone().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_providers_by_format() {
        let openai_providers = get_providers_by_format(ProviderFormat::OpenAI);
        assert!(!openai_providers.is_empty());
        println!("OpenAI compatible providers: {}", openai_providers.len());
        for provider in openai_providers.iter().take(3) {
            println!("  - {} ({}) - {} models", provider.name, provider.id, provider.model_count);
        }

        let anthropic_providers = get_providers_by_format(ProviderFormat::Anthropic);
        println!("Anthropic compatible providers: {}", anthropic_providers.len());
        for provider in anthropic_providers.iter().take(3) {
            println!("  - {} ({}) - {} models", provider.name, provider.id, provider.model_count);
        }
    }

    #[test]
    fn test_get_provider_template() {
        // Test with providers we know exist
        let openai_providers = get_providers_by_format(ProviderFormat::OpenAI);
        if let Some(first) = openai_providers.first() {
            let template = get_provider_template(&first.id);
            assert!(template.is_some());

            if let Some(t) = template {
                assert!(!t.models.is_empty());
                assert!(!t.api_url.is_empty());
            }
        }
    }
}
