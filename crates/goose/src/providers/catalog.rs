use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Models.dev API data embedded in the binary
const MODELS_DEV_DATA: &str = include_str!("../../../../../models_dev_api.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevProvider {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub npm: String,
    #[serde(default)]
    pub api: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub doc: String,
    pub models: HashMap<String, ModelsDevModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub temperature: bool,
    #[serde(default)]
    pub knowledge: String,
    pub limit: ModelsDevLimit,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsDevLimit {
    pub context: usize,
    #[serde(default)]
    pub output: Option<usize>,
}

/// Provider catalog loaded from models.dev
static PROVIDER_CATALOG: Lazy<HashMap<String, ModelsDevProvider>> = Lazy::new(|| {
    serde_json::from_str::<HashMap<String, ModelsDevProvider>>(MODELS_DEV_DATA)
        .unwrap_or_else(|e| {
            eprintln!("Failed to parse models.dev data: {}", e);
            HashMap::new()
        })
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
pub fn detect_format_from_npm(npm_package: &str) -> ProviderFormat {
    match npm_package {
        "@ai-sdk/openai" | "@ai-sdk/openai-compatible" => ProviderFormat::OpenAI,
        "@ai-sdk/anthropic" => ProviderFormat::Anthropic,
        _ if npm_package.contains("ollama") => ProviderFormat::Ollama,
        _ => ProviderFormat::OpenAI, // Default to most common
    }
}

/// Provider catalog entry for API responses
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct ModelTemplate {
    pub id: String,
    pub name: String,
    pub context_limit: usize,
    pub capabilities: ModelCapabilities,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelCapabilities {
    pub tool_call: bool,
    pub reasoning: bool,
    pub attachment: bool,
    pub temperature: bool,
}

/// Get all providers from catalog filtered by format
pub fn get_providers_by_format(format: ProviderFormat) -> Vec<ProviderCatalogEntry> {
    let mut entries: Vec<ProviderCatalogEntry> = PROVIDER_CATALOG
        .values()
        .filter_map(|provider| {
            let provider_format = detect_format_from_npm(&provider.npm);
            if provider_format != format {
                return None;
            }

            // Only include providers with API URLs
            if provider.api.is_empty() {
                return None;
            }

            Some(ProviderCatalogEntry {
                id: provider.id.clone(),
                name: provider.name.clone(),
                format: format.as_str().to_string(),
                api_url: provider.api.clone(),
                model_count: provider.models.len(),
                doc_url: provider.doc.clone(),
                env_var: provider.env.first().cloned().unwrap_or_default(),
            })
        })
        .collect();

    // Sort by name
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Get provider template by ID for auto-filling form
pub fn get_provider_template(provider_id: &str) -> Option<ProviderTemplate> {
    let provider = PROVIDER_CATALOG.get(provider_id)?;

    // Only return providers with API URLs
    if provider.api.is_empty() {
        return None;
    }

    let format = detect_format_from_npm(&provider.npm);

    let models: Vec<ModelTemplate> = provider
        .models
        .values()
        .filter(|m| m.status.as_deref() != Some("deprecated"))
        .map(|model| ModelTemplate {
            id: model.id.clone(),
            name: model.name.clone(),
            context_limit: model.limit.context,
            capabilities: ModelCapabilities {
                tool_call: model.tool_call,
                reasoning: model.reasoning,
                attachment: model.attachment,
                temperature: model.temperature,
            },
            deprecated: model.status.as_deref() == Some("deprecated"),
        })
        .collect();

    Some(ProviderTemplate {
        id: provider.id.clone(),
        name: provider.name.clone(),
        format: format.as_str().to_string(),
        api_url: provider.api.clone(),
        models,
        supports_streaming: true, // Default to true for most providers
        env_var: provider.env.first().cloned().unwrap_or_default(),
        doc_url: provider.doc.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_from_npm() {
        assert_eq!(
            detect_format_from_npm("@ai-sdk/openai-compatible"),
            ProviderFormat::OpenAI
        );
        assert_eq!(
            detect_format_from_npm("@ai-sdk/anthropic"),
            ProviderFormat::Anthropic
        );
        assert_eq!(
            detect_format_from_npm("@ai-sdk/openai"),
            ProviderFormat::OpenAI
        );
    }

    #[test]
    fn test_get_providers_by_format() {
        let openai_providers = get_providers_by_format(ProviderFormat::OpenAI);
        assert!(!openai_providers.is_empty());

        let anthropic_providers = get_providers_by_format(ProviderFormat::Anthropic);
        assert!(!anthropic_providers.is_empty());
    }

    #[test]
    fn test_get_provider_template() {
        // Test with a provider we know exists in models.dev
        let template = get_provider_template("deepseek");
        assert!(template.is_some());

        if let Some(t) = template {
            assert_eq!(t.id, "deepseek");
            assert!(!t.models.is_empty());
            assert!(!t.api_url.is_empty());
        }
    }
}
