use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::Config;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Deserialize, ToSchema)]
pub struct UpsertConfigQuery {
    pub key: String,
    pub value: Value,
    pub is_secret: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct ConfigKeyQuery {
    pub key: String,
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
    State(_state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<UpsertConfigQuery>,
) -> Result<Json<Value>, StatusCode> {
    let config = Config::global();

    let result = if query.is_secret.unwrap_or(false) {
        config.set_secret(&query.key, query.value)
    } else {
        config.set(&query.key, query.value)
    };

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
    State(_state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<String>, StatusCode> {
    let config = Config::global();

    match config.delete(&query.key) {
        Ok(_) => Ok(Json(format!("Removed key {}", query.key))),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[utoipa::path(
    get,
    path = "/config/read",
    request_body = ConfigKeyQuery,
    responses(
        (status = 200, description = "Configuration value retrieved successfully", body = Value),
        (status = 404, description = "Configuration key not found")
    )
)]
pub async fn read_config(
    State(_state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<Value>, StatusCode> {
    let config = Config::global();

    match config.get::<Value>(&query.key) {
        Ok(value) => Ok(Json(value)),
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
    State(_state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(extension): Json<ExtensionQuery>,
) -> Result<Json<String>, StatusCode> {
    let config = Config::global();

    // Get current extensions or initialize empty map
    let mut extensions: HashMap<String, Value> =
        config.get("extensions").unwrap_or_else(|_| HashMap::new());

    // Add new extension
    extensions.insert(extension.name.clone(), extension.config);

    // Save updated extensions
    match config.set(
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
    State(_state): State<Arc<Mutex<HashMap<String, Value>>>>,
    Json(query): Json<ConfigKeyQuery>,
) -> Result<Json<String>, StatusCode> {
    let config = Config::global();

    // Get current extensions
    let mut extensions: HashMap<String, Value> = match config.get("extensions") {
        Ok(exts) => exts,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Remove extension if it exists
    if extensions.remove(&query.key).is_some() {
        // Save updated extensions
        match config.set(
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
    State(_state): State<Arc<Mutex<HashMap<String, Value>>>>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    let config = Config::global();

    // Load values from config file
    let values = config.load_values().unwrap_or_default();

    Ok(Json(ConfigResponse { config: values }))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/config", get(read_all_config))
        .route("/config/upsert", post(upsert_config))
        .route("/config/remove", post(remove_config))
        .route("/config/read", post(read_config))
        .route("/config/extension", post(add_extension))
        .route("/config/extension", delete(remove_extension))
        .with_state(state.config)
}
