use serde::{Deserialize, Serialize};

/// Modalities supported by a model (mirrors models.dev structure)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Modalities {
    /// Input modalities (e.g., ["text", "image", "pdf"])
    #[serde(default)]
    pub input: Vec<String>,

    /// Output modalities (e.g., ["text"])
    #[serde(default)]
    pub output: Vec<String>,
}

/// Pricing/cost information for a model (all costs in USD per million tokens)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pricing {
    /// Cost per million input tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,

    /// Cost per million output tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,

    /// Cost per million cached read tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,

    /// Cost per million cached write tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
}

/// Token limits for a model
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Limit {
    /// Maximum context window size in tokens
    pub context: usize,

    /// Maximum output/completion tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<usize>,
}

/// Canonical representation of a model (mirrors models.dev API structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalModel {
    /// Model identifier (e.g., "anthropic/claude-3-5-sonnet")
    pub id: String,

    /// Human-readable name (e.g., "Claude Sonnet 3.5 v2")
    pub name: String,

    /// Model family (e.g., "claude-sonnet", "gpt")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,

    /// Whether the model supports tool calling
    #[serde(default)]
    pub tool_call: bool,

    /// Input and output modalities
    #[serde(default)]
    pub modalities: Modalities,

    /// Pricing information
    #[serde(default)]
    pub cost: Pricing,

    /// Token limits
    #[serde(default)]
    pub limit: Limit,
}
