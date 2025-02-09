use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use goose::config::Config;
use goose::{agents::AgentFactory, model::ModelConfig, providers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Serialize)]
struct VersionsResponse {
    available_versions: Vec<String>,
    default_version: String,
}

#[derive(Deserialize)]
struct ExtendPromptRequest {
    extension: String,
}

#[derive(Serialize)]
struct ExtendPromptResponse {
    success: bool,
}

#[derive(Deserialize)]
struct CreateAgentRequest {
    version: Option<String>,
    provider: String,
    model: Option<String>,
}

#[derive(Serialize)]
struct CreateAgentResponse {
    version: String,
}

#[derive(Deserialize)]
struct ProviderFile {
    name: String,
    description: String,
    models: Vec<String>,
    required_keys: Vec<String>,
}

#[derive(Serialize)]
struct ProviderDetails {
    name: String,
    description: String,
    models: Vec<String>,
    required_keys: Vec<String>,
}

#[derive(Serialize)]
struct ProviderList {
    id: String,
    details: ProviderDetails,
}

async fn get_versions() -> Json<VersionsResponse> {
    let versions = AgentFactory::available_versions();
    let default_version = AgentFactory::default_version().to_string();

    Json(VersionsResponse {
        available_versions: versions.iter().map(|v| v.to_string()).collect(),
        default_version,
    })
}

async fn extend_prompt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ExtendPromptRequest>,
) -> Result<Json<ExtendPromptResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut agent = state.agent.lock().await;
    if let Some(ref mut agent) = *agent {
        agent.extend_system_prompt(payload.extension).await;
        Ok(Json(ExtendPromptResponse { success: true }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn create_agent(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateAgentRequest>,
) -> Result<Json<CreateAgentResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Set the environment variable for the model if provided
    if let Some(model) = &payload.model {
        let env_var_key = format!("{}_MODEL", payload.provider.to_uppercase());
        env::set_var(env_var_key.clone(), model);
        println!("Set environment variable: {}={}", env_var_key, model);
    }

    let config = Config::global();
    let model = payload.model.unwrap_or_else(|| {
        config
            .get("GOOSE_MODEL")
            .expect("Did not find a model on payload or in env")
    });
    let model_config = ModelConfig::new(model);
    let provider =
        providers::create(&payload.provider, model_config).expect("Failed to create provider");

    let version = payload
        .version
        .unwrap_or_else(|| AgentFactory::default_version().to_string());

    let new_agent = AgentFactory::create(&version, provider).expect("Failed to create agent");

    let mut agent = state.agent.lock().await;
    *agent = Some(new_agent);

    Ok(Json(CreateAgentResponse { version }))
}

async fn list_providers() -> Json<Vec<ProviderList>> {
    let contents = include_str!("providers_and_keys.json");

    let providers: HashMap<String, ProviderFile> =
        serde_json::from_str(contents).expect("Failed to parse providers_and_keys.json");

    let response: Vec<ProviderList> = providers
        .into_iter()
        .map(|(id, provider)| ProviderList {
            id,
            details: ProviderDetails {
                name: provider.name,
                description: provider.description,
                models: provider.models,
                required_keys: provider.required_keys,
            },
        })
        .collect();

    // Return the response as JSON.
    Json(response)
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/agent/versions", get(get_versions))
        .route("/agent/providers", get(list_providers))
        .route("/agent/prompt", post(extend_prompt))
        .route("/agent", post(create_agent))
        .with_state(state)
}
