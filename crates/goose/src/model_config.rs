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
                .map(|config| config.with_request_headers(model_config.request_headers.clone()))
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

    match crate::session_context::with_session_id(
        Some(session_id.to_string()),
        provider.complete(&fast_model_config, system, messages, tools),
    )
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
            crate::session_context::with_session_id(
                Some(session_id.to_string()),
                provider.complete(&fallback_config, system, messages, tools),
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
