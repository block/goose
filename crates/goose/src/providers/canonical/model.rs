use serde::{Deserialize, Serialize};

/// Pricing information for a model variant (all costs in USD per token as strings)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    /// Cost per prompt token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    /// Cost per completion token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<String>,

    /// Cost per request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,

    /// Cost per image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// Cost per audio token/unit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    /// Cost for web search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<String>,

    /// Cost for internal reasoning tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_reasoning: Option<String>,

    /// Cost for input cache reads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cache_read: Option<String>,

    /// Cost for input cache writes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cache_write: Option<String>,
}

impl Pricing {
    /// Parse prompt cost as f64 (cost per token)
    pub fn prompt_cost(&self) -> Option<f64> {
        self.prompt.as_ref().and_then(|s| s.parse().ok())
    }

    /// Parse completion cost as f64 (cost per token)
    pub fn completion_cost(&self) -> Option<f64> {
        self.completion.as_ref().and_then(|s| s.parse().ok())
    }
}

/// A pricing variant for a model (e.g., base, extended, thinking)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariant {
    /// Variant name (empty string "" for base model, or "extended", "thinking", etc.)
    pub variant: String,

    /// Pricing for this variant
    pub pricing: Pricing,
}

/// Canonical representation of a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalModel {
    /// Model identifier (e.g., "anthropic/claude-3-5-sonnet")
    pub id: String,

    /// Human-readable name (e.g., "Claude 3.5 Sonnet")
    pub name: String,

    /// Maximum context window size in tokens
    pub context_length: usize,

    /// Maximum completion tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<usize>,

    /// Input modalities supported (e.g., ["text", "image"])
    #[serde(default)]
    pub input_modalities: Vec<String>,

    /// Output modalities supported (e.g., ["text"])
    #[serde(default)]
    pub output_modalities: Vec<String>,

    /// Tokenizer type (e.g., "GPT", "Claude", "Gemini")
    pub tokenizer: String,

    /// Pricing variants for this model (base variant has variant = "")
    pub variants: Vec<ModelVariant>,
}

impl CanonicalModel {
    /// Get the base variant (variant = "")
    pub fn base_variant(&self) -> Option<&ModelVariant> {
        self.variants.iter().find(|v| v.variant.is_empty())
    }

    /// Get a specific variant by name
    pub fn get_variant(&self, variant: &str) -> Option<&ModelVariant> {
        self.variants.iter().find(|v| v.variant == variant)
    }

    /// Check if the model supports prompt caching (any variant has cache pricing)
    pub fn supports_cache(&self) -> bool {
        self.variants.iter().any(|v| {
            v.pricing.input_cache_read.is_some() || v.pricing.input_cache_write.is_some()
        })
    }

    /// Check if the model supports vision/image inputs
    pub fn supports_vision(&self) -> bool {
        self.input_modalities.contains(&"image".to_string())
    }

    /// Get the provider name from the id (e.g., "anthropic" from "anthropic/claude-3-5-sonnet")
    pub fn provider(&self) -> Option<&str> {
        self.id.split('/').next()
    }

    /// Get the model name without the provider prefix
    pub fn model_name(&self) -> Option<&str> {
        self.id.split('/').nth(1)
    }

    /// Get base variant prompt cost as f64 (cost per token)
    pub fn prompt_cost(&self) -> Option<f64> {
        self.base_variant().and_then(|v| v.pricing.prompt_cost())
    }

    /// Get base variant completion cost as f64 (cost per token)
    pub fn completion_cost(&self) -> Option<f64> {
        self.base_variant()
            .and_then(|v| v.pricing.completion_cost())
    }
}
