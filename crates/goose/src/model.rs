use serde::{Deserialize, Serialize};

const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

// Tokenizer names, used to infer from model name
pub const GPT_4O_TOKENIZER: &str = "Xenova--gpt-4o";
pub const CLAUDE_TOKENIZER: &str = "Xenova--claude-tokenizer";

/// Configuration for model-specific settings and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The name of the model to use
    pub model_name: String,
    // Optional tokenizer name (corresponds to the sanitized HuggingFace tokenizer name)
    // "Xenova/gpt-4o" -> "Xenova/gpt-4o"
    // If not provided, best attempt will be made to infer from model name or default
    pub tokenizer_name: String,
    /// Optional explicit context limit that overrides any defaults
    pub context_limit: Option<usize>,
    /// Optional temperature setting (0.0 - 1.0)
    pub temperature: Option<f32>,
    /// Optional maximum tokens to generate
    pub max_tokens: Option<i32>,
    /// Whether to interpret tool calls with toolshim
    pub toolshim: bool,
    /// Model to use for toolshim (optional as a default exists)
    pub toolshim_model: Option<String>,
}

impl ModelConfig {
    /// Create a new ModelConfig with the specified model name
    ///
    /// The context limit is set with the following precedence:
    /// 1. Explicit context_limit if provided in config
    /// 2. Model-specific default based on model name
    /// 3. Global default (128_000) (in get_context_limit)
    pub fn new(model_name: String) -> Self {
        let context_limit = Self::get_model_specific_limit(&model_name);
        let tokenizer_name = Self::infer_tokenizer_name(&model_name);

        let toolshim = std::env::var("GOOSE_TOOLSHIM")
            .map(|val| val == "1" || val.to_lowercase() == "true")
            .unwrap_or(false);

        let toolshim_model = std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL").ok();

        let temperature = std::env::var("GOOSE_TEMPERATURE")
            .ok()
            .and_then(|val| val.parse::<f32>().ok());

        Self {
            model_name,
            tokenizer_name: tokenizer_name.to_string(),
            context_limit,
            temperature,
            max_tokens: None,
            toolshim,
            toolshim_model,
        }
    }

    fn infer_tokenizer_name(model_name: &str) -> &'static str {
        if model_name.contains("claude") {
            CLAUDE_TOKENIZER
        } else {
            // Default tokenizer
            GPT_4O_TOKENIZER
        }
    }

    /// Get model-specific context limit based on model name
    fn get_model_specific_limit(model_name: &str) -> Option<usize> {
        // Implement some sensible defaults
        match model_name {
            // OpenAI models, https://platform.openai.com/docs/models#models-overview
            name if name.contains("gpt-4o") => Some(128_000),
            name if name.contains("gpt-4-turbo") => Some(128_000),
            name if name.contains("o1-mini") || name.contains("o1-preview") => Some(128_000),
            name if name.contains("o1") => Some(200_000),
            name if name.contains("o3-mini") => Some(200_000),
            name if name.contains("gpt-4.1") => Some(1_000_000),
            name if name.contains("gpt-4-1") => Some(1_000_000),

            // Anthropic models, https://docs.anthropic.com/en/docs/about-claude/models
            name if name.contains("claude-3") => Some(200_000),

            // Google models, https://ai.google/get-started/our-models/
            name if name.contains("gemini-2.5") => Some(1_000_000),
            name if name.contains("gemini-2-5") => Some(1_000_000),

            // Meta Llama models, https://github.com/meta-llama/llama-models/tree/main?tab=readme-ov-file#llama-models-1
            name if name.contains("llama3.2") => Some(128_000),
            name if name.contains("llama3.3") => Some(128_000),
            _ => None,
        }
    }

    /// Set an explicit context limit
    pub fn with_context_limit(mut self, limit: Option<usize>) -> Self {
        // Default is None and therefore DEFAULT_CONTEXT_LIMIT, only set
        // if input is Some to allow passing through with_context_limit in
        // configuration cases
        if limit.is_some() {
            self.context_limit = limit;
        }
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temp: Option<f32>) -> Self {
        self.temperature = temp;
        self
    }

    /// Set the max tokens
    pub fn with_max_tokens(mut self, tokens: Option<i32>) -> Self {
        self.max_tokens = tokens;
        self
    }

    /// Set whether to interpret tool calls
    pub fn with_toolshim(mut self, toolshim: bool) -> Self {
        self.toolshim = toolshim;
        self
    }

    /// Set the tool call interpreter model
    pub fn with_toolshim_model(mut self, model: Option<String>) -> Self {
        self.toolshim_model = model;
        self
    }

    /// Get the tokenizer name
    pub fn tokenizer_name(&self) -> &str {
        &self.tokenizer_name
    }

    /// Get the context_limit for the current model
    /// If none are defined, use the DEFAULT_CONTEXT_LIMIT
    pub fn context_limit(&self) -> usize {
        self.context_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_config_context_limits() {
        // Test explicit limit
        let config =
            ModelConfig::new("claude-3-opus".to_string()).with_context_limit(Some(150_000));
        assert_eq!(config.context_limit(), 150_000);

        // Test model-specific defaults
        let config = ModelConfig::new("claude-3-opus".to_string());
        assert_eq!(config.context_limit(), 200_000);

        let config = ModelConfig::new("gpt-4-turbo".to_string());
        assert_eq!(config.context_limit(), 128_000);

        // Test fallback to default
        let config = ModelConfig::new("unknown-model".to_string());
        assert_eq!(config.context_limit(), DEFAULT_CONTEXT_LIMIT);
    }

    #[test]
    fn test_model_config_settings() {
        let config = ModelConfig::new("test-model".to_string())
            .with_temperature(Some(0.7))
            .with_max_tokens(Some(1000))
            .with_context_limit(Some(50_000));

        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.context_limit, Some(50_000));
    }

    #[test]
    fn test_model_config_tool_interpretation() {
        // Test without env vars - should be false
        let config = ModelConfig::new("test-model".to_string());
        assert!(!config.toolshim);

        // Test with tool interpretation setting
        let config = ModelConfig::new("test-model".to_string()).with_toolshim(true);
        assert!(config.toolshim);

        // Test tool interpreter model
        let config = ModelConfig::new("test-model".to_string())
            .with_toolshim_model(Some("mistral-nemo".to_string()));
        assert_eq!(config.toolshim_model, Some("mistral-nemo".to_string()));
    }

    #[test]
    fn test_model_config_temp_env_var() {
        use temp_env::with_var;

        with_var("GOOSE_TEMPERATURE", Some("0.128"), || {
            let config = ModelConfig::new("test-model".to_string());
            assert_eq!(config.temperature, Some(0.128));
        });

        with_var("GOOSE_TEMPERATURE", Some("notanum"), || {
            let config = ModelConfig::new("test-model".to_string());
            assert_eq!(config.temperature, None);
        });

        with_var("GOOSE_TEMPERATURE", Some(""), || {
            let config = ModelConfig::new("test-model".to_string());
            assert_eq!(config.temperature, None);
        });

        let config = ModelConfig::new("test-model".to_string());
        assert_eq!(config.temperature, None);
    }
}
