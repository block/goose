use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable '{0}' not found")]
    EnvVarMissing(String),
    #[error("Invalid value for '{0}': '{1}' - {2}")]
    InvalidValue(String, String, String),
    #[error("Value for '{0}' is out of valid range: {1}")]
    InvalidRange(String, String),
}

static MODEL_SPECIFIC_LIMITS: Lazy<Vec<(&'static str, usize)>> = Lazy::new(|| {
    vec![
        // openai
        ("gpt-5", 272_000),
        ("gpt-4-turbo", 128_000),
        ("gpt-4.1", 1_000_000),
        ("gpt-4-1", 1_000_000),
        ("gpt-4o", 128_000),
        ("o4-mini", 200_000),
        ("o3-mini", 200_000),
        ("o3", 200_000),
        // anthropic - all 200k
        ("claude", 200_000),
        // google
        ("gemini-1", 128_000),
        ("gemini-2", 1_000_000),
        ("gemma-3-27b", 128_000),
        ("gemma-3-12b", 128_000),
        ("gemma-3-4b", 128_000),
        ("gemma-3-1b", 32_000),
        ("gemma3-27b", 128_000),
        ("gemma3-12b", 128_000),
        ("gemma3-4b", 128_000),
        ("gemma3-1b", 32_000),
        ("gemma-2-27b", 8_192),
        ("gemma-2-9b", 8_192),
        ("gemma-2-2b", 8_192),
        ("gemma2-", 8_192),
        ("gemma-7b", 8_192),
        ("gemma-2b", 8_192),
        ("gemma1", 8_192),
        ("gemma", 8_192),
        // facebook
        ("llama-2-1b", 32_000),
        ("llama", 128_000),
        // qwen
        ("qwen3-coder", 262_144),
        ("qwen2-7b", 128_000),
        ("qwen2-14b", 128_000),
        ("qwen2-32b", 131_072),
        ("qwen2-70b", 262_144),
        ("qwen2", 128_000),
        ("qwen3-32b", 131_072),
        // other
        ("kimi-k2", 131_072),
        ("grok-4", 256_000),
        ("grok", 131_072),
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_name: String,
    pub context_limit: Option<usize>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub toolshim: bool,
    pub toolshim_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimitConfig {
    pub pattern: String,
    pub context_limit: usize,
}

impl ModelConfig {
    pub fn new(model_name: &str) -> Result<Self, ConfigError> {
        Self::new_with_context_env(model_name.to_string(), None)
    }

    /// Create a new ModelConfig with the specified model name and custom context limit env var
    ///
    /// This is useful for specific model purposes like lead, worker, planner models
    /// that may have their own context limit environment variables.
    pub fn new_with_context_env(
        model_name: String,
        context_env_var: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let context_limit = Self::parse_context_limit(&model_name, context_env_var)?;
        let temperature = Self::parse_temperature()?;
        let max_tokens = Self::parse_max_tokens()?;
        let toolshim = Self::parse_toolshim()?;
        let toolshim_model = Self::parse_toolshim_model()?;

        Ok(Self {
            model_name,
            context_limit,
            temperature,
            max_tokens,
            toolshim,
            toolshim_model,
        })
    }

    fn parse_context_limit(
        model_name: &str,
        custom_env_var: Option<&str>,
    ) -> Result<Option<usize>, ConfigError> {
        let config = crate::config::Config::global();

        // Try custom env var first
        if let Some(env_var) = custom_env_var {
            match config.get_param::<String>(env_var) {
                Ok(val) => {
                    let limit = val.parse::<usize>().map_err(|_| {
                        ConfigError::InvalidValue(
                            env_var.to_string(),
                            val.clone(),
                            "must be a positive integer".to_string(),
                        )
                    })?;
                    if limit < 4 * 1024 {
                        return Err(ConfigError::InvalidRange(
                            env_var.to_string(),
                            "must be greater than 4K".to_string(),
                        ));
                    }
                    return Ok(Some(limit));
                }
                Err(crate::config::ConfigError::NotFound(_)) => {
                    // Continue to check default env var
                }
                Err(crate::config::ConfigError::DeserializeError(_)) => {
                    // This might be because the config system parsed it as an integer
                    // Try to get it as an integer directly
                    match config.get_param::<usize>(env_var) {
                        Ok(limit) => {
                            if limit < 4 * 1024 {
                                return Err(ConfigError::InvalidRange(
                                    env_var.to_string(),
                                    "must be greater than 4K".to_string(),
                                ));
                            }
                            return Ok(Some(limit));
                        }
                        Err(_) => {
                            // Still can't parse, return original error
                            return Err(ConfigError::InvalidValue(
                                env_var.to_string(),
                                "invalid".to_string(),
                                "must be a positive integer".to_string(),
                            ));
                        }
                    }
                }
                Err(e) => {
                    return Err(ConfigError::InvalidValue(
                        env_var.to_string(),
                        "unknown".to_string(),
                        format!("config error: {}", e),
                    ));
                }
            }
        }

        // Try default GOOSE_CONTEXT_LIMIT
        match config.get_param::<String>("GOOSE_CONTEXT_LIMIT") {
            Ok(val) => {
                let limit = val.parse::<usize>().map_err(|_| {
                    ConfigError::InvalidValue(
                        "GOOSE_CONTEXT_LIMIT".to_string(),
                        val.clone(),
                        "must be a positive integer".to_string(),
                    )
                })?;
                if limit < 4 * 1024 {
                    return Err(ConfigError::InvalidRange(
                        "GOOSE_CONTEXT_LIMIT".to_string(),
                        "must be greater than 4K".to_string(),
                    ));
                }
                Ok(Some(limit))
            }
            Err(crate::config::ConfigError::NotFound(_)) => {
                // Not found, fall back to model-specific defaults
                Ok(Self::get_model_specific_limit(model_name))
            }
            Err(crate::config::ConfigError::DeserializeError(_)) => {
                // This might be because the config system parsed it as an integer
                // Try to get it as an integer directly
                match config.get_param::<usize>("GOOSE_CONTEXT_LIMIT") {
                    Ok(limit) => {
                        if limit < 4 * 1024 {
                            return Err(ConfigError::InvalidRange(
                                "GOOSE_CONTEXT_LIMIT".to_string(),
                                "must be greater than 4K".to_string(),
                            ));
                        }
                        Ok(Some(limit))
                    }
                    Err(_) => {
                        // Still can't parse, return original error
                        Err(ConfigError::InvalidValue(
                            "GOOSE_CONTEXT_LIMIT".to_string(),
                            "invalid".to_string(),
                            "must be a positive integer".to_string(),
                        ))
                    }
                }
            }
            Err(e) => Err(ConfigError::InvalidValue(
                "GOOSE_CONTEXT_LIMIT".to_string(),
                "unknown".to_string(),
                format!("config error: {}", e),
            )),
        }
    }

    fn parse_temperature() -> Result<Option<f32>, ConfigError> {
        let config = crate::config::Config::global();

        // Try to get as string first to capture the original value for error reporting
        match config.get_param::<String>("GOOSE_TEMPERATURE") {
            Ok(val) => {
                let temp = val.parse::<f32>().map_err(|_| {
                    ConfigError::InvalidValue(
                        "GOOSE_TEMPERATURE".to_string(),
                        val.clone(),
                        "must be a valid number".to_string(),
                    )
                })?;
                if temp < 0.0 {
                    return Err(ConfigError::InvalidRange(
                        "GOOSE_TEMPERATURE".to_string(),
                        val,
                    ));
                }
                Ok(Some(temp))
            }
            Err(crate::config::ConfigError::NotFound(_)) => {
                // Not found is OK, means it's not set
                Ok(None)
            }
            Err(crate::config::ConfigError::DeserializeError(_)) => {
                // This might be because the config system parsed it as a float
                // Try to get it as a float directly
                match config.get_param::<f32>("GOOSE_TEMPERATURE") {
                    Ok(temp) => {
                        if temp < 0.0 {
                            return Err(ConfigError::InvalidRange(
                                "GOOSE_TEMPERATURE".to_string(),
                                temp.to_string(),
                            ));
                        }
                        Ok(Some(temp))
                    }
                    Err(_) => {
                        // Still can't parse, return original error
                        Err(ConfigError::InvalidValue(
                            "GOOSE_TEMPERATURE".to_string(),
                            "invalid".to_string(),
                            "must be a valid number".to_string(),
                        ))
                    }
                }
            }
            Err(e) => {
                // Other config errors (file errors, etc.)
                Err(ConfigError::InvalidValue(
                    "GOOSE_TEMPERATURE".to_string(),
                    "unknown".to_string(),
                    format!("config error: {}", e),
                ))
            }
        }
    }

    fn parse_max_tokens() -> Result<Option<i32>, ConfigError> {
        let config = crate::config::Config::global();

        // First try to get as string to capture the original value
        match config.get_param::<String>("GOOSE_MAX_TOKENS") {
            Ok(val) => {
                let tokens = val.parse::<i32>().map_err(|_| {
                    ConfigError::InvalidValue(
                        "GOOSE_MAX_TOKENS".to_string(),
                        val.clone(),
                        "must be a valid positive integer".to_string(),
                    )
                })?;
                if tokens <= 0 {
                    return Err(ConfigError::InvalidRange(
                        "GOOSE_MAX_TOKENS".to_string(),
                        "must be greater than 0".to_string(),
                    ));
                }
                Ok(Some(tokens))
            }
            Err(crate::config::ConfigError::NotFound(_)) => {
                // Not found is OK, means it's not set
                Ok(None)
            }
            Err(crate::config::ConfigError::DeserializeError(_)) => {
                // This might be because the config system parsed it as an integer
                // Try to get it as an integer directly
                match config.get_param::<i32>("GOOSE_MAX_TOKENS") {
                    Ok(tokens) => {
                        if tokens <= 0 {
                            return Err(ConfigError::InvalidRange(
                                "GOOSE_MAX_TOKENS".to_string(),
                                "must be greater than 0".to_string(),
                            ));
                        }
                        Ok(Some(tokens))
                    }
                    Err(_) => {
                        // Still can't parse, return original error
                        Err(ConfigError::InvalidValue(
                            "GOOSE_MAX_TOKENS".to_string(),
                            "invalid".to_string(),
                            "must be a valid positive integer".to_string(),
                        ))
                    }
                }
            }
            Err(e) => {
                // Other config errors (file errors, etc.)
                Err(ConfigError::InvalidValue(
                    "GOOSE_MAX_TOKENS".to_string(),
                    "unknown".to_string(),
                    format!("config error: {}", e),
                ))
            }
        }
    }

    fn parse_toolshim() -> Result<bool, ConfigError> {
        let config = crate::config::Config::global();

        // First try to get as string for validation
        match config.get_param::<String>("GOOSE_TOOLSHIM") {
            Ok(val) => match val.to_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Ok(true),
                "0" | "false" | "no" | "off" => Ok(false),
                _ => Err(ConfigError::InvalidValue(
                    "GOOSE_TOOLSHIM".to_string(),
                    val,
                    "must be one of: 1, true, yes, on, 0, false, no, off".to_string(),
                )),
            },
            Err(crate::config::ConfigError::NotFound(_)) => {
                // Not found is OK, means it's not set
                Ok(false)
            }
            Err(crate::config::ConfigError::DeserializeError(_)) => {
                // This might be because the config system parsed it as a boolean
                // Try to get it as a boolean directly
                match config.get_param::<bool>("GOOSE_TOOLSHIM") {
                    Ok(val) => Ok(val),
                    Err(_) => {
                        // Still can't parse, return original error
                        Err(ConfigError::InvalidValue(
                            "GOOSE_TOOLSHIM".to_string(),
                            "invalid".to_string(),
                            "must be one of: 1, true, yes, on, 0, false, no, off".to_string(),
                        ))
                    }
                }
            }
            Err(e) => {
                // Other config errors (file errors, etc.)
                Err(ConfigError::InvalidValue(
                    "GOOSE_TOOLSHIM".to_string(),
                    "unknown".to_string(),
                    format!("config error: {}", e),
                ))
            }
        }
    }

    fn parse_toolshim_model() -> Result<Option<String>, ConfigError> {
        let config = crate::config::Config::global();

        match config.get_param::<String>("GOOSE_TOOLSHIM_OLLAMA_MODEL") {
            Ok(val) if val.trim().is_empty() => Err(ConfigError::InvalidValue(
                "GOOSE_TOOLSHIM_OLLAMA_MODEL".to_string(),
                val,
                "cannot be empty if set".to_string(),
            )),
            Ok(val) => Ok(Some(val)),
            Err(_) => Ok(None),
        }
    }

    fn get_model_specific_limit(model_name: &str) -> Option<usize> {
        MODEL_SPECIFIC_LIMITS
            .iter()
            .find(|(pattern, _)| model_name.contains(pattern))
            .map(|(_, limit)| *limit)
    }

    pub fn get_all_model_limits() -> Vec<ModelLimitConfig> {
        MODEL_SPECIFIC_LIMITS
            .iter()
            .map(|(pattern, context_limit)| ModelLimitConfig {
                pattern: pattern.to_string(),
                context_limit: *context_limit,
            })
            .collect()
    }

    pub fn with_context_limit(mut self, limit: Option<usize>) -> Self {
        if limit.is_some() {
            self.context_limit = limit;
        }
        self
    }

    pub fn with_temperature(mut self, temp: Option<f32>) -> Self {
        self.temperature = temp;
        self
    }

    pub fn with_max_tokens(mut self, tokens: Option<i32>) -> Self {
        self.max_tokens = tokens;
        self
    }

    pub fn with_toolshim(mut self, toolshim: bool) -> Self {
        self.toolshim = toolshim;
        self
    }

    pub fn with_toolshim_model(mut self, model: Option<String>) -> Self {
        self.toolshim_model = model;
        self
    }

    pub fn context_limit(&self) -> usize {
        self.context_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }

    pub fn new_or_fail(model_name: &str) -> ModelConfig {
        ModelConfig::new(model_name).unwrap_or_else(|err| {
            // For tests and backwards compatibility, try creating a basic config
            // if validation fails, but log the error
            tracing::warn!(
                "Failed to create validated model config for {}: {}. Creating basic config.",
                model_name,
                err
            );
            ModelConfig {
                model_name: model_name.to_string(),
                context_limit: Self::get_model_specific_limit(model_name),
                temperature: None,
                max_tokens: None,
                toolshim: false,
                toolshim_model: None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use temp_env::with_var;

    #[test]
    #[serial]
    fn test_model_config_context_limits() {
        // Clear all GOOSE environment variables to ensure clean test environment
        with_var("GOOSE_TEMPERATURE", None::<&str>, || {
            with_var("GOOSE_CONTEXT_LIMIT", None::<&str>, || {
                with_var("GOOSE_TOOLSHIM", None::<&str>, || {
                    with_var("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>, || {
                        let config = ModelConfig::new("claude-3-opus")
                            .unwrap()
                            .with_context_limit(Some(150_000));
                        assert_eq!(config.context_limit(), 150_000);

                        let config = ModelConfig::new("claude-3-opus").unwrap();
                        assert_eq!(config.context_limit(), 200_000);

                        let config = ModelConfig::new("gpt-4-turbo").unwrap();
                        assert_eq!(config.context_limit(), 128_000);

                        let config = ModelConfig::new("unknown-model").unwrap();
                        assert_eq!(config.context_limit(), DEFAULT_CONTEXT_LIMIT);
                    });
                });
            });
        });
    }

    #[test]
    #[serial]
    fn test_invalid_context_limit() {
        with_var("GOOSE_CONTEXT_LIMIT", Some("abc"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            if let Err(ConfigError::InvalidValue(var, val, msg)) = result {
                assert_eq!(var, "GOOSE_CONTEXT_LIMIT");
                assert_eq!(val, "abc");
                assert!(msg.contains("positive integer"));
            }
        });

        with_var("GOOSE_CONTEXT_LIMIT", Some("0"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                ConfigError::InvalidRange(_, _)
            ));
        });
    }

    #[test]
    #[serial]
    fn test_invalid_temperature() {
        with_var("GOOSE_TEMPERATURE", Some("hot"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
        });

        with_var("GOOSE_TEMPERATURE", Some("-1.0"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
        });
    }

    #[test]
    #[serial]
    fn test_invalid_toolshim() {
        with_var("GOOSE_TOOLSHIM", Some("maybe"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            if let Err(ConfigError::InvalidValue(var, val, msg)) = result {
                assert_eq!(var, "GOOSE_TOOLSHIM");
                assert_eq!(val, "maybe");
                assert!(msg.contains("must be one of"));
            }
        });
    }

    #[test]
    #[serial]
    fn test_empty_toolshim_model() {
        with_var("GOOSE_TOOLSHIM_OLLAMA_MODEL", Some(""), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                ConfigError::InvalidValue(_, _, _)
            ));
        });

        with_var("GOOSE_TOOLSHIM_OLLAMA_MODEL", Some("   "), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
        });
    }

    #[test]
    #[serial]
    fn test_invalid_max_tokens() {
        with_var("GOOSE_MAX_TOKENS", Some("not_a_number"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            if let Err(ConfigError::InvalidValue(var, val, msg)) = result {
                assert_eq!(var, "GOOSE_MAX_TOKENS");
                assert_eq!(val, "not_a_number");
                assert!(msg.contains("positive integer"));
            }
        });

        with_var("GOOSE_MAX_TOKENS", Some("-1"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            match result.unwrap_err() {
                ConfigError::InvalidRange(var, msg) => {
                    assert_eq!(var, "GOOSE_MAX_TOKENS");
                    assert!(msg.contains("greater than 0"));
                }
                other => {
                    panic!("Expected InvalidRange error, got: {:?}", other);
                }
            }
        });

        with_var("GOOSE_MAX_TOKENS", Some("0"), || {
            let result = ModelConfig::new("test-model");
            assert!(result.is_err());
            match result.unwrap_err() {
                ConfigError::InvalidRange(var, msg) => {
                    assert_eq!(var, "GOOSE_MAX_TOKENS");
                    assert!(msg.contains("greater than 0"));
                }
                other => {
                    panic!("Expected InvalidRange error, got: {:?}", other);
                }
            }
        });
    }

    #[test]
    #[serial]
    fn test_valid_configurations() {
        // Test with environment variables set
        with_var("GOOSE_CONTEXT_LIMIT", Some("50000"), || {
            with_var("GOOSE_TEMPERATURE", Some("0.7"), || {
                with_var("GOOSE_MAX_TOKENS", Some("1000"), || {
                    with_var("GOOSE_TOOLSHIM", Some("true"), || {
                        with_var("GOOSE_TOOLSHIM_OLLAMA_MODEL", Some("llama3"), || {
                            let config = ModelConfig::new("test-model").unwrap();
                            assert_eq!(config.context_limit(), 50_000);
                            assert_eq!(config.temperature, Some(0.7));
                            assert_eq!(config.max_tokens, Some(1000));
                            assert!(config.toolshim);
                            assert_eq!(config.toolshim_model, Some("llama3".to_string()));
                        });
                    });
                });
            });
        });
    }
}
