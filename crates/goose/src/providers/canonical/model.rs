use serde::{Deserialize, Serialize};

/// Architecture information for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    /// The modality of the model (e.g., "text->text", "text+image->text")
    pub modality: String,

    /// Input modalities supported (e.g., ["text", "image"])
    #[serde(default)]
    pub input_modalities: Vec<String>,

    /// Output modalities supported (e.g., ["text"])
    #[serde(default)]
    pub output_modalities: Vec<String>,

    /// Tokenizer type
    pub tokenizer: String,

    /// Instruction type, if applicable
    pub instruct_type: Option<String>,
}

/// Pricing information for a model (all costs in USD)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    /// Cost per prompt token
    pub prompt: String,

    /// Cost per completion token
    pub completion: String,

    /// Cost per request
    #[serde(default)]
    pub request: String,

    /// Cost per image token
    #[serde(default)]
    pub image: String,

    /// Cost for input cache reads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cache_read: Option<String>,

    /// Cost for input cache writes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cache_write: Option<String>,
}

/// Top provider information for a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProvider {
    /// Context length from the top provider (may be null for some models)
    pub context_length: Option<usize>,

    /// Maximum completion tokens
    #[serde(default)]
    pub max_completion_tokens: Option<usize>,

    /// Whether the model is moderated
    #[serde(default)]
    pub is_moderated: bool,
}

/// Canonical representation of a model based on OpenRouter's schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalModel {
    /// OpenRouter's API identifier (e.g., "anthropic/claude-sonnet-4.5")
    pub id: String,

    /// Canonical slug - standardized reference with version info (e.g., "anthropic/claude-4.5-sonnet-20250929")
    /// This is our primary identifier for model mapping
    pub canonical_slug: String,

    /// Human-readable name
    pub name: String,

    /// Unix timestamp of when the model was created
    #[serde(default)]
    pub created: Option<u64>,

    /// Description of the model
    #[serde(default)]
    pub description: String,

    /// Maximum context window size in tokens
    pub context_length: usize,

    /// Architecture information
    pub architecture: Architecture,

    /// Pricing information (all in USD)
    pub pricing: Pricing,

    /// Top provider metadata
    pub top_provider: TopProvider,

    /// List of supported parameters (e.g., "temperature", "top_p", "tools")
    #[serde(default)]
    pub supported_parameters: Vec<String>,
}

impl CanonicalModel {
    /// Check if the model supports tool/function calling
    pub fn supports_tools(&self) -> bool {
        self.supported_parameters.iter().any(|p| p == "tools" || p == "tool_choice")
    }

    /// Check if the model supports prompt caching
    pub fn supports_cache(&self) -> bool {
        self.pricing.input_cache_read.is_some() || self.pricing.input_cache_write.is_some()
    }

    /// Check if the model supports vision/image inputs
    pub fn supports_vision(&self) -> bool {
        self.architecture.input_modalities.contains(&"image".to_string())
    }

    /// Get the provider name from the canonical slug (e.g., "anthropic" from "anthropic/claude-3-5-sonnet")
    pub fn provider(&self) -> Option<&str> {
        self.canonical_slug.split('/').next()
    }

    /// Get the model name without the provider prefix
    pub fn model_name(&self) -> Option<&str> {
        self.canonical_slug.split('/').nth(1)
    }

    /// Parse prompt cost as f64 (cost per token)
    pub fn prompt_cost(&self) -> Option<f64> {
        self.pricing.prompt.parse().ok()
    }

    /// Parse completion cost as f64 (cost per token)
    pub fn completion_cost(&self) -> Option<f64> {
        self.pricing.completion.parse().ok()
    }
}