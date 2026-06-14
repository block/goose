use crate::formats::openai::{extract_reasoning_effort, is_openai_responses_model};
use crate::thinking::ThinkingEffort;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;
use utoipa::ToSchema;

pub const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Environment variable '{0}' not found")]
    EnvVarMissing(String),
    #[error("Invalid value for '{0}': '{1}' - {2}")]
    InvalidValue(String, String, String),
    #[error("Value for '{0}' is out of valid range: {1}")]
    InvalidRange(String, String),
}

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct ModelConfig {
    pub model_name: String,
    pub context_limit: Option<usize>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub toolshim: bool,
    pub toolshim_model: Option<String>,
    #[serde(skip)]
    pub fast_model_config: Option<Box<ModelConfig>>,
    /// Provider-specific request parameters (e.g., anthropic_beta headers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_params: Option<HashMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<bool>,
}

impl<'de> Deserialize<'de> for ModelConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawModelConfig {
            model_name: String,
            context_limit: Option<usize>,
            temperature: Option<f32>,
            max_tokens: Option<i32>,
            toolshim: bool,
            toolshim_model: Option<String>,
            #[serde(default)]
            fast_model_config: Option<Box<ModelConfig>>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            request_params: Option<HashMap<String, Value>>,
            #[serde(default, skip_serializing_if = "Option::is_none")]
            reasoning: Option<bool>,
        }

        let raw = RawModelConfig::deserialize(deserializer)?;
        let mut config = Self {
            model_name: raw.model_name,
            context_limit: raw.context_limit,
            temperature: raw.temperature,
            max_tokens: raw.max_tokens,
            toolshim: raw.toolshim,
            toolshim_model: raw.toolshim_model,
            fast_model_config: raw.fast_model_config,
            request_params: raw.request_params,
            reasoning: raw.reasoning,
        };
        config.normalize_effort_suffix();
        Ok(config)
    }
}

impl ModelConfig {
    pub fn new(model_name: impl AsRef<str>) -> Result<Self, ConfigError> {
        let mut config = Self {
            model_name: model_name.as_ref().to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            fast_model_config: None,
            request_params: None,
            reasoning: None,
        };
        config.normalize_effort_suffix();
        Ok(config)
    }

