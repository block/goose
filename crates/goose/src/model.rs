use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

// Attempt to fetch from Git repo first (raw URL), then fall back to local file, then defaults
const DEFAULT_MODEL_LIMITS_YAML_URL: &str =
    "https://raw.githubusercontent.com/block/goose/main/model-limits.yaml";
const DEFAULT_MODEL_LIMITS_JSON_URL: &str =
    "https://raw.githubusercontent.com/block/goose/main/model-limits.json";

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable '{0}' not found")]
    EnvVarMissing(String),
    #[error("Invalid value for '{0}': '{1}' - {2}")]
    InvalidValue(String, String, String),
    #[error("Value for '{0}' is out of valid range: {1}")]
    InvalidRange(String, String),
}

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

#[cfg(test)]
static MODEL_SPECIFIC_LIMITS: Lazy<Vec<ModelLimitConfig>> = Lazy::new(|| {
    // In tests, always use the built-in default list to keep expectations deterministic
    ModelLimitConfig::default_list()
});

#[cfg(not(test))]
static MODEL_SPECIFIC_LIMITS: Lazy<Vec<ModelLimitConfig>> = Lazy::new(|| {
    if let Some(list) = load_model_limits_from_remote() {
        return list;
    }
    ModelLimitConfig::default_list()
});

