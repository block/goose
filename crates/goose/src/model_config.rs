use crate::config::{Config, ConfigError};
use crate::conversation::message::Message;
use crate::providers::base::Provider;
use anyhow::{anyhow, Result};
use goose_providers::conversation::token_usage::ProviderUsage;
use goose_providers::errors::ProviderError;
use goose_providers::model::ModelConfig;
use goose_providers::thinking::ThinkingEffort;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;

pub fn model_config_from_user_config(
    provider_name: &str,
    model_name: impl AsRef<str>,
) -> Result<ModelConfig> {
    let model = base_model_config_from_user_config(model_name.as_ref())?;
    materialize_model_config(provider_name, model)
}

pub fn model_config_from_user_config_with_session_settings(
    provider_name: &str,
    model_name: impl AsRef<str>,
    previous: Option<&ModelConfig>,
    request_params: Option<HashMap<String, Value>>,
    context_limit: Option<usize>,
) -> Result<ModelConfig> {
    let config = Config::global();
    let model = base_model_config_from_user_config(model_name.as_ref())?;
    let model = materialize_model_config_inner(model, provider_name, false)?
        .with_context_limit(context_limit)
        .with_inherited_session_settings_from(previous, request_params)
        .with_default_thinking_effort(config.get_goose_thinking_effort());

    Ok(model.with_canonical_limits(provider_name))
}

pub fn materialize_model_config(provider_name: &str, model: ModelConfig) -> Result<ModelConfig> {
    let model = materialize_model_config_inner(model, provider_name, true)?;
    Ok(model.with_canonical_limits(provider_name))
}

fn materialize_model_config_inner(
    mut model: ModelConfig,
    provider_name: &str,
    include_default_thinking_effort: bool,
) -> Result<ModelConfig> {
    let config = Config::global();

    if model.temperature.is_none() {
        model = model.with_temperature(get_goose_temperature(config)?);
    }

    if model.toolshim && model.toolshim_model.is_none() {
        model = model.with_toolshim_model(get_goose_toolshim_model(config)?);
    }

    model = model
        .with_default_context_limit(config.get_goose_context_limit()?)
        .with_default_max_tokens(config.get_goose_max_tokens()?);

    if include_default_thinking_effort {
        model = model.with_default_thinking_effort(config.get_goose_thinking_effort());
    }

    if provider_name == goose_providers::openai::OPEN_AI_PROVIDER_NAME {
        model = apply_openai_request_params(model);
    }

    Ok(model)
}

