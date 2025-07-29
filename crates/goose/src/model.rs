use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

// Define the model limits as a static HashMap for reuse
static MODEL_SPECIFIC_LIMITS: Lazy<HashMap<&'static str, usize>> = Lazy::new(|| {
    let mut map = HashMap::new();
    // OpenAI models, https://platform.openai.com/docs/models#models-overview
    map.insert("gpt-4o", 128_000);
    map.insert("gpt-4-turbo", 128_000);
    map.insert("o3", 200_000);
    map.insert("o3-mini", 200_000);
    map.insert("o4-mini", 200_000);
    map.insert("gpt-4.1", 1_000_000);
    map.insert("gpt-4-1", 1_000_000);

    // Anthropic models, https://docs.anthropic.com/en/docs/about-claude/models
    map.insert("claude", 200_000);

    // Google models, https://ai.google/get-started/our-models/
    map.insert("gemini-2.5", 1_000_000);
    map.insert("gemini-2-5", 1_000_000);

    // Meta Llama models, https://github.com/meta-llama/llama-models/tree/main?tab=readme-ov-file#llama-models-1
    map.insert("llama3.2", 128_000);
    map.insert("llama3.3", 128_000);

    // x.ai Grok models, https://docs.x.ai/docs/overview
    map.insert("grok", 131_072);

    // Groq models, https://console.groq.com/docs/models
    map.insert("gemma2-9b", 8_192);
    map.insert("kimi-k2", 131_072);
    map.insert("qwen3-32b", 131_072);
    map.insert("grok-3", 131_072);
    map.insert("grok-4", 256_000); // 256K
    map.insert("qwen3-coder", 262_144); // 262K

    map
});

/// Configuration for model-specific settings and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The name of the model to use
    pub model_name: String,
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

/// Struct to represent model pattern matches and their limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimitConfig {
    pub pattern: String,
    pub context_limit: usize,
}

impl ModelConfig {
    /// Create a new ModelConfig with the specified model name
    ///
    /// The context limit is set with the following precedence:
    /// 1. Explicit context_limit if provided in config
    /// 2. Environment variable override (GOOSE_CONTEXT_LIMIT)
    /// 3. Model-specific default based on model name
    /// 4. Global default (128_000) (in get_context_limit)
    pub fn new(model_name: String) -> Self {
        Self::new_with_context_env(model_name, None)
    }

    /// Create a new ModelConfig with the specified model name and custom context limit env var
    ///
    /// This is useful for specific model purposes like lead, worker, planner models
    /// that may have their own context limit environment variables.
    pub fn new_with_context_env(model_name: String, context_env_var: Option<&str>) -> Self {
        let context_limit = Self::get_context_limit_with_env_override(&model_name, context_env_var);

        let toolshim = std::env::var("GOOSE_TOOLSHIM")
            .map(|val| val == "1" || val.to_lowercase() == "true")
            .unwrap_or(false);

        let toolshim_model = std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL").ok();

        let temperature = std::env::var("GOOSE_TEMPERATURE")
            .ok()
            .and_then(|val| val.parse::<f32>().ok());

        Self {
            model_name,
            context_limit,
            temperature,
            max_tokens: None,
            toolshim,
            toolshim_model,
        }
    }

    /// Get model-specific context limit based on model name
    fn get_model_specific_limit(model_name: &str) -> Option<usize> {
        for (pattern, &limit) in MODEL_SPECIFIC_LIMITS.iter() {
            if model_name.contains(pattern) {
                return Some(limit);
            }
        }
        None
    }

    /// Get all model pattern matches and their limits
    pub fn get_all_model_limits() -> Vec<ModelLimitConfig> {
        MODEL_SPECIFIC_LIMITS
            .iter()
            .map(|(&pattern, &context_limit)| ModelLimitConfig {
                pattern: pattern.to_string(),
                context_limit,
            })
            .collect()
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

    /// Get the context_limit for the current model
    /// If none are defined, use the DEFAULT_CONTEXT_LIMIT
    pub fn context_limit(&self) -> usize {
        self.context_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }

    /// Get context limit with environment variable override support
    ///
    /// The context limit is resolved with the following precedence:
    /// 1. Custom environment variable (if specified)
    /// 2. GOOSE_CONTEXT_LIMIT (default environment variable)
    /// 3. Model-specific default based on model name
    /// 4. Global default (128_000)
    fn get_context_limit_with_env_override(
        model_name: &str,
        custom_env_var: Option<&str>,
    ) -> Option<usize> {
        // 1. Check custom environment variable first (e.g., GOOSE_LEAD_CONTEXT_LIMIT)
        if let Some(env_var) = custom_env_var {
            if let Ok(limit_str) = std::env::var(env_var) {
                if let Ok(limit) = limit_str.parse::<usize>() {
                    return Some(limit);
                }
            }
        }

        // 2. Check default context limit environment variable
        if let Ok(limit_str) = std::env::var("GOOSE_CONTEXT_LIMIT") {
            if let Ok(limit) = limit_str.parse::<usize>() {
                return Some(limit);
            }
        }

        // 3. Fall back to model-specific defaults
        Self::get_model_specific_limit(model_name)
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

    #[test]
    fn test_get_all_model_limits() {
        let limits = ModelConfig::get_all_model_limits();
        assert!(!limits.is_empty());

        // Test that we can find specific patterns
        let gpt4_limit = limits.iter().find(|l| l.pattern == "gpt-4o");
        assert!(gpt4_limit.is_some());
        assert_eq!(gpt4_limit.unwrap().context_limit, 128_000);
    }

    #[test]
    #[serial_test::serial]
    fn test_model_config_context_limit_env_vars() {
        use temp_env::with_vars;

        // Test default context limit environment variable
        with_vars([("GOOSE_CONTEXT_LIMIT", Some("250000"))], || {
            let config = ModelConfig::new("unknown-model".to_string());
            assert_eq!(config.context_limit(), 250_000);
        });

        // Test custom context limit environment variable
        with_vars(
            [
                ("GOOSE_LEAD_CONTEXT_LIMIT", Some("300000")),
                ("GOOSE_CONTEXT_LIMIT", Some("250000")),
            ],
            || {
                let config = ModelConfig::new_with_context_env(
                    "unknown-model".to_string(),
                    Some("GOOSE_LEAD_CONTEXT_LIMIT"),
                );
                // Should use the custom env var, not the default one
                assert_eq!(config.context_limit(), 300_000);
            },
        );

        // Test fallback to model-specific when env var is invalid
        with_vars([("GOOSE_CONTEXT_LIMIT", Some("invalid"))], || {
            let config = ModelConfig::new("gpt-4o".to_string());
            assert_eq!(config.context_limit(), 128_000); // Should use model-specific default
        });

        // Test fallback to default when no env vars and unknown model
        let config = ModelConfig::new("unknown-model".to_string());
        assert_eq!(config.context_limit(), DEFAULT_CONTEXT_LIMIT);
    }
}
