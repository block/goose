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

/// Pricing/cost information for a model
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Pricing {
    /// Cost in USD per million input tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,

    /// Cost in USD per million output tokens
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

    /// Whether the model supports attachments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment: Option<bool>,

    /// Whether the model supports reasoning/thinking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,

    /// Whether the model supports tool calling
    #[serde(default)]
    pub tool_call: bool,

    /// Whether the model supports temperature parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<bool>,

    /// Knowledge cutoff date (e.g., "2024-04-30")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub knowledge: Option<String>,

    /// Release date (e.g., "2024-10-22")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,

    /// Last updated date (e.g., "2024-10-22")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,

    /// Input and output modalities
    #[serde(default)]
    pub modalities: Modalities,

    /// Whether the model has open weights
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_weights: Option<bool>,

    /// Pricing information
    #[serde(default)]
    pub cost: Pricing,

    /// Token limits
    #[serde(default)]
    pub limit: Limit,
}
