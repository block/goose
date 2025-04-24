use super::utils::verify_secret_key;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use goose::config::Config;
use http::{HeaderMap, StatusCode};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

#[derive(Serialize)]
struct ConfigResponse {
    error: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigRequest {
    key: String,
    value: String,
    is_secret: bool,
}

async fn store_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ConfigRequest>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    let config = Config::global();
    let result = if request.is_secret {
        config.set_secret(&request.key, Value::String(request.value))
    } else {
        config.set_param(&request.key, Value::String(request.value))
    };
    match result {
        Ok(_) => Ok(Json(ConfigResponse { error: false })),
        Err(_) => Ok(Json(ConfigResponse { error: true })),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderConfigRequest {
    pub providers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigStatus {
    pub is_set: bool,
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub supported: bool,
    pub name: Option<String>,
    pub description: Option<String>,
    pub models: Option<Vec<String>>,
    pub config_status: HashMap<String, ConfigStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProviderConfig {
    name: String,
    description: String,
    models: Vec<String>,
    required_keys: Vec<String>,
}

static PROVIDER_ENV_REQUIREMENTS: Lazy<HashMap<String, ProviderConfig>> = Lazy::new(|| {
    let contents = include_str!("providers_and_keys.json");
    serde_json::from_str(contents).expect("Failed to parse providers_and_keys.json")
});

fn check_key_status(config: &Config, key: &str) -> (bool, Option<String>) {
    if let Ok(_value) = std::env::var(key) {
        (true, Some("env".to_string()))
    } else if config.get_param::<String>(key).is_ok() {
        (true, Some("yaml".to_string()))
    } else if config.get_secret::<String>(key).is_ok() {
        (true, Some("keyring".to_string()))
    } else {
        (false, None)
    }
}

async fn check_provider_configs(
    Json(request): Json<ProviderConfigRequest>,
) -> Result<Json<HashMap<String, ProviderResponse>>, StatusCode> {
    let mut response = HashMap::new();
    let config = Config::global();

    for provider_name in request.providers {
        if let Some(provider_config) = PROVIDER_ENV_REQUIREMENTS.get(&provider_name) {
            let mut config_status = HashMap::new();

            for key in &provider_config.required_keys {
                let (key_set, key_location) = check_key_status(config, key);
                config_status.insert(
                    key.to_string(),
                    ConfigStatus {
                        is_set: key_set,
                        location: key_location,
                    },
                );
            }

            response.insert(
                provider_name,
                ProviderResponse {
                    supported: true,
                    name: Some(provider_config.name.clone()),
                    description: Some(provider_config.description.clone()),
                    models: Some(provider_config.models.clone()),
                    config_status,
                },
            );
        } else {
            response.insert(
                provider_name,
                ProviderResponse {
                    supported: false,
                    name: None,
                    description: None,
                    models: None,
                    config_status: HashMap::new(),
                },
            );
        }
    }

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct GetConfigQuery {
    key: String,
}

#[derive(Serialize)]
pub struct GetConfigResponse {
    value: Option<String>,
}

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<GetConfigQuery>,
) -> Result<Json<GetConfigResponse>, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Fetch the configuration value. Right now we don't allow get a secret.
    let config = Config::global();
    let value = if let Ok(config_value) = config.get_param::<String>(&query.key) {
        Some(config_value)
    } else {
        std::env::var(&query.key).ok()
    };

    // Return the value
    Ok(Json(GetConfigResponse { value }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteConfigRequest {
    key: String,
    is_secret: bool,
}

async fn delete_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeleteConfigRequest>,
) -> Result<StatusCode, StatusCode> {
    verify_secret_key(&headers, &state)?;

    // Attempt to delete the key
    let config = Config::global();
    let result = if request.is_secret {
        config.delete_secret(&request.key)
    } else {
        config.delete(&request.key)
    };
    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/configs/providers", post(check_provider_configs))
        .route("/configs/get", get(get_config))
        .route("/configs/store", post(store_config))
        .route("/configs/delete", delete(delete_config))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unsupported_provider() {
        // Setup
        let request = ProviderConfigRequest {
            providers: vec!["unsupported_provider".to_string()],
        };

        // Execute
        let result = check_provider_configs(Json(request)).await;

        // Assert
        assert!(result.is_ok());
        let Json(response) = result.unwrap();

        let provider_status = response
            .get("unsupported_provider")
            .expect("Provider should exist");
        assert!(!provider_status.supported);
        assert!(provider_status.config_status.is_empty());
    }
}
