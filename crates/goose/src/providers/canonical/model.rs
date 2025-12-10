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

    /// Whether the model supports tool calling
    #[serde(default)]
    pub supports_tools: bool,

    /// Pricing for this model
    pub pricing: Pricing,
}