fn configured_model_name(key: &str) -> Option<String> {
    Config::global()
        .get_param::<String>(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn configured_fast_model_name() -> Option<String> {
    configured_model_name("GOOSE_FAST_MODEL")
}

fn configured_compaction_model_name() -> Option<String> {
    configured_compaction_model_name_from(Config::global())
}

fn configured_compaction_model_name_from(config: &Config) -> Option<String> {
    match config.get_param::<String>("GOOSE_COMPACTION_MODEL") {
        Ok(value) => (!value.trim().is_empty()).then(|| value.trim().to_string()),
        Err(ConfigError::NotFound(_)) => None,
        Err(error) => {
            tracing::warn!("Ignoring unreadable GOOSE_COMPACTION_MODEL value: {error}");
            None
        }
    }
}

/// Model for lightweight auxiliary tasks (session naming, tool-call labels).
/// Resolution order:
///   1. `GOOSE_FAST_MODEL`
///   2. the provider's declared default fast model
///   3. the supplied `model_config` (i.e. the main model)
pub async fn get_fast_model(
    provider_name: &str,
    model_config: &ModelConfig,
) -> Result<ModelConfig> {
    resolve_lightweight_model(provider_name, model_config, configured_fast_model_name()).await
}

/// Model that performs compaction and tool-pair summarization. Resolution
/// order is `get_fast_model`'s with a compaction-specific override in front:
///   1. `GOOSE_COMPACTION_MODEL`
///   2. `GOOSE_FAST_MODEL`
///   3. the provider's declared default fast model
///   4. the supplied `model_config` (i.e. the main model)
pub async fn get_compaction_model(
    provider_name: &str,
    model_config: &ModelConfig,
) -> Result<ModelConfig> {
    let override_name = configured_compaction_model_name().or_else(configured_fast_model_name);
    resolve_lightweight_model(provider_name, model_config, override_name).await
}

async fn resolve_lightweight_model(
    provider_name: &str,
    model_config: &ModelConfig,
    override_name: Option<String>,
) -> Result<ModelConfig> {
    let model_name = match override_name {
        Some(name) => Some(name),
        None => provider_default_fast_model(provider_name).await,
    };

    match model_name {
        Some(name) if name != model_config.model_name => {
            model_config_from_user_config(provider_name, name)
                .map(|config| config.with_request_headers(model_config.request_headers.clone()))
        }
        _ => Ok(model_config.clone()),
    }
}

struct CompletionRequest<'a> {
    configured_model_key: Option<&'static str>,
    session_id: &'a str,
    system: &'a str,
    messages: &'a [Message],
    tools: &'a [Tool],
}

pub async fn complete_fast(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<(Message, ProviderUsage), ProviderError> {
    let resolved = get_fast_model(provider.get_name(), model_config).await;
    complete_with_model(
        provider,
        model_config,
        resolved,
        CompletionRequest {
            configured_model_key: None,
            session_id,
            system,
            messages,
            tools,
        },
    )
    .await
}

pub async fn complete_compaction(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<(Message, ProviderUsage), ProviderError> {
    let configured_model_key = configured_compaction_model_name().map(|_| "GOOSE_COMPACTION_MODEL");
    let resolved = get_compaction_model(provider.get_name(), model_config).await;
    complete_with_model(
        provider,
        model_config,
        resolved,
        CompletionRequest {
            configured_model_key,
            session_id,
            system,
            messages,
            tools,
        },
    )
    .await
}

async fn complete_with_model(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    resolved: Result<ModelConfig>,
    request: CompletionRequest<'_>,
) -> Result<(Message, ProviderUsage), ProviderError> {
    let resolved_config = resolved
        .map_err(|e| ProviderError::ExecutionError(e.to_string()))?
        .with_thinking_effort(ThinkingEffort::Off);

    match crate::session_context::with_session_id(
        Some(request.session_id.to_string()),
        provider.complete(
            &resolved_config,
            request.system,
            request.messages,
            request.tools,
        ),
    )
    .await
    {
        Ok(response) => Ok(response),
        Err(e) if resolved_config.model_name != model_config.model_name => {
            match request.configured_model_key {
                Some(key) => tracing::warn!(
                    "Model {} (set via {}) failed with error: {}. Falling back to main model {}",
                    resolved_config.model_name,
                    key,
                    e,
                    model_config.model_name
                ),
                None => tracing::warn!(
                    "Model {} failed with error: {}. Falling back to main model {}",
                    resolved_config.model_name,
                    e,
                    model_config.model_name
                ),
            }
            let fallback_config = model_config
                .clone()
                .with_thinking_effort(ThinkingEffort::Off);
            crate::session_context::with_session_id(
                Some(request.session_id.to_string()),
                provider.complete(
                    &fallback_config,
                    request.system,
                    request.messages,
                    request.tools,
                ),
            )
            .await
        }
        Err(e) => Err(e),
    }
}

async fn provider_default_fast_model(provider_name: &str) -> Option<String> {
    if provider_name == goose_providers::openai::OPEN_AI_PROVIDER_NAME {
        return crate::providers::openai_def::live_fast_model();
    }

    crate::providers::get_from_registry(provider_name)
        .await
        .ok()
        .and_then(|entry| entry.metadata().fast_model.clone())
}

fn apply_openai_request_params(mut model: ModelConfig) -> ModelConfig {
    let config = Config::global();
    if let Some(store) = config.get_openai_store() {
        model = model.with_merged_request_params(HashMap::from([(
            "store".to_string(),
            serde_json::json!(store),
        )]));
    }
    model
}

fn base_model_config_from_user_config(model_name: &str) -> Result<ModelConfig> {
    let config = Config::global();
    let mut model = ModelConfig {
        model_name: model_name.to_string(),
        context_limit: None,
        temperature: get_goose_temperature(config)?,
        max_tokens: None,
        toolshim: get_goose_toolshim(config)?.unwrap_or(false),
        toolshim_model: get_goose_toolshim_model(config)?,
        request_params: None,
        reasoning: None,
        request_headers: None,
    };
    model.normalize_effort_suffix();
    Ok(model)
}

fn get_goose_temperature(config: &Config) -> Result<Option<f32>> {
    match config.get_param::<f32>("GOOSE_TEMPERATURE") {
        Ok(temp) if temp < 0.0 => Err(anyhow!(
            "Value for 'GOOSE_TEMPERATURE' is out of valid range: {temp}"
        )),
        Ok(temp) => Ok(Some(temp)),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn get_goose_toolshim(config: &Config) -> Result<Option<bool>> {
    match config.get_param::<serde_yaml::Value>("GOOSE_TOOLSHIM") {
        Ok(value) => parse_yaml_bool_config("GOOSE_TOOLSHIM", value).map(Some),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Resolve the global toolshim setting, defaulting to false when unset.
pub fn global_toolshim() -> bool {
    get_goose_toolshim(Config::global())
        .ok()
        .flatten()
        .unwrap_or(false)
}

fn get_goose_toolshim_model(config: &Config) -> Result<Option<String>> {
    match config.get_param::<String>("GOOSE_TOOLSHIM_OLLAMA_MODEL") {
        Ok(value) if value.trim().is_empty() => Err(anyhow!(
            "Invalid value for 'GOOSE_TOOLSHIM_OLLAMA_MODEL': '{value}' - cannot be empty if set"
        )),
        Ok(value) => Ok(Some(value)),
        Err(ConfigError::NotFound(_)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn parse_bool_config(key: &str, value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(anyhow!(
            "Invalid value for '{key}': '{value}' - must be one of: 1, true, yes, on, 0, false, no, off"
        )),
    }
}

fn parse_yaml_bool_config(key: &str, value: serde_yaml::Value) -> Result<bool> {
    match value {
        serde_yaml::Value::Bool(value) => Ok(value),
        serde_yaml::Value::Number(value) => parse_bool_config(key, &value.to_string()),
        serde_yaml::Value::String(value) => parse_bool_config(key, &value),
        other => {
            Err(anyhow!(
            "Invalid value for '{key}': '{}' - must be one of: 1, true, yes, on, 0, false, no, off",
            serde_yaml::to_string(&other).unwrap_or_else(|_| "<unprintable>".to_string()).trim()
        ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::base::{stream_from_single_message, MessageStream};
    use async_trait::async_trait;
    use goose_providers::conversation::token_usage::Usage;
    use std::sync::Mutex;

    fn test_config() -> (tempfile::TempDir, Config) {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = Config::new_with_file_secrets(
            temp_dir.path().join("config.yaml"),
            temp_dir.path().join("secrets.yaml"),
        )
        .unwrap();

        (temp_dir, config)
    }

    #[test]
    fn compaction_model_name_treats_missing_or_blank_value_as_unset() {
        let (_temp_dir, config) = test_config();

        assert_eq!(configured_compaction_model_name_from(&config), None);

        config.set_param("GOOSE_COMPACTION_MODEL", "   ").unwrap();
        assert_eq!(configured_compaction_model_name_from(&config), None);
    }

    #[test]
    fn compaction_model_name_treats_malformed_value_as_unset() {
        let (_temp_dir, config) = test_config();

        config
            .set_param("GOOSE_COMPACTION_MODEL", vec!["not-a-model-name"])
            .unwrap();

        assert_eq!(configured_compaction_model_name_from(&config), None);
    }

    #[test]
    fn compaction_model_name_uses_valid_value() {
        let (_temp_dir, config) = test_config();

        config
            .set_param("GOOSE_COMPACTION_MODEL", " compaction-model ")
            .unwrap();

        assert_eq!(
            configured_compaction_model_name_from(&config),
            Some("compaction-model".to_string())
        );
    }

    struct FallbackProbe {
        failing_model: &'static str,
        calls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl Provider for FallbackProbe {
        fn get_name(&self) -> &str {
            "mock"
        }

        async fn stream(
            &self,
            model_config: &ModelConfig,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> std::result::Result<MessageStream, ProviderError> {
            self.calls
                .lock()
                .unwrap()
                .push(model_config.model_name.clone());
            if model_config.model_name == self.failing_model {
                return Err(ProviderError::ExecutionError("no such model".to_string()));
            }
            Ok(stream_from_single_message(
                Message::assistant().with_text("ok"),
                ProviderUsage::new(model_config.model_name.clone(), Usage::default()),
            ))
        }
    }

    #[tokio::test]
    async fn explicit_compaction_model_failure_falls_back_to_main_model() {
        let provider = FallbackProbe {
            failing_model: "broken-compaction-model",
            calls: Mutex::new(Vec::new()),
        };
        let main_model = ModelConfig::new("main-model");
        let resolved = Ok(ModelConfig::new("broken-compaction-model"));

        let (message, _usage) = complete_with_model(
            &provider,
            &main_model,
            resolved,
            CompletionRequest {
                configured_model_key: Some("GOOSE_COMPACTION_MODEL"),
                session_id: "test-session",
                system: "system",
                messages: &[],
                tools: &[],
            },
        )
        .await
        .expect("fallback to the main model should succeed");

        assert_eq!(message.as_concat_text(), "ok");
        assert_eq!(
            provider.calls.lock().unwrap().as_slice(),
            ["broken-compaction-model", "main-model"]
        );
    }
}