fn load_model_limits_from_remote() -> Option<Vec<ModelLimitConfig>> {
    let handle = std::thread::spawn(|| {
        let url_env = std::env::var("GOOSE_MODEL_LIMITS_URL").ok();

        let urls: Vec<&str> = match url_env.as_deref() {
            Some(u) => vec![u],
            None => vec![DEFAULT_MODEL_LIMITS_YAML_URL, DEFAULT_MODEL_LIMITS_JSON_URL],
        };

        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("goose/1.0 model-limits-fetcher")
            .build()
        {
            Ok(c) => c,
            Err(_) => return None,
        };

        for url in urls {
            if let Ok(resp) = client.get(url).send() {
                if let Ok(resp) = resp.error_for_status() {
                    if let Ok(text) = resp.text() {
                        if let Ok(list) = serde_yaml::from_str::<Vec<ModelLimitConfig>>(&text) {
                            if !list.is_empty() {
                                return Some(list);
                            }
                        }
                        if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(&text) {
                            if let Ok(json_value) = serde_json::to_value(yaml_value) {
                                if let Some(list) = parse_limits_from_json_value(&json_value) {
                                    if !list.is_empty() {
                                        return Some(list);
                                    }
                                }
                            }
                        }
                        if let Ok(list) = serde_json::from_str::<Vec<ModelLimitConfig>>(&text) {
                            if !list.is_empty() {
                                return Some(list);
                            }
                        }
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(list) = parse_limits_from_json_value(&json_value) {
                                if !list.is_empty() {
                                    return Some(list);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    });

    handle.join().unwrap_or_default()
}

fn parse_limits_from_json_value(v: &serde_json::Value) -> Option<Vec<ModelLimitConfig>> {
    match v {
        serde_json::Value::Array(_) => {
            serde_json::from_value::<Vec<ModelLimitConfig>>(v.clone()).ok()
        }
        serde_json::Value::Object(map) => {
            for key in ["models", "model_limits", "limits"].iter() {
                if let Some(val) = map.get(*key) {
                    if let Ok(list) = serde_json::from_value::<Vec<ModelLimitConfig>>(val.clone()) {
                        return Some(list);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

impl ModelLimitConfig {
    #[allow(clippy::too_many_lines)]
    fn default_list() -> Vec<ModelLimitConfig> {
        vec![
            // openai
            ModelLimitConfig {
                pattern: "gpt-5".to_string(),
                context_limit: 256_000,
            },
            ModelLimitConfig {
                pattern: "gpt-4-turbo".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gpt-4.1".to_string(),
                context_limit: 1_000_000,
            },
            ModelLimitConfig {
                pattern: "gpt-4-1".to_string(),
                context_limit: 1_000_000,
            },
            ModelLimitConfig {
                pattern: "gpt-4o".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "o4-mini".to_string(),
                context_limit: 200_000,
            },
            ModelLimitConfig {
                pattern: "o3-mini".to_string(),
                context_limit: 200_000,
            },
            ModelLimitConfig {
                pattern: "o3".to_string(),
                context_limit: 200_000,
            },
            // anthropic - all 200k
            ModelLimitConfig {
                pattern: "claude".to_string(),
                context_limit: 200_000,
            },
            // google
            ModelLimitConfig {
                pattern: "gemini-1".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemini-2".to_string(),
                context_limit: 1_000_000,
            },
            ModelLimitConfig {
                pattern: "gemma-3-27b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemma-3-12b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemma-3-4b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemma-3-1b".to_string(),
                context_limit: 32_000,
            },
            ModelLimitConfig {
                pattern: "gemma3-27b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemma3-12b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemma3-4b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "gemma3-1b".to_string(),
                context_limit: 32_000,
            },
            ModelLimitConfig {
                pattern: "gemma-2-27b".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma-2-9b".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma-2-2b".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma2-".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma-7b".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma-2b".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma1".to_string(),
                context_limit: 8_192,
            },
            ModelLimitConfig {
                pattern: "gemma".to_string(),
                context_limit: 8_192,
            },
            // facebook
            ModelLimitConfig {
                pattern: "llama-2-1b".to_string(),
                context_limit: 32_000,
            },
            ModelLimitConfig {
                pattern: "llama".to_string(),
                context_limit: 128_000,
            },
            // qwen
            ModelLimitConfig {
                pattern: "qwen3-coder".to_string(),
                context_limit: 262_144,
            },
            ModelLimitConfig {
                pattern: "qwen2-7b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "qwen2-14b".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "qwen2-32b".to_string(),
                context_limit: 131_072,
            },
            ModelLimitConfig {
                pattern: "qwen2-70b".to_string(),
                context_limit: 262_144,
            },
            ModelLimitConfig {
                pattern: "qwen2".to_string(),
                context_limit: 128_000,
            },
            ModelLimitConfig {
                pattern: "qwen3-32b".to_string(),
                context_limit: 131_072,
            },
            // other
            ModelLimitConfig {
                pattern: "kimi-k2".to_string(),
                context_limit: 131_072,
            },
            ModelLimitConfig {
                pattern: "grok-4".to_string(),
                context_limit: 256_000,
            },
            ModelLimitConfig {
                pattern: "grok".to_string(),
                context_limit: 131_072,
            },
        ]
    }
}

impl ModelConfig {
    pub fn new(model_name: &str) -> Result<Self, ConfigError> {
        Self::new_with_context_env(model_name.to_string(), None)
    }

    pub fn new_with_context_env(
        model_name: String,
        context_env_var: Option<&str>,
    ) -> Result<Self, ConfigError> {
        let context_limit = Self::parse_context_limit(&model_name, context_env_var)?;
        let temperature = Self::parse_temperature()?;
        let toolshim = Self::parse_toolshim()?;
        let toolshim_model = Self::parse_toolshim_model()?;

        Ok(Self {
            model_name,
            context_limit,
            temperature,
            max_tokens: None,
            toolshim,
            toolshim_model,
        })
    }

    fn parse_context_limit(
        model_name: &str,
        custom_env_var: Option<&str>,
    ) -> Result<Option<usize>, ConfigError> {
        if let Some(env_var) = custom_env_var {
            if let Ok(val) = std::env::var(env_var) {
                return Self::validate_context_limit(&val, env_var).map(Some);
            }
        }
        if let Ok(val) = std::env::var("GOOSE_CONTEXT_LIMIT") {
            return Self::validate_context_limit(&val, "GOOSE_CONTEXT_LIMIT").map(Some);
        }
        Ok(Self::get_model_specific_limit(model_name))
    }

    fn validate_context_limit(val: &str, env_var: &str) -> Result<usize, ConfigError> {
        let limit = val.parse::<usize>().map_err(|_| {
            ConfigError::InvalidValue(
                env_var.to_string(),
                val.to_string(),
                "must be a positive integer".to_string(),
            )
        })?;

        if limit < 4 * 1024 {
            return Err(ConfigError::InvalidRange(
                env_var.to_string(),
                "must be greater than 4K".to_string(),
            ));
        }

        Ok(limit)
    }

    fn parse_temperature() -> Result<Option<f32>, ConfigError> {
        if let Ok(val) = std::env::var("GOOSE_TEMPERATURE") {
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
        } else {
            Ok(None)
        }
    }

    fn parse_toolshim() -> Result<bool, ConfigError> {
        if let Ok(val) = std::env::var("GOOSE_TOOLSHIM") {
            match val.to_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Ok(true),
                "0" | "false" | "no" | "off" => Ok(false),
                _ => Err(ConfigError::InvalidValue(
                    "GOOSE_TOOLSHIM".to_string(),
                    val,
                    "must be one of: 1, true, yes, on, 0, false, no, off".to_string(),
                )),
            }
        } else {
            Ok(false)
        }
    }

    fn parse_toolshim_model() -> Result<Option<String>, ConfigError> {
        match std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL") {
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
            .find(|ml| model_name.contains(&ml.pattern))
            .map(|ml| ml.context_limit)
    }

    pub fn get_all_model_limits() -> Vec<ModelLimitConfig> {
        MODEL_SPECIFIC_LIMITS.clone()
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
        ModelConfig::new(model_name)
            .unwrap_or_else(|_| panic!("Failed to create model config for {}", model_name))
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
    fn test_valid_configurations() {
        // Test with environment variables set
        with_var("GOOSE_CONTEXT_LIMIT", Some("50000"), || {
            with_var("GOOSE_TEMPERATURE", Some("0.7"), || {
                with_var("GOOSE_TOOLSHIM", Some("true"), || {
                    with_var("GOOSE_TOOLSHIM_OLLAMA_MODEL", Some("llama3"), || {
                        let config = ModelConfig::new("test-model").unwrap();
                        assert_eq!(config.context_limit(), 50_000);
                        assert_eq!(config.temperature, Some(0.7));
                        assert!(config.toolshim);
                        assert_eq!(config.toolshim_model, Some("llama3".to_string()));
                    });
                });
            });
        });
    }
}
