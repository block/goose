use super::utils::verify_secret_key;
use crate::routes::utils::check_provider_configured;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use etcetera::{choose_app_strategy, AppStrategy};
use goose::config::APP_STRATEGY;
use goose::config::{extensions::name_to_key, PermissionManager};
use goose::config::{Config, ConfigError};
use goose::config::{ExtensionConfigManager, ExtensionEntry};
use goose::model::ModelConfig;
use goose::providers::base::ProviderMetadata;
use goose::providers::pricing::{
    get_all_pricing, get_model_pricing, parse_model_id, refresh_pricing,
};
use goose::providers::providers as get_providers;
use goose::{agents::ExtensionConfig, config::permission::PermissionLevel};
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::{collections::HashMap, sync::Arc};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ExtensionResponse {
    pub extensions: Vec<ExtensionEntry>,
}

#[derive(Deserialize, ToSchema)]
pub struct ExtensionQuery {
    pub name: String,
    pub config: ExtensionConfig,
    pub enabled: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct UpsertConfigQuery {
    pub key: String,
    pub value: Value,
    pub is_secret: bool,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ConfigKeyQuery {
    pub key: String,
    pub is_secret: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ConfigResponse {
    pub config: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderDetails {
    pub name: String,

    pub metadata: ProviderMetadata,

    pub is_configured: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderDetails>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ToolPermission {
    pub tool_name: String,
    pub permission: PermissionLevel,
}

#[derive(Deserialize, ToSchema)]
pub struct UpsertPermissionsQuery {
    pub tool_permissions: Vec<ToolPermission>,
}

#[utoipa::path(
    post,
    path = "/config/upsert",
    request_body = UpsertConfigQuery,
    responses(
        (status = 200, description = "Configuration value upserted successfully", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upsert_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(query): Json<UpsertConfigQuery>,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();
    let result = config.set(&query.key, query.value, query.is_secret);

    match result {
        Ok(_) => Ok(Json(Value::String(format!("Upserted key {}", query.key)))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[utoipa::path(
    post,
    path = "/config/remove",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value removed successfully", body = String),
        (status = 404, description = "Configuration key not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    let result = if query.is_secret {
        config.delete_secret(&query.key)
    } else {
        config.delete(&query.key)
    };

    match result {
        Ok(_) => Ok(Json(format!("Removed key {}", query.key))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    post,
    path = "/config/read",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value retrieved successfully", body = Value),
        (status = 500, description = "Unable to get the configuration value"),
    )
)]
pub async fn read_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    if query.key == "model-limits" {
        let limits = ModelConfig::get_all_model_limits();
        return Ok(Json(
            serde_json::to_value(limits).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ));
    }

    let config = Config::global();
    let response_value = match config.get(&query.key, query.is_secret) {
        Ok(value) => {
            if query.is_secret {
                Value::Bool(true)
            } else {
                value
            }
        }
        Err(ConfigError::NotFound(_)) => {
            if query.is_secret {
                Value::Bool(false)
            } else {
                Value::Null
            }
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    Ok(Json(response_value))
}

#[utoipa::path(
    get,
    path = "/config/extensions",
    responses(
        (status = 200, description = "All extensions retrieved successfully", body = ExtensionResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_extensions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    match ExtensionConfigManager::get_all() {
        Ok(extensions) => Ok(Json(ExtensionResponse { extensions })),
        Err(err) => {
            if err
                .downcast_ref::<goose::config::base::ConfigError>()
                .is_some_and(|e| matches!(e, goose::config::base::ConfigError::DeserializeError(_)))
            {
                Err(StatusCode::UNPROCESSABLE_ENTITY)
            } else {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/config/extensions",
    request_body = ExtensionQuery,
    responses(
        (status = 200, description = "Extension added or updated successfully", body = String),
        (status = 400, description = "Invalid request"),
        (status = 422, description = "Could not serialize config.yaml"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn add_extension(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(extension_query): Json<ExtensionQuery>,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let extensions =
        ExtensionConfigManager::get_all().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = name_to_key(&extension_query.name);

    let is_update = extensions.iter().any(|e| e.config.key() == key);

    match ExtensionConfigManager::set(ExtensionEntry {
        enabled: extension_query.enabled,
        config: extension_query.config,
    }) {
        Ok(_) => {
            if is_update {
                Ok(Json(format!("Updated extension {}", extension_query.name)))
            } else {
                Ok(Json(format!("Added extension {}", extension_query.name)))
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[utoipa::path(
    delete,
    path = "/config/extensions/{name}",
    responses(
        (status = 200, description = "Extension removed successfully", body = String),
        (status = 404, description = "Extension not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_extension(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let key = name_to_key(&name);
    match ExtensionConfigManager::remove(&key) {
        Ok(_) => Ok(Json(format!("Removed extension {}", name))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = ConfigResponse)
    )
)]
pub async fn read_all_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ConfigResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    let values = config
        .load_values()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    Ok(Json(ConfigResponse { config: values }))
}

#[utoipa::path(
    get,
    path = "/config/providers",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = [ProviderDetails])
    )
)]
pub async fn providers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderDetails>>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let providers_metadata = get_providers();

    let providers_response: Vec<ProviderDetails> = providers_metadata
        .into_iter()
        .map(|metadata| {
            let is_configured = check_provider_configured(&metadata);

            ProviderDetails {
                name: metadata.name.clone(),
                metadata,
                is_configured,
            }
        })
        .collect();

    Ok(Json(providers_response))
}

#[derive(Serialize, ToSchema)]
pub struct PricingData {
    pub provider: String,
    pub model: String,
    pub input_token_cost: f64,
    pub output_token_cost: f64,
    pub currency: String,
    pub context_length: Option<u32>,
}

#[derive(Serialize, ToSchema)]
pub struct PricingResponse {
    pub pricing: Vec<PricingData>,
    pub source: String,
}

#[derive(Deserialize, ToSchema)]
pub struct PricingQuery {
    /// If true, only return pricing for configured providers. If false, return all.
    pub configured_only: Option<bool>,
}

#[utoipa::path(
    post,
    path = "/config/pricing",
    request_body = PricingQuery,
    responses(
        (status = 200, description = "Model pricing data retrieved successfully", body = PricingResponse)
    )
)]
pub async fn get_pricing(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(query): Json<PricingQuery>,
) -> Result<Json<PricingResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let configured_only = query.configured_only.unwrap_or(true);

    // If refresh requested (configured_only = false), refresh the cache
    if !configured_only {
        if let Err(e) = refresh_pricing().await {
            tracing::error!("Failed to refresh pricing data: {}", e);
        }
    }

    let mut pricing_data = Vec::new();

    if !configured_only {
        // Get ALL pricing data from the cache
        let all_pricing = get_all_pricing().await;

        for (provider, models) in all_pricing {
            for (model, pricing) in models {
                pricing_data.push(PricingData {
                    provider: provider.clone(),
                    model: model.clone(),
                    input_token_cost: pricing.input_cost,
                    output_token_cost: pricing.output_cost,
                    currency: "$".to_string(),
                    context_length: pricing.context_length,
                });
            }
        }
    } else {
        // Get only configured providers' pricing
        let providers_metadata = get_providers();

        for metadata in providers_metadata {
            // Skip unconfigured providers if filtering
            if !check_provider_configured(&metadata) {
                continue;
            }

            for model_info in &metadata.known_models {
                // Handle OpenRouter models specially - they store full provider/model names
                let (lookup_provider, lookup_model) = if metadata.name == "openrouter" {
                    // For OpenRouter, parse the model name to extract real provider/model
                    if let Some((provider, model)) = parse_model_id(&model_info.name) {
                        (provider, model)
                    } else {
                        // Fallback if parsing fails
                        (metadata.name.clone(), model_info.name.clone())
                    }
                } else {
                    // For other providers, use names as-is
                    (metadata.name.clone(), model_info.name.clone())
                };

                // Only get pricing from OpenRouter cache
                if let Some(pricing) = get_model_pricing(&lookup_provider, &lookup_model).await {
                    pricing_data.push(PricingData {
                        provider: metadata.name.clone(),
                        model: model_info.name.clone(),
                        input_token_cost: pricing.input_cost,
                        output_token_cost: pricing.output_cost,
                        currency: "$".to_string(),
                        context_length: pricing.context_length,
                    });
                }
                // No fallback to hardcoded prices
            }
        }
    }

    tracing::debug!(
        "Returning pricing for {} models{}",
        pricing_data.len(),
        if configured_only {
            " (configured providers only)"
        } else {
            " (all cached models)"
        }
    );

    Ok(Json(PricingResponse {
        pricing: pricing_data,
        source: "openrouter".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/config/init",
    responses(
        (status = 200, description = "Config initialization check completed", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn init_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    if config.exists() {
        return Ok(Json("Config already exists".to_string()));
    }

    // Use the shared function to load init-config.yaml
    match goose::config::base::load_init_config_from_workspace() {
        Ok(init_values) => match config.save_values(init_values) {
            Ok(_) => Ok(Json("Config initialized successfully".to_string())),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Err(_) => Ok(Json(
            "No init-config.yaml found, using default configuration".to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/config/permissions",
    request_body = UpsertPermissionsQuery,
    responses(
        (status = 200, description = "Permission update completed", body = String),
        (status = 400, description = "Invalid request"),
    )
)]
pub async fn upsert_permissions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(query): Json<UpsertPermissionsQuery>,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let mut permission_manager = PermissionManager::default();

    for tool_permission in &query.tool_permissions {
        permission_manager.update_user_permission(
            &tool_permission.tool_name,
            tool_permission.permission.clone(),
        );
    }

    Ok(Json("Permissions updated successfully".to_string()))
}

#[utoipa::path(
    post,
    path = "/config/backup",
    responses(
        (status = 200, description = "Config file backed up", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn backup_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config_dir = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .config_dir();

    let config_path = config_dir.join("config.yaml");

    if config_path.exists() {
        let file_name = config_path
            .file_name()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut backup_name = file_name.to_os_string();
        backup_name.push(".bak");

        let backup = config_path.with_file_name(backup_name);
        match std::fs::copy(&config_path, &backup) {
            Ok(_) => Ok(Json(format!("Copied {:?} to {:?}", config_path, backup))),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

#[utoipa::path(
    post,
    path = "/config/recover",
    responses(
        (status = 200, description = "Config recovery attempted", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn recover_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // Force a reload which will trigger recovery if needed
    match config.load_values() {
        Ok(values) => {
            let recovered_keys: Vec<String> = values.keys().cloned().collect();
            if recovered_keys.is_empty() {
                Ok(Json("Config recovery completed, but no data was recoverable. Starting with empty configuration.".to_string()))
            } else {
                Ok(Json(format!(
                    "Config recovery completed. Recovered {} keys: {}",
                    recovered_keys.len(),
                    recovered_keys.join(", ")
                )))
            }
        }
        Err(e) => {
            tracing::error!("Config recovery failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/config/validate",
    responses(
        (status = 200, description = "Config validation result", body = String),
        (status = 422, description = "Config file is corrupted")
    )
)]
pub async fn validate_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config_dir = choose_app_strategy(APP_STRATEGY.clone())
        .expect("goose requires a home dir")
        .config_dir();

    let config_path = config_dir.join("config.yaml");

    if !config_path.exists() {
        return Ok(Json("Config file does not exist".to_string()));
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => match serde_yaml::from_str::<serde_yaml::Value>(&content) {
            Ok(_) => Ok(Json("Config file is valid".to_string())),
            Err(e) => {
                tracing::warn!("Config validation failed: {}", e);
                Err(StatusCode::UNPROCESSABLE_ENTITY)
            }
        },
        Err(e) => {
            tracing::error!("Failed to read config file: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/config/current-model",
    responses(
        (status = 200, description = "Current model retrieved successfully", body = String),
    )
)]
pub async fn get_current_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let current_model = goose::providers::base::get_current_model();

    Ok(Json(serde_json::json!({
        "model": current_model
    })))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", post(remove_config))
        .route("/config/read", post(read_config))
        .route("/config/extensions", get(get_extensions))
        .route("/config/extensions", post(add_extension))
        .route("/config/extensions/{name}", delete(remove_extension))
        .route("/config/providers", get(providers))
        .route("/config/pricing", post(get_pricing))
        .route("/config/init", post(init_config))
        .route("/config/backup", post(backup_config))
        .route("/config/recover", post(recover_config))
        .route("/config/validate", get(validate_config))
        .route("/config/permissions", post(upsert_permissions))
        .route("/config/current-model", get(get_current_model))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_model_limits() {
        let test_state = AppState::new(
            Arc::new(goose::agents::Agent::default()),
            "test".to_string(),
        )
        .await;
        let sched_storage_path = choose_app_strategy(APP_STRATEGY.clone())
            .unwrap()
            .data_dir()
            .join("schedules.json");
        let sched = goose::scheduler_factory::SchedulerFactory::create_legacy(sched_storage_path)
            .await
            .unwrap();
        test_state.set_scheduler(sched).await;
        let mut headers = HeaderMap::new();
        headers.insert("X-Secret-Key", "test".parse().unwrap());

        let result = read_config(
            State(test_state),
            headers,
            Json(ConfigKeyQuery {
                key: "model-limits".to_string(),
                is_secret: false,
            }),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();

        let limits: Vec<goose::model::ModelLimitConfig> =
            serde_json::from_value(response.0).unwrap();
        assert!(!limits.is_empty());

        let gpt4_limit = limits.iter().find(|l| l.pattern == "gpt-4o");
        assert!(gpt4_limit.is_some());
        assert_eq!(gpt4_limit.unwrap().context_limit, 128_000);
    }
}
