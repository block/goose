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
    let model = materialize_model_config_inner(model, false)?
        .with_context_limit(context_limit)
        .with_inherited_session_settings_from(previous, request_params)
        .with_default_thinking_effort(config.get_goose_thinking_effort());

    // Canonical first, then GOOSE_CONTEXT_LIMIT only as a last-resort default.
    Ok(model
        .with_canonical_limits(provider_name)
        .with_default_context_limit(config.get_goose_context_limit()?))
}

pub fn materialize_model_config(provider_name: &str, model: ModelConfig) -> Result<ModelConfig> {
    let config = Config::global();
    let model = materialize_model_config_inner(model, true)?;
    // Canonical (model-specific) limits win; GOOSE_CONTEXT_LIMIT is only a
    // last-resort default for models with no known limit, never a cap over them.
    Ok(model
        .with_canonical_limits(provider_name)
        .with_default_context_limit(config.get_goose_context_limit()?))
}

fn materialize_model_config_inner(
    mut model: ModelConfig,
    include_default_thinking_effort: bool,
) -> Result<ModelConfig> {
    let config = Config::global();

    if model.temperature.is_none() {
        model = model.with_temperature(get_goose_temperature(config)?);
    }

    if model.toolshim && model.toolshim_model.is_none() {
        model = model.with_toolshim_model(get_goose_toolshim_model(config)?);
    }

    // GOOSE_CONTEXT_LIMIT is applied later, after canonical limits, so a model's
    // known context window is never overridden by the global default.
    model = model.with_default_max_tokens(config.get_goose_max_tokens()?);

    if include_default_thinking_effort {
        model = model.with_default_thinking_effort(config.get_goose_thinking_effort());
    }

    Ok(model)
}

fn configured_fast_model_name() -> Option<String> {
    Config::global()
        .get_param::<String>("GOOSE_FAST_MODEL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Resolve the model config to use for lightweight "fast" tasks (session
/// naming, compaction, summarization). Resolution order:
///   1. `GOOSE_FAST_MODEL` (user override)
///   2. the provider's declared default fast model
///   3. the supplied `model_config` (i.e. the main model)
///
/// The resulting config is materialized against the same provider so it picks
/// up context limits, temperature, and other provider defaults.
pub async fn get_fast_model(
    provider_name: &str,
    model_config: &ModelConfig,
) -> Result<ModelConfig> {
    let fast_model_name = match configured_fast_model_name() {
        Some(name) => Some(name),
        None => provider_default_fast_model(provider_name).await,
    };

    match fast_model_name {
        Some(name) if name != model_config.model_name => {
            model_config_from_user_config(provider_name, name)
        }
        _ => Ok(model_config.clone()),
    }
}

/// Run a completion for a lightweight "fast" task (session naming, compaction,
/// summarization) using the provider's fast model, falling back to the supplied
/// main `model_config` if the fast model errors.
pub async fn complete_fast(
    provider: &dyn Provider,
    model_config: &ModelConfig,
    session_id: &str,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<(Message, ProviderUsage), ProviderError> {
    let fast_model_config = get_fast_model(provider.get_name(), model_config)
        .await
        .map_err(|e| ProviderError::ExecutionError(e.to_string()))?
        .with_thinking_effort(ThinkingEffort::Off);

    match provider
        .complete(&fast_model_config, session_id, system, messages, tools)
        .await
    {
        Ok(response) => Ok(response),
        Err(e) if fast_model_config.model_name != model_config.model_name => {
            tracing::warn!(
                "Fast model {} failed with error: {}. Falling back to main model {}",
                fast_model_config.model_name,
                e,
                model_config.model_name
            );
            let fallback_config = model_config
                .clone()
                .with_thinking_effort(ThinkingEffort::Off);
            provider
                .complete(&fallback_config, session_id, system, messages, tools)
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
    use goose_providers::model::ModelConfig;

    #[test]
    fn canonical_limit_wins_over_global_context_limit() {
        let _guard = env_lock::lock_env([("GOOSE_CONTEXT_LIMIT", Some("1000"))]);

        // A model with a known canonical context window keeps it instead of being
        // capped to the much smaller global default.
        let canonical = ModelConfig::new("claude-3-5-sonnet-20241022")
            .with_canonical_limits("anthropic")
            .context_limit();
        assert!(
            canonical > 1000,
            "expected a canonical context limit for the test model"
        );

        let materialized =
            materialize_model_config("anthropic", ModelConfig::new("claude-3-5-sonnet-20241022"))
                .expect("materialize");
        assert_eq!(materialized.context_limit(), canonical);
    }

    #[test]
    fn global_context_limit_is_last_resort_for_unknown_models() {
        let _guard = env_lock::lock_env([("GOOSE_CONTEXT_LIMIT", Some("1000"))]);

        let materialized = materialize_model_config(
            "anthropic",
            ModelConfig::new("a-model-not-in-the-canonical-registry-xyz"),
        )
        .expect("materialize");
        assert_eq!(materialized.context_limit(), 1000);
    }
}
