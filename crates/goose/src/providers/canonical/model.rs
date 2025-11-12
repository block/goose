use serde::{Deserialize, Serialize};

/// Pricing information for a model (all costs in USD per token)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    /// Cost per prompt token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<f64>,

    /// Cost per completion token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<f64>,

    /// Cost per request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<f64>,

    /// Cost per image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<f64>,

    /// Cost per audio token/unit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<f64>,

    /// Cost for web search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<f64>,

    /// Cost for internal reasoning tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_reasoning: Option<f64>,

    /// Cost for input cache reads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cache_read: Option<f64>,

    /// Cost for input cache writes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_cache_write: Option<f64>,
}

/// Canonical representation of a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalModel {
    /// Model identifier (e.g., "anthropic/claude-3-5-sonnet" or "openai/gpt-4o:extended")
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

    /// Whether the model supports tool calling
    #[serde(default)]
    pub supports_tools: bool,

    /// Pricing for this model
    pub pricing: Pricing,
}

impl CanonicalModel {
    /// Check if the model supports prompt caching
    pub fn supports_cache(&self) -> bool {
        self.pricing.input_cache_read.is_some() || self.pricing.input_cache_write.is_some()
    }

    /// Check if the model supports vision/image inputs
    pub fn supports_vision(&self) -> bool {
        self.input_modalities.contains(&"image".to_string())
    }

    /// Get the provider name from the id (e.g., "anthropic" from "anthropic/claude-3-5-sonnet")
    pub fn provider(&self) -> Option<&str> {
        self.id.split('/').next()
    }

    /// Get the model name without the provider prefix (may include variant like ":extended")
    pub fn model_name(&self) -> Option<&str> {
        self.id.split('/').nth(1)
    }

    /// Get the base model ID without variant (e.g., "anthropic/claude-3.7-sonnet:thinking" -> "anthropic/claude-3.7-sonnet")
    pub fn base_model_id(&self) -> String {
        if let Some(pos) = self.id.rfind(':') {
            self.id[..pos].to_string()
        } else {
            self.id.clone()
        }
    }

    /// Get the variant suffix if present (e.g., "thinking" from "claude-3.7-sonnet:thinking")
    pub fn variant(&self) -> Option<&str> {
        self.id.split(':').nth(1)
    }

    /// Get prompt cost as f64 (cost per token)
    pub fn prompt_cost(&self) -> Option<f64> {
        self.pricing.prompt
    }

    /// Get completion cost as f64 (cost per token)
    pub fn completion_cost(&self) -> Option<f64> {
        self.pricing.completion
    }
}