    pub fn with_canonical_limits(mut self, provider_name: &str) -> Self {
        // Try canonical lookup with the full model name first, then fall back
        // to the name with reasoning-effort suffixes stripped (e.g.
        // "databricks-gpt-5.4-high" → "databricks-gpt-5.4").
        let canonical =
            crate::canonical::maybe_get_canonical_model(provider_name, &self.model_name).or_else(
                || {
                    let (base, _effort) = extract_reasoning_effort(&self.model_name);
                    if base != self.model_name {
                        crate::canonical::maybe_get_canonical_model(provider_name, &base)
                    } else {
                        None
                    }
                },
            );

        if let Some(canonical) = canonical {
            if self.context_limit.is_none() {
                self.context_limit = Some(canonical.limit.context);
            }
            if self.max_tokens.is_none() {
                self.max_tokens = canonical
                    .limit
                    .output
                    .filter(|&output| output < canonical.limit.context)
                    .map(|output| output as i32);
            }
            if self.reasoning.is_none() {
                self.reasoning = canonical.reasoning;
            }
        }

        self
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

    pub fn with_default_context_limit(mut self, limit: Option<usize>) -> Self {
        if self.context_limit.is_none() {
            self.context_limit = limit;
        }

        if let Some(fast_config) = self.fast_model_config.take() {
            self.fast_model_config = Some(Box::new(fast_config.with_default_context_limit(limit)));
        }

        self
    }

    pub fn with_default_max_tokens(mut self, tokens: Option<i32>) -> Self {
        if self.max_tokens.is_none() {
            self.max_tokens = tokens;
        }

        if let Some(fast_config) = self.fast_model_config.take() {
            self.fast_model_config = Some(Box::new(fast_config.with_default_max_tokens(tokens)));
        }

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

    pub fn with_fast(
        mut self,
        fast_model_name: &str,
        provider_name: &str,
    ) -> Result<Self, ConfigError> {
        let fast_config = ModelConfig::new(fast_model_name)?.with_canonical_limits(provider_name);
        self.fast_model_config = Some(Box::new(fast_config));
        Ok(self)
    }

    pub fn with_fast_model_config(mut self, fast_model_config: ModelConfig) -> Self {
        self.fast_model_config = Some(Box::new(fast_model_config));
        self
    }

    pub fn with_merged_request_params(mut self, params: HashMap<String, Value>) -> Self {
        match self.request_params.as_mut() {
            Some(existing) => {
                for (k, v) in params {
                    existing.insert(k, v);
                }
            }
            None => {
                self.request_params = Some(params);
            }
        }
        self
    }

    pub fn with_thinking_effort(mut self, effort: ThinkingEffort) -> Self {
        let params = self.request_params.get_or_insert_with(HashMap::new);
        params.insert(
            "thinking_effort".to_string(),
            serde_json::json!(effort.to_string()),
        );
        self
    }

    pub fn with_default_thinking_effort(mut self, effort: Option<ThinkingEffort>) -> Self {
        if self.thinking_effort().is_none() {
            if let Some(effort) = effort {
                self = self.with_thinking_effort(effort);
            }
        }

        if let Some(fast_config) = self.fast_model_config.take() {
            self.fast_model_config =
                Some(Box::new(fast_config.with_default_thinking_effort(effort)));
        }

        self
    }

    pub fn with_inherited_session_settings_from(
        mut self,
        previous: Option<&ModelConfig>,
        request_params: Option<HashMap<String, Value>>,
    ) -> Self {
        if let Some(previous) = previous {
            let has_thinking_effort = self
                .request_params
                .as_ref()
                .and_then(|params| params.get("thinking_effort"))
                .is_some();

            if !has_thinking_effort {
                if let Some(thinking_effort) = previous
                    .request_params
                    .as_ref()
                    .and_then(|params| params.get("thinking_effort"))
                    .cloned()
                {
                    let params = self.request_params.get_or_insert_with(HashMap::new);
                    params.insert("thinking_effort".to_string(), thinking_effort);
                }
            }
        }

        if let Some(request_params) = request_params {
            self = self.with_merged_request_params(request_params);
        }

        self
    }

    pub fn use_fast_model(&self) -> Self {
        if let Some(fast_config) = &self.fast_model_config {
            *fast_config.clone()
        } else {
            self.clone()
        }
    }

    pub fn context_limit(&self) -> usize {
        self.context_limit.unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }

    pub fn is_openai_reasoning_model(&self) -> bool {
        is_openai_responses_model(&self.model_name)
    }

    pub fn is_reasoning_model(&self) -> bool {
        if let Some(reasoning) = self.reasoning {
            return reasoning;
        }

        self.is_openai_reasoning_model()
            || self.model_name.to_lowercase().contains("claude")
            || Self::is_gemini3_reasoning_model_name(&self.model_name)
    }

    fn is_gemini3_reasoning_model_name(model_name: &str) -> bool {
        let lower = model_name.to_lowercase();
        lower.starts_with("gemini-3") || lower.contains("/gemini-3") || lower.contains("-gemini-3")
    }

    pub fn max_output_tokens(&self) -> i32 {
        if let Some(tokens) = self.max_tokens {
            return tokens;
        }

        4_096
    }

    pub fn normalize_effort_suffix(&mut self) {
        if !self.is_openai_reasoning_model() {
            return;
        }
        let parts: Vec<&str> = self.model_name.split('-').collect();
        let last = match parts.last() {
            Some(l) => *l,
            None => return,
        };
        let effort = match last {
            "none" => ThinkingEffort::Off,
            "low" => ThinkingEffort::Low,
            "medium" => ThinkingEffort::Medium,
            "high" => ThinkingEffort::High,
            "xhigh" => ThinkingEffort::Max,
            _ => return,
        };
        self.model_name = parts[..parts.len() - 1].join("-");
        let has_explicit_effort = self
            .request_params
            .as_ref()
            .and_then(|p| p.get("thinking_effort"))
            .is_some();
        if !has_explicit_effort {
            let params = self.request_params.get_or_insert_with(HashMap::new);
            params.insert(
                "thinking_effort".to_string(),
                serde_json::json!(effort.to_string()),
            );
        }
    }

    pub fn thinking_effort(&self) -> Option<ThinkingEffort> {
        self.request_param::<String>("thinking_effort")
            .and_then(|s| s.parse::<ThinkingEffort>().ok())
    }

    pub fn request_param<T: for<'de> serde::Deserialize<'de>>(
        &self,
        request_key: &str,
    ) -> Option<T> {
        self.request_params
            .as_ref()
            .and_then(|params| params.get(request_key))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn new_or_fail(model_name: &str) -> ModelConfig {
        ModelConfig::new(model_name)
            .unwrap_or_else(|_| panic!("Failed to create model config for {}", model_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_preserves_fast_model_config() {
        let config: ModelConfig = serde_json::from_value(serde_json::json!({
            "model_name": "primary-model",
            "context_limit": null,
            "temperature": null,
            "max_tokens": null,
            "toolshim": false,
            "toolshim_model": null,
            "fast_model_config": {
                "model_name": "fast-model",
                "context_limit": 4096,
                "temperature": null,
                "max_tokens": 1024,
                "toolshim": false,
                "toolshim_model": null
            }
        }))
        .unwrap();

        let fast_config = config.fast_model_config.as_ref().unwrap();
        assert_eq!(fast_config.model_name, "fast-model");
        assert_eq!(fast_config.context_limit, Some(4096));
        assert_eq!(fast_config.max_tokens, Some(1024));
        assert_eq!(config.use_fast_model().model_name, "fast-model");
    }

    mod thinking_effort_tests {
        use super::*;

        #[test]
        fn from_request_params() {
            let mut params = HashMap::new();
            params.insert("thinking_effort".to_string(), serde_json::json!("medium"));
            let config = ModelConfig {
                model_name: "test".to_string(),
                request_params: Some(params),
                ..Default::default()
            };
            assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Medium));
        }

        #[test]
        fn with_thinking_effort_sets_request_param() {
            let config = ModelConfig {
                model_name: "test".to_string(),
                ..Default::default()
            }
            .with_thinking_effort(ThinkingEffort::High);

            assert_eq!(
                config
                    .request_params
                    .as_ref()
                    .and_then(|params| params.get("thinking_effort")),
                Some(&serde_json::json!("high"))
            );
        }

        #[test]
        fn preserves_explicit_thinking_effort() {
            let previous = ModelConfig {
                model_name: "previous".to_string(),
                request_params: Some(HashMap::from([(
                    "thinking_effort".to_string(),
                    serde_json::json!("high"),
                )])),
                ..Default::default()
            };
            let config = ModelConfig {
                model_name: "next".to_string(),
                ..Default::default()
            }
            .with_inherited_session_settings_from(Some(&previous), None);

            assert_eq!(
                config
                    .request_params
                    .as_ref()
                    .and_then(|params| params.get("thinking_effort")),
                Some(&serde_json::json!("high"))
            );
        }

        #[test]
        fn does_not_override_existing_thinking_effort() {
            let previous = ModelConfig {
                model_name: "previous".to_string(),
                request_params: Some(HashMap::from([(
                    "thinking_effort".to_string(),
                    serde_json::json!("high"),
                )])),
                ..Default::default()
            };
            let config = ModelConfig {
                model_name: "next".to_string(),
                request_params: Some(HashMap::from([(
                    "thinking_effort".to_string(),
                    serde_json::json!("low"),
                )])),
                ..Default::default()
            }
            .with_inherited_session_settings_from(Some(&previous), None);

            assert_eq!(
                config
                    .request_params
                    .as_ref()
                    .and_then(|params| params.get("thinking_effort")),
                Some(&serde_json::json!("low"))
            );
        }

        #[test]
        fn does_not_preserve_unrelated_request_params() {
            let previous = ModelConfig {
                model_name: "previous".to_string(),
                request_params: Some(HashMap::from([(
                    "provider_specific".to_string(),
                    serde_json::json!("old"),
                )])),
                ..Default::default()
            };
            let config = ModelConfig {
                model_name: "next".to_string(),
                ..Default::default()
            }
            .with_inherited_session_settings_from(Some(&previous), None);

            assert!(config.request_params.is_none());
        }

        #[test]
        fn explicit_request_params_override_preserved_session_settings() {
            let previous = ModelConfig {
                model_name: "previous".to_string(),
                request_params: Some(HashMap::from([(
                    "thinking_effort".to_string(),
                    serde_json::json!("high"),
                )])),
                ..Default::default()
            };
            let config = ModelConfig {
                model_name: "next".to_string(),
                ..Default::default()
            }
            .with_inherited_session_settings_from(
                Some(&previous),
                Some(HashMap::from([(
                    "thinking_effort".to_string(),
                    serde_json::json!("low"),
                )])),
            );

            assert_eq!(
                config
                    .request_params
                    .as_ref()
                    .and_then(|params| params.get("thinking_effort")),
                Some(&serde_json::json!("low"))
            );
        }

        #[test]
        fn effort_suffix_stripped_from_model_name() {
            let _guard = env_lock::lock_env([
                ("GOOSE_THINKING_EFFORT", None::<&str>),
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_TEMPERATURE", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
                ("GOOSE_TOOLSHIM", None::<&str>),
                ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ]);
            let config = ModelConfig::new("o3-mini-high").unwrap();
            assert_eq!(config.model_name, "o3-mini");
            assert_eq!(config.thinking_effort(), Some(ThinkingEffort::High));
        }

        #[test]
        fn none_suffix_stripped_from_model_name() {
            let _guard = env_lock::lock_env([
                ("GOOSE_THINKING_EFFORT", Some("high")),
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_TEMPERATURE", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
                ("GOOSE_TOOLSHIM", None::<&str>),
                ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ]);
            let config = ModelConfig::new("o3-mini-none").unwrap();
            assert_eq!(config.model_name, "o3-mini");
            assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Off));
        }

        #[test]
        fn xhigh_suffix_stripped_from_model_name() {
            let _guard = env_lock::lock_env([
                ("GOOSE_THINKING_EFFORT", Some("low")),
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_TEMPERATURE", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
                ("GOOSE_TOOLSHIM", None::<&str>),
                ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ]);
            let config = ModelConfig::new("gpt-5.4-xhigh").unwrap();
            assert_eq!(config.model_name, "gpt-5.4");
            assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Max));
        }

        #[test]
        fn effort_suffix_not_stripped_when_thinking_effort_set() {
            let _guard = env_lock::lock_env([
                ("GOOSE_THINKING_EFFORT", None::<&str>),
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_TEMPERATURE", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
                ("GOOSE_TOOLSHIM", None::<&str>),
                ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ]);
            let mut params = HashMap::new();
            params.insert("thinking_effort".to_string(), serde_json::json!("low"));
            let mut config = ModelConfig::new("o3-mini-high").unwrap();
            // Suffix was already normalized during new(), but if request_params
            // were set before construction, the suffix would not be stripped.
            // Verify the normalized state:
            assert_eq!(config.model_name, "o3-mini");

            // Now simulate setting explicit effort after construction
            config.request_params = Some(params);
            assert_eq!(config.thinking_effort(), Some(ThinkingEffort::Low));
        }

        #[test]
        fn no_suffix_no_change() {
            let _guard = env_lock::lock_env([
                ("GOOSE_THINKING_EFFORT", None::<&str>),
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_TEMPERATURE", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
                ("GOOSE_TOOLSHIM", None::<&str>),
                ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ]);
            let config = ModelConfig::new("o3-mini").unwrap();
            assert_eq!(config.model_name, "o3-mini");
        }

        #[test]
        fn non_reasoning_model_suffix_not_stripped() {
            let _guard = env_lock::lock_env([
                ("GOOSE_THINKING_EFFORT", None::<&str>),
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_TEMPERATURE", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
                ("GOOSE_TOOLSHIM", None::<&str>),
                ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None::<&str>),
            ]);
            let config = ModelConfig::new("claude-sonnet-4-high").unwrap();
            assert_eq!(config.model_name, "claude-sonnet-4-high");
        }

