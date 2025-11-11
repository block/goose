use serde::{Deserialize, Serialize};

/// The type of model (chat, voice, embedding, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelType {
    /// Text generation / chat completion model
    Chat,
    /// Voice/audio model
    Voice,
    /// Embedding model
    Embedding,
    /// Image generation model
    Image,
    /// Other/unknown type
    Other,
}

/// Canonical representation of a model with standardized metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalModel {
    /// Canonical name for this model (e.g., "claude-3-5-sonnet-20241022")
    pub name: String,

    /// The type of model
    pub model_type: ModelType,

    /// Maximum context window size in tokens
    pub context_limit: usize,

    /// Whether the model supports streaming responses
    pub supports_streaming: bool,

    /// Whether the model supports tool/function calling
    pub supports_tools: bool,

    /// Whether the model supports vision/image inputs
    pub supports_vision: bool,

    /// Whether the model supports computer use/MCP
    pub supports_computer_use: bool,

    /// Cost per million input tokens
    pub input_token_cost: Option<f64>,

    /// Cost per million output tokens
    pub output_token_cost: Option<f64>,

    /// Currency for pricing (defaults to USD)
    #[serde(default = "default_currency")]
    pub currency: String,

    /// Whether the model supports prompt caching
    #[serde(default)]
    pub supports_cache_control: bool,

    /// Additional metadata as key-value pairs
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

fn default_currency() -> String {
    "USD".to_string()
}

impl CanonicalModel {
    /// Create a new canonical model with minimal required fields
    pub fn new(
        name: impl Into<String>,
        model_type: ModelType,
        context_limit: usize,
    ) -> Self {
        Self {
            name: name.into(),
            model_type,
            context_limit,
            supports_streaming: false,
            supports_tools: false,
            supports_vision: false,
            supports_computer_use: false,
            input_token_cost: None,
            output_token_cost: None,
            currency: default_currency(),
            supports_cache_control: false,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Builder method to set streaming support
    pub fn with_streaming(mut self, supports: bool) -> Self {
        self.supports_streaming = supports;
        self
    }

    /// Builder method to set tool support
    pub fn with_tools(mut self, supports: bool) -> Self {
        self.supports_tools = supports;
        self
    }

    /// Builder method to set vision support
    pub fn with_vision(mut self, supports: bool) -> Self {
        self.supports_vision = supports;
        self
    }

    /// Builder method to set computer use support
    pub fn with_computer_use(mut self, supports: bool) -> Self {
        self.supports_computer_use = supports;
        self
    }

    /// Builder method to set pricing
    pub fn with_pricing(mut self, input_cost: f64, output_cost: f64) -> Self {
        self.input_token_cost = Some(input_cost);
        self.output_token_cost = Some(output_cost);
        self
    }

    /// Builder method to set cache control support
    pub fn with_cache_control(mut self, supports: bool) -> Self {
        self.supports_cache_control = supports;
        self
    }

    /// Builder method to add custom metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_model_builder() {
        let model = CanonicalModel::new("test-model", ModelType::Chat, 8192)
            .with_streaming(true)
            .with_tools(true)
            .with_pricing(1.0, 2.0);

        assert_eq!(model.name, "test-model");
        assert_eq!(model.model_type, ModelType::Chat);
        assert_eq!(model.context_limit, 8192);
        assert!(model.supports_streaming);
        assert!(model.supports_tools);
        assert_eq!(model.input_token_cost, Some(1.0));
        assert_eq!(model.output_token_cost, Some(2.0));
    }
}
