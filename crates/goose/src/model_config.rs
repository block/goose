use crate::config::{Config, ConfigError};
use anyhow::{anyhow, Result};
use goose_providers::model::ModelConfig;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
struct PredefinedModel {
    name: String,
    #[serde(default)]
    context_limit: Option<usize>,
    #[serde(default)]
    request_params: Option<HashMap<String, Value>>,
}

pub fn model_config_from_user_config(
    provider_name: &str,
    model_name: impl AsRef<str>,
) -> Result<ModelConfig> {
    let config = Config::global();
    let model = ModelConfig::new(model_name.as_ref())?
        .with_temperature(get_goose_temperature(config)?)
        .with_toolshim(get_goose_toolshim(config)?.unwrap_or(false))
        .with_toolshim_model(get_goose_toolshim_model(config)?);
    materialize_model_config(provider_name, model)
}

pub fn materialize_model_config(
    provider_name: &str,
    mut model: ModelConfig,
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
        .with_default_max_tokens(config.get_goose_max_tokens()?)
        .with_default_thinking_effort(config.get_goose_thinking_effort());

    model = apply_predefined_model_metadata(config, model);

    Ok(model.with_canonical_limits(provider_name))
}

pub fn configured_fast_model_name(default_model: &str) -> String {
    Config::global()
        .get_param::<String>("GOOSE_FAST_MODEL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default_model.to_string())
}

pub fn with_configured_fast_model(
    model: ModelConfig,
    provider_name: &str,
    default_fast_model_name: &str,
) -> Result<ModelConfig> {
    let fast_model_name = configured_fast_model_name(default_fast_model_name);
    let fast_model_config = model_config_from_user_config(provider_name, fast_model_name)?;
    Ok(model.with_fast_model_config(fast_model_config))
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

fn predefined_models(config: &Config) -> Vec<PredefinedModel> {
    match config.get_param::<Vec<PredefinedModel>>("GOOSE_PREDEFINED_MODELS") {
        Ok(models) => models,
        Err(ConfigError::NotFound(_)) => Vec::new(),
        Err(error) => {
            tracing::warn!("Failed to parse GOOSE_PREDEFINED_MODELS: {}", error);
            Vec::new()
        }
    }
}

fn apply_predefined_model_metadata(config: &Config, mut model: ModelConfig) -> ModelConfig {
    let Some(predefined) = predefined_models(config)
        .into_iter()
        .find(|predefined| predefined.name == model.model_name)
    else {
        return model;
    };

    if model.context_limit.is_none() {
        model.context_limit = predefined.context_limit;
    }

    if let Some(request_params) = predefined.request_params {
        let params = model.request_params.get_or_insert_with(HashMap::new);
        for (key, value) in request_params {
            params.entry(key).or_insert(value);
        }
    }

    model
}
