use axum::routing::put;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::Config;
use goose::providers::base::ProviderMetadata;
use goose::providers::providers as get_providers;
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use utoipa::ToSchema;

use crate::state::AppState;

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

#[derive(Deserialize, ToSchema)]
pub struct ExtensionQuery {
    pub name: String,
    pub config: Value,
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

    match config.delete(&query.key) {
        Ok(_) => Ok(Json(format!("Removed key {}", query.key))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    post, // Change from get to post
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
    post,
    path = "/config/extension",
    request_body = ExtensionQuery,
    responses(
        (status = 200, description = "Extension added successfully", body = String),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn add_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(extension): Json<ExtensionQuery>,
) -> Result<Json<String>, StatusCode> {
    // Use the helper function to verify the secret key
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // Get current extensions or initialize empty map
    let mut extensions: HashMap<String, Value> = config
        .get_param("extensions")
        .unwrap_or_else(|_| HashMap::new());

    // Add new extension
    extensions.insert(extension.name.clone(), extension.config);

    // Save updated extensions
    match config.set_param(
        "extensions",
        Value::Object(extensions.into_iter().collect()),
    ) {
        Ok(_) => Ok(Json(format!("Added extension {}", extension.name))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[utoipa::path(
    delete,
    path = "/config/extension",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Extension removed successfully", body = String),
        (status = 404, description = "Extension not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn remove_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<String>, StatusCode> {
    // Use the helper function to verify the secret key
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // Get current extensions
    let mut extensions: HashMap<String, Value> = match config.get_param("extensions") {
        Ok(exts) => exts,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Remove extension if it exists
    if extensions.remove(&query.key).is_some() {
        // Save updated extensions
        match config.set_param(
            "extensions",
            Value::Object(extensions.into_iter().collect()),
        ) {
            Ok(_) => Ok(Json(format!("Removed extension {}", query.key))),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        Err(StatusCode::NOT_FOUND)
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

#[utoipa::path(
    put,
    path = "/config/extension",
    request_body = ExtensionQuery,
    responses(
        (status = 200, description = "Extension configuration updated successfully", body = String),
        (status = 404, description = "Extension not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(extension): Json<ExtensionQuery>,
) -> Result<Json<String>, StatusCode> {
    // Use the helper function to verify the secret key
    verify_secret_key(&headers, &state)?;

    let config = Config::global();

    // Get current extensions
    let mut extensions: HashMap<String, Value> = match config.get_param("extensions") {
        Ok(exts) => exts,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Check if extension exists
    if !extensions.contains_key(&extension.name) {
        return Err(StatusCode::NOT_FOUND);
    }

    // Update extension configuration
    extensions.insert(extension.name.clone(), extension.config);

    // Save updated extensions
    match config.set_param(
        "extensions",
        Value::Object(extensions.into_iter().collect()),
    ) {
        Ok(_) => Ok(Json(format!("Updated extension {}", extension.name))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
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

fn check_provider_configured(metadata: &ProviderMetadata) -> bool {
    let config = Config::global();

    // Check all required keys for the provider
    for key in &metadata.config_keys {
        if key.required {
            let key_name = &key.name;

            // First, check if the key is set in the environment
            let is_set_in_env = env::var(key_name).is_ok();

            // If not set in environment, check the config file based on whether it's a secret or not
            let is_set_in_config = config.get(key_name, key.secret).is_ok();

            // If the key is neither in the environment nor in the config, the provider is not configured
            if !is_set_in_env && !is_set_in_config {
                return false;
            }
        }
    }

    // If all required keys are accounted for, the provider is considered configured
    true
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", post(remove_config))
        .route("/config/read", post(read_config))
        .route("/config/extension", post(add_extension))
        .route("/config/extension", put(update_extension))
        .route("/config/extension", delete(remove_extension))
        .route("/config/providers", get(providers))
        .with_state(state)
}