        #[test]
        fn parse_aliases() {
            assert_eq!("off".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Off));
            assert_eq!(
                "disabled".parse::<ThinkingEffort>(),
                Ok(ThinkingEffort::Off)
            );
            assert_eq!("med".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Medium));
            assert_eq!("max".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Max));
            assert_eq!("xhigh".parse::<ThinkingEffort>(), Ok(ThinkingEffort::Max));
            assert!("invalid".parse::<ThinkingEffort>().is_err());
        }
    }

    mod with_canonical_limits {
        use super::*;

        #[test]
        fn sets_limits_from_canonical_model() {
            let _guard = env_lock::lock_env([
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ]);
            let config = ModelConfig::new_or_fail("gpt-4o").with_canonical_limits("openai");

            assert_eq!(config.context_limit, Some(128_000));
            assert_eq!(config.max_tokens, Some(16_384));
            assert_eq!(config.reasoning, Some(false));
        }

        #[test]
        fn does_not_override_existing_context_limit() {
            let _guard = env_lock::lock_env([
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ]);
            let mut config = ModelConfig::new_or_fail("gpt-4o");
            config.context_limit = Some(64_000);
            let config = config.with_canonical_limits("openai");

            assert_eq!(config.context_limit, Some(64_000));
        }

        #[test]
        fn does_not_override_existing_max_tokens() {
            let _guard = env_lock::lock_env([
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ]);
            let mut config = ModelConfig::new_or_fail("gpt-4o");
            config.max_tokens = Some(1_000);
            let config = config.with_canonical_limits("openai");

            assert_eq!(config.max_tokens, Some(1_000));
        }

        #[test]
        fn skips_canonical_output_limit_when_it_equals_context_limit() {
            let _guard = env_lock::lock_env([
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ]);
            let config =
                ModelConfig::new_or_fail("moonshotai/kimi-k2.6").with_canonical_limits("nvidia");

            assert_eq!(config.context_limit, Some(262_144));
            assert_eq!(config.max_tokens, None);
            assert_eq!(config.max_output_tokens(), 4_096);
        }

        #[test]
        fn unknown_model_leaves_fields_none() {
            let _guard = env_lock::lock_env([
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ]);
            let config =
                ModelConfig::new_or_fail("totally-unknown-model").with_canonical_limits("openai");

            assert_eq!(config.context_limit, None);
            assert_eq!(config.max_tokens, None);
            assert_eq!(config.reasoning, None);
        }

        #[test]
        fn resolves_after_stripping_reasoning_effort_suffix() {
            let _guard = env_lock::lock_env([
                ("GOOSE_MAX_TOKENS", None::<&str>),
                ("GOOSE_CONTEXT_LIMIT", None::<&str>),
            ]);

            // "databricks-gpt-5.4-high" should resolve via "databricks-gpt-5.4"
            let config = ModelConfig::new_or_fail("databricks-gpt-5.4-high")
                .with_canonical_limits("databricks");
            assert_eq!(config.context_limit, Some(1_050_000));

            // "gpt-5.4-xhigh" should resolve via "gpt-5.4"
            let config = ModelConfig::new_or_fail("gpt-5.4-xhigh").with_canonical_limits("openai");
            assert_eq!(config.context_limit, Some(1_050_000));

            // "gpt-5.4-nano-low" should resolve via "gpt-5.4-nano"
            let config =
                ModelConfig::new_or_fail("gpt-5.4-nano-low").with_canonical_limits("openai");
            assert_eq!(config.context_limit, Some(400_000));
        }
    }

    mod is_openai_reasoning_model {
        use super::*;

        const ENV_LOCK_KEYS: [(&str, Option<&str>); 5] = [
            ("GOOSE_MAX_TOKENS", None),
            ("GOOSE_TEMPERATURE", None),
            ("GOOSE_CONTEXT_LIMIT", None),
            ("GOOSE_TOOLSHIM", None),
            ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None),
        ];

        #[test]
        fn bare_reasoning_models() {
            let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
            assert!(ModelConfig::new_or_fail("o1").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("o1-preview").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("o3").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("o3-mini").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("o4-mini").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("gpt-5").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("gpt-5-3-codex").is_openai_reasoning_model());
        }

        #[test]
        fn goose_prefixed_reasoning_models() {
            let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
            assert!(ModelConfig::new_or_fail("goose-o3-mini").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("goose-o4-mini").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("goose-gpt-5").is_openai_reasoning_model());
        }

        #[test]
        fn databricks_prefixed_reasoning_models() {
            let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
            assert!(ModelConfig::new_or_fail("databricks-o3-mini").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("databricks-o4-mini").is_openai_reasoning_model());
            assert!(ModelConfig::new_or_fail("databricks-gpt-5").is_openai_reasoning_model());
        }

        #[test]
        fn non_reasoning_models() {
            let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
            assert!(!ModelConfig::new_or_fail("claude-sonnet-4").is_openai_reasoning_model());
            assert!(!ModelConfig::new_or_fail("gpt-4o").is_openai_reasoning_model());
            assert!(
                !ModelConfig::new_or_fail("databricks-claude-sonnet-4").is_openai_reasoning_model()
            );
            assert!(!ModelConfig::new_or_fail("goose-claude-sonnet-4").is_openai_reasoning_model());
            assert!(!ModelConfig::new_or_fail("llama-3-70b").is_openai_reasoning_model());
        }
    }

    mod is_reasoning_model {
        use super::*;

        const ENV_LOCK_KEYS: [(&str, Option<&str>); 5] = [
            ("GOOSE_MAX_TOKENS", None),
            ("GOOSE_TEMPERATURE", None),
            ("GOOSE_CONTEXT_LIMIT", None),
            ("GOOSE_TOOLSHIM", None),
            ("GOOSE_TOOLSHIM_OLLAMA_MODEL", None),
        ];

        #[test]
        fn includes_reasoning_model_families() {
            let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
            assert!(ModelConfig::new_or_fail("o3-mini").is_reasoning_model());
            assert!(ModelConfig::new_or_fail("claude-sonnet-4").is_reasoning_model());
            assert!(ModelConfig::new_or_fail("gemini-3-pro").is_reasoning_model());
        }

        #[test]
        fn uses_explicit_metadata_first() {
            let _guard = env_lock::lock_env(ENV_LOCK_KEYS);
            let mut config = ModelConfig::new_or_fail("provider-alias");
            config.reasoning = Some(true);
            assert!(config.is_reasoning_model());

            let mut config = ModelConfig::new_or_fail("claude-sonnet-4");
            config.reasoning = Some(false);
            assert!(!config.is_reasoning_model());
        }
    }
}
