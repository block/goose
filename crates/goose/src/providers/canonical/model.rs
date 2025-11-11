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
    /// Canonical name for this model (e.g., "anthropic/claude-3-5-sonnet")
    pub name: String,

    /// The type of model
    pub model_type: ModelType,

    /// Maximum context window size in tokens
    pub context_limit: usize,

    /// Whether the model supports streaming responses
    pub supports_streaming: bool,

    /// Whether the model supports tool/function calling
    pub supports_tools: bool,

    /// Cost per million input tokens (in USD)
    pub input_token_cost: Option<f64>,

    /// Cost per million output tokens (in USD)
    pub output_token_cost: Option<f64>,

    /// Whether the model supports prompt caching
    pub supports_cache_control: bool,
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
            input_token_cost: None,
            output_token_cost: None,
            supports_cache_control: false,
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

    /// Builder method to set pricing (in USD per million tokens)
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
