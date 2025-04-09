use crate::routes::utils::check_provider_configured;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use goose::agents::ExtensionConfig;
use goose::config::extensions::name_to_key;
use goose::config::Config;
use goose::config::{ExtensionEntry, ExtensionManager};
use goose::providers::base::ProviderMetadata;
use goose::providers::providers as get_providers;
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::collections::HashMap;
use utoipa::ToSchema;

fn verify_secret_key(headers: &HeaderMap, state: &AppState) -> Result<StatusCode, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        Err(StatusCode::UNAUTHORIZED)
    } else {
        Ok(StatusCode::OK)
    }
}

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

// Define a new structure to encapsulate the provider details along with configuration status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderDetails {
    /// Unique identifier and name of the provider
    pub name: String,
    /// Metadata about the provider
    pub metadata: ProviderMetadata,
    /// Indicates whether the provider is fully configured
    pub is_configured: bool,
}

#[derive(Serialize, ToSchema)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderDetails>,
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(query): Json<UpsertConfigQuery>,
) -> Result<Json<Value>, StatusCode> {
    // Use the helper function to verify the secret key
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<String>, StatusCode> {
    // Use the helper function to verify the secret key
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // Check if the secret flag is true and call the appropriate method
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
    request_body = ConfigKeyQuery, // Switch back to request_body
    responses(
        (status = 200, description = "Configuration value retrieved successfully", body = Value),
        (status = 404, description = "Configuration key not found")
    )
)]
pub async fn read_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<Value>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    match config.get(&query.key, query.is_secret) {
        // Always get the actual value
        Ok(value) => {
            if query.is_secret {
                // If it's marked as secret, return a boolean indicating presence
                Ok(Json(Value::Bool(true)))
            } else {
                // Return the actual value if not secret
                Ok(Json(value))
            }
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
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
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ExtensionResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    match ExtensionManager::get_all() {
        Ok(extensions) => Ok(Json(ExtensionResponse { extensions })),
        Err(err) => {
            // Return UNPROCESSABLE_ENTITY only for DeserializeError, INTERNAL_SERVER_ERROR for everything else
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(extension_query): Json<ExtensionQuery>,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Get existing extensions to check if this is an update
    let extensions = ExtensionManager::get_all().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = name_to_key(&extension_query.name);

    let is_update = extensions.iter().any(|e| e.config.key() == key);

    // Use ExtensionManager to set the extension
    match ExtensionManager::set(ExtensionEntry {
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
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let key = name_to_key(&name);
    // Use ExtensionManager to remove the extension
    match ExtensionManager::remove(&key) {
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
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ConfigResponse>, StatusCode> {
    // Use the helper function to verify the secret key
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // Load values from config file
    let values = config.load_values().unwrap_or_default();

    Ok(Json(ConfigResponse { config: values }))
}

// Modified providers function using the new response type
#[utoipa::path(
    get,
    path = "/config/providers",
    responses(
        (status = 200, description = "All configuration values retrieved successfully", body = [ProviderDetails])
    )
)]
pub async fn providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProviderDetails>>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Fetch the list of providers, which are likely stored in the AppState or can be retrieved via a function call
    let providers_metadata = get_providers();

    // Construct the response by checking configuration status for each provider
    let providers_response: Vec<ProviderDetails> = providers_metadata
        .into_iter()
        .map(|metadata| {
            // Check if the provider is configured (this will depend on how you track configuration status)
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

#[utoipa::path(
    post,
    path = "/config/init",
    responses(
        (status = 200, description = "Config initialization check completed", body = String),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn init_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<String>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // 200 if config already exists
    if config.exists() {
        return Ok(Json("Config already exists".to_string()));
    }

    // Find the workspace root (where the top-level Cargo.toml with [workspace] is)
    let workspace_root = match std::env::current_exe() {
        Ok(mut exe_path) => {
            // Start from the executable's directory and traverse up
            while let Some(parent) = exe_path.parent() {
                let cargo_toml = parent.join("Cargo.toml");
                if cargo_toml.exists() {
                    // Read the Cargo.toml file
                    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                        // Check if it contains [workspace]
                        if content.contains("[workspace]") {
                            exe_path = parent.to_path_buf();
                            break;
                        }
                    }
                }
                exe_path = parent.to_path_buf();
            }
            exe_path
        }
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Check if init-config.yaml exists at workspace root
    let init_config_path = workspace_root.join("init-config.yaml");
    if !init_config_path.exists() {
        return Ok(Json(
            "No init-config.yaml found, using default configuration".to_string(),
        ));
    }

    // Read init-config.yaml and validate
    let init_content = match std::fs::read_to_string(&init_config_path) {
        Ok(content) => content,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let init_values: HashMap<String, Value> = match serde_yaml::from_str(&init_content) {
        Ok(values) => values,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Save init-config.yaml to ~/.config/goose/config.yaml
    match config.save_values(init_values) {
        Ok(_) => Ok(Json("Config initialized successfully".to_string())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", post(remove_config))
        .route("/config/read", post(read_config))
        .route("/config/extensions", get(get_extensions))
        .route("/config/extensions", post(add_extension))
        .route("/config/extensions/:name", delete(remove_extension))
        .route("/config/providers", get(providers))
        .route("/config/init", post(init_config))
        .with_state(state)
}
