use crate::routes::utils::check_provider_configured;
use crate::state::AppState;
use axum::routing::put;
use axum::{
    extract::Path,
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::declarative_providers::LoadedProvider;
use goose::config::paths::Paths;
use goose::config::ExtensionEntry;
use goose::config::{Config, ConfigError};
use goose::model::ModelConfig;
use goose::providers::auto_detect::detect_provider_from_api_key;
use goose::providers::base::{ProviderMetadata, ProviderType};
use goose::providers::canonical::maybe_get_canonical_model;
use goose::providers::create_with_default_model;
use goose::providers::errors::ProviderError;
use goose::providers::providers as get_providers;
use goose::providers::{retry_operation, RetryConfig};
use goose::{
    agents::execute_commands, agents::ExtensionConfig, config::permission::PermissionLevel,
    slash_commands,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::{collections::HashMap, sync::Arc};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ExtensionResponse {
    pub extensions: Vec<ExtensionEntry>,
    #[serde(default)]
    pub warnings: Vec<String>,
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
    pub provider_type: ProviderType,
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

#[derive(Deserialize, ToSchema)]
pub struct UpdateCustomProviderRequest {
    pub engine: String,
    pub display_name: String,
    pub api_url: String,
    pub api_key: String,
    pub models: Vec<String>,
    pub supports_streaming: Option<bool>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize, ToSchema)]
pub struct CheckProviderRequest {
    pub provider: String,
}

#[derive(Deserialize, ToSchema)]
pub struct SetProviderRequest {
    pub provider: String,
    pub model: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaskedSecret {
    pub masked_value: String,
}

#[derive(Serialize, ToSchema)]
#[serde(untagged)]
pub enum ConfigValueResponse {
    Value(Value),
    MaskedValue(MaskedSecret),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum CommandType {
    Builtin,
    Recipe,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlashCommand {
    pub command: String,
    pub help: String,
    pub command_type: CommandType,
}
#[derive(Serialize, ToSchema)]
pub struct SlashCommandsResponse {
    pub commands: Vec<SlashCommand>,
}

#[derive(Deserialize, ToSchema)]
pub struct DetectProviderRequest {
    pub api_key: String,
}

#[derive(Serialize, ToSchema)]
pub struct DetectProviderResponse {
    pub provider_name: String,
    pub models: Vec<String>,
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
    Json(query): Json<UpsertConfigQuery>,
) -> Result<Json<Value>, StatusCode> {
    let config = Config::global();
    let result = config.set(&query.key, &query.value, query.is_secret);

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
pub async fn remove_config(Json(query): Json<ConfigKeyQuery>) -> Result<Json<String>, StatusCode> {
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

const SECRET_MASK_SHOW_LEN: usize = 8;

fn mask_secret(secret: Value) -> String {
    let as_string = match secret {
        Value::String(s) => s,
        _ => serde_json::to_string(&secret).unwrap_or_else(|_| secret.to_string()),
    };

    let chars: Vec<_> = as_string.chars().collect();
    let show_len = std::cmp::min(chars.len() / 2, SECRET_MASK_SHOW_LEN);
    let visible: String = chars.iter().take(show_len).collect();
    let mask = "*".repeat(chars.len() - show_len);

    format!("{}{}", visible, mask)
}

fn is_valid_provider_name(provider_name: &str) -> bool {
    !provider_name.is_empty()
        && provider_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<ConfigValueResponse>, StatusCode> {
    if query.key == "model-limits" {
        let limits = ModelConfig::get_all_model_limits();
        return Ok(Json(ConfigValueResponse::Value(
            serde_json::to_value(limits).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        )));
    }

    let config = Config::global();

    let response_value = match config.get(&query.key, query.is_secret) {
        Ok(value) => {
            if query.is_secret {
                ConfigValueResponse::MaskedValue(MaskedSecret {
                    masked_value: mask_secret(value),
                })
            } else {
                ConfigValueResponse::Value(value)
            }
        }
        Err(ConfigError::NotFound(_)) => ConfigValueResponse::Value(Value::Null),
        Err(_) => {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
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
pub async fn get_extensions() -> Result<Json<ExtensionResponse>, StatusCode> {
    let extensions = goose::config::get_all_extensions();
    let warnings = goose::config::get_warnings();
    Ok(Json(ExtensionResponse {
        extensions,
        warnings,
    }))
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
    Json(extension_query): Json<ExtensionQuery>,
) -> Result<Json<String>, StatusCode> {
    let extensions = goose::config::get_all_extensions();
    let key = goose::config::extensions::name_to_key(&extension_query.name);

    let is_update = extensions.iter().any(|e| e.config.key() == key);

    goose::config::set_extension(ExtensionEntry {
        enabled: extension_query.enabled,
        config: extension_query.config,
    });

    if is_update {
        Ok(Json(format!("Updated extension {}", extension_query.name)))
    } else {
        Ok(Json(format!("Added extension {}", extension_query.name)))
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
pub async fn remove_extension(Path(name): Path<String>) -> Result<Json<String>, StatusCode> {
    let key = goose::config::extensions::name_to_key(&name);
    goose::config::remove_extension(&key);
    Ok(Json(format!("Removed extension {}", name)))
}

#[utoipa::path(
    get,
    path = "/config",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = ConfigResponse)
    )
)]
pub async fn read_all_config() -> Result<Json<ConfigResponse>, StatusCode> {
    let config = Config::global();

    let values = config
        .all_values()
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
pub async fn providers() -> Result<Json<Vec<ProviderDetails>>, StatusCode> {
    let providers = get_providers().await;
    let providers_response: Vec<ProviderDetails> = providers
        .into_iter()
        .map(|(metadata, provider_type)| {
            let is_configured = check_provider_configured(&metadata, provider_type);

            ProviderDetails {
                name: metadata.name.clone(),
                metadata,
                is_configured,
                provider_type,
            }
        })
        .collect();

    Ok(Json(providers_response))
}

#[utoipa::path(
    get,
    path = "/config/providers/{name}/models",
    params(
        ("name" = String, Path, description = "Provider name (e.g., openai)")
    ),
    responses(
        (status = 200, description = "Models fetched successfully", body = [String]),
        (status = 400, description = "Unknown provider, provider not configured, or authentication error"),
        (status = 429, description = "Rate limit exceeded"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_provider_models(
    Path(name): Path<String>,
) -> Result<Json<Vec<String>>, StatusCode> {
    let loaded_provider = goose::config::declarative_providers::load_provider(name.as_str()).ok();
    // TODO(Douwe): support a get models url for custom providers
    if let Some(loaded_provider) = loaded_provider {
        return Ok(Json(
            loaded_provider
                .config
                .models
                .into_iter()
                .map(|m| m.name)
                .collect::<Vec<_>>(),
        ));
    }

    let all = get_providers()
        .await
        .into_iter()
        //.map(|(m, p)| m)
        .collect::<Vec<_>>();
    let Some((metadata, provider_type)) = all.into_iter().find(|(m, _)| m.name == name) else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if !check_provider_configured(&metadata, provider_type) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let model_config =
        ModelConfig::new(&metadata.default_model).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let provider = goose::providers::create(&name, model_config)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Config endpoints have no user session; use an ephemeral id for the probe.
    let session_id = Uuid::new_v4().to_string();
    let models_result = retry_operation(&RetryConfig::default(), || async {
        provider.fetch_recommended_models(&session_id).await
    })
    .await;

    match models_result {
        Ok(Some(models)) => Ok(Json(models)),
        Ok(None) => Ok(Json(Vec::new())),
        Err(provider_error) => {
            let status_code = match provider_error {
                // Permanent misconfigurations - client should fix configuration
                ProviderError::Authentication(_) => StatusCode::BAD_REQUEST,
                ProviderError::UsageError(_) => StatusCode::BAD_REQUEST,

                // Transient errors - client should retry later
                ProviderError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,

                // All other errors - internal server error
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };

            tracing::warn!(
                "Provider {} failed to fetch models: {}",
                name,
                provider_error
            );
            Err(status_code)
        }
    }
}

#[utoipa::path(
    get,
    path = "/config/slash_commands",
    responses(
        (status = 200, description = "Slash commands retrieved successfully", body = SlashCommandsResponse)
    )
)]
pub async fn get_slash_commands() -> Result<Json<SlashCommandsResponse>, StatusCode> {
    let mut commands: Vec<_> = slash_commands::list_commands()
        .iter()
        .map(|command| SlashCommand {
            command: command.command.clone(),
            help: command.recipe_path.clone(),
            command_type: CommandType::Recipe,
        })
        .collect();

    for cmd_def in execute_commands::list_commands() {
        commands.push(SlashCommand {
            command: cmd_def.name.to_string(),
            help: cmd_def.description.to_string(),
            command_type: CommandType::Builtin,
        });
    }

    Ok(Json(SlashCommandsResponse { commands }))
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
    pub provider: String,
    pub model: String,
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
    Json(query): Json<PricingQuery>,
) -> Result<Json<PricingResponse>, StatusCode> {
    let canonical_model =
        maybe_get_canonical_model(&query.provider, &query.model).ok_or(StatusCode::NOT_FOUND)?;

    let mut pricing_data = Vec::new();

    if let (Some(input_cost), Some(output_cost)) = (
        canonical_model.pricing.prompt,
        canonical_model.pricing.completion,
    ) {
        pricing_data.push(PricingData {
            provider: query.provider.clone(),
            model: query.model.clone(),
            input_token_cost: input_cost,
            output_token_cost: output_cost,
            currency: "$".to_string(),
            context_length: Some(canonical_model.context_length as u32),
        });
    }

    Ok(Json(PricingResponse {
        pricing: pricing_data,
        source: "canonical".to_string(),
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
pub async fn init_config() -> Result<Json<String>, StatusCode> {
    let config = Config::global();

    if config.exists() {
        return Ok(Json("Config already exists".to_string()));
    }

    // Use the shared function to load init-config.yaml
    match goose::config::base::load_init_config_from_workspace() {
        Ok(init_values) => match config.initialize_if_empty(init_values) {
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
    Json(query): Json<UpsertPermissionsQuery>,
) -> Result<Json<String>, StatusCode> {
    let permission_manager = goose::config::PermissionManager::instance();

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
    path = "/config/detect-provider",
    request_body = DetectProviderRequest,
    responses(
        (status = 200, description = "Provider detected successfully", body = DetectProviderResponse),
        (status = 404, description = "No matching provider found"),
    )
)]
pub async fn detect_provider(
    Json(detect_request): Json<DetectProviderRequest>,
) -> Result<Json<DetectProviderResponse>, StatusCode> {
    let api_key = detect_request.api_key.trim();
    // Provider detection runs without a user session; use an ephemeral id.
    let session_id = Uuid::new_v4().to_string();

    match detect_provider_from_api_key(&session_id, api_key).await {
        Some((provider_name, models)) => Ok(Json(DetectProviderResponse {
            provider_name,
            models,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    post,
    path = "/config/backup",
    responses(
        (status = 200, description = "Config file backed up", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn backup_config() -> Result<Json<String>, StatusCode> {
    let config_path = Paths::config_dir().join("config.yaml");

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
pub async fn recover_config() -> Result<Json<String>, StatusCode> {
    let config = Config::global();

    // Force a reload which will trigger recovery if needed
    match config.all_values() {
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
pub async fn validate_config() -> Result<Json<String>, StatusCode> {
    let config_path = Paths::config_dir().join("config.yaml");

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
    post,
    path = "/config/custom-providers",
    request_body = UpdateCustomProviderRequest,
    responses(
        (status = 200, description = "Custom provider created successfully", body = String),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_custom_provider(
    Json(request): Json<UpdateCustomProviderRequest>,
) -> Result<Json<String>, StatusCode> {
    let config = goose::config::declarative_providers::create_custom_provider(
        &request.engine,
        request.display_name,
        request.api_url,
        request.api_key,
        request.models,
        request.supports_streaming,
        request.headers,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Err(e) = goose::providers::refresh_custom_providers().await {
        tracing::warn!("Failed to refresh custom providers after creation: {}", e);
    }

    Ok(Json(format!("Custom provider added - ID: {}", config.id())))
}

#[utoipa::path(
    get,
    path = "/config/custom-providers/{id}",
    responses(
        (status = 200, description = "Custom provider retrieved successfully", body = LoadedProvider),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_custom_provider(
    Path(id): Path<String>,
) -> Result<Json<LoadedProvider>, StatusCode> {
    let loaded_provider = goose::config::declarative_providers::load_provider(id.as_str())
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(loaded_provider))
}

#[utoipa::path(
    delete,
    path = "/config/custom-providers/{id}",
    responses(
        (status = 200, description = "Custom provider removed successfully", body = String),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_custom_provider(Path(id): Path<String>) -> Result<Json<String>, StatusCode> {
    goose::config::declarative_providers::remove_custom_provider(&id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Err(e) = goose::providers::refresh_custom_providers().await {
        tracing::warn!("Failed to refresh custom providers after deletion: {}", e);
    }

    Ok(Json(format!("Removed custom provider: {}", id)))
}

#[utoipa::path(
    put,
    path = "/config/custom-providers/{id}",
    request_body = UpdateCustomProviderRequest,
    responses(
        (status = 200, description = "Custom provider updated successfully", body = String),
        (status = 404, description = "Provider not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_custom_provider(
    Path(id): Path<String>,
    Json(request): Json<UpdateCustomProviderRequest>,
) -> Result<Json<String>, StatusCode> {
    goose::config::declarative_providers::update_custom_provider(
        &id,
        &request.engine,
        request.display_name,
        request.api_url,
        request.api_key,
        request.models,
        request.supports_streaming,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Err(e) = goose::providers::refresh_custom_providers().await {
        tracing::warn!("Failed to refresh custom providers after update: {}", e);
    }

    Ok(Json(format!("Updated custom provider: {}", id)))
}

#[utoipa::path(
    post,
    path = "/config/check_provider",
    request_body = CheckProviderRequest,
)]
pub async fn check_provider(
    Json(CheckProviderRequest { provider }): Json<CheckProviderRequest>,
) -> Result<(), (StatusCode, String)> {
    create_with_default_model(&provider)
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/config/set_provider",
    request_body = SetProviderRequest,
)]
pub async fn set_config_provider(
    Json(SetProviderRequest { provider, model }): Json<SetProviderRequest>,
) -> Result<(), (StatusCode, String)> {
    create_with_default_model(&provider)
        .await
        .and_then(|_| {
            let config = Config::global();
            config
                .set_goose_provider(provider)
                .and_then(|_| config.set_goose_model(model))
                .map_err(|e| anyhow::anyhow!(e))
        })
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/config/providers/{name}/oauth",
    params(
        ("name" = String, Path, description = "Provider name")
    ),
    responses(
        (status = 200, description = "OAuth configuration completed"),
        (status = 400, description = "OAuth configuration failed")
    )
)]
pub async fn configure_provider_oauth(
    Path(provider_name): Path<String>,
) -> Result<Json<String>, (StatusCode, String)> {
    use goose::model::ModelConfig;
    use goose::providers::create;

    if !is_valid_provider_name(&provider_name) {
        return Err((StatusCode::BAD_REQUEST, "Invalid provider name".to_string()));
    }

    let temp_model =
        ModelConfig::new("temp").map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let provider = create(&provider_name, temp_model).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to create provider: {}", e),
        )
    })?;

    provider.configure_oauth().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("OAuth configuration failed: {}", e),
        )
    })?;

    // Mark the provider as configured after successful OAuth
    let configured_marker = format!("{}_configured", provider_name);
    let config = goose::config::Config::global();
    config
        .set_param(&configured_marker, true)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json("OAuth configuration completed".to_string()))
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
        .route("/config/providers/{name}/models", get(get_provider_models))
        .route("/config/detect-provider", post(detect_provider))
        .route("/config/slash_commands", get(get_slash_commands))
        .route("/config/pricing", post(get_pricing))
        .route("/config/init", post(init_config))
        .route("/config/backup", post(backup_config))
        .route("/config/recover", post(recover_config))
        .route("/config/validate", get(validate_config))
        .route("/config/permissions", post(upsert_permissions))
        .route("/config/custom-providers", post(create_custom_provider))
        .route(
            "/config/custom-providers/{id}",
            delete(remove_custom_provider),
        )
        .route("/config/custom-providers/{id}", put(update_custom_provider))
        .route("/config/custom-providers/{id}", get(get_custom_provider))
        .route("/config/check_provider", post(check_provider))
        .route("/config/set_provider", post(set_config_provider))
        .route(
            "/config/providers/{name}/oauth",
            post(configure_provider_oauth),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use http::HeaderMap;

    use super::*;

    #[tokio::test]
    async fn test_read_model_limits() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Secret-Key", "test".parse().unwrap());

        let result = read_config(Json(ConfigKeyQuery {
            key: "model-limits".to_string(),
            is_secret: false,
        }))
        .await;

        assert!(result.is_ok());
        let response = match result.unwrap().0 {
            ConfigValueResponse::Value(value) => value,
            ConfigValueResponse::MaskedValue(_) => panic!("unexpected secret"),
        };

        let limits: Vec<goose::model::ModelLimitConfig> = serde_json::from_value(response).unwrap();
        assert!(!limits.is_empty());

        let gpt4_limit = limits.iter().find(|l| l.pattern == "gpt-4o");
        assert!(gpt4_limit.is_some());
        assert_eq!(gpt4_limit.unwrap().context_limit, 128_000);
    }
}
