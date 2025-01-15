use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use goose::{agents::AgentFactory, providers::factory};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct VersionsResponse {
    available_versions: Vec<String>,
    default_version: String,
}

#[derive(Deserialize)]
struct CreateAgentRequest {
    version: Option<String>,
    provider: String,
}

#[derive(Serialize)]
struct CreateAgentResponse {
    version: String,
}

async fn get_versions() -> Json<VersionsResponse> {
    let versions = AgentFactory::available_versions();
    let default_version = AgentFactory::default_version().to_string();

    Json(VersionsResponse {
        available_versions: versions.iter().map(|v| v.to_string()).collect(),
        default_version,
    })
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

    let provider = factory::get_provider(&payload.provider).expect("Failed to create provider");

    let version = payload
        .version
        .unwrap_or_else(|| AgentFactory::default_version().to_string());

    let new_agent = AgentFactory::create(&version, provider).expect("Failed to create agent");

    let mut agent = state.agent.lock().await;
    *agent = Some(new_agent);

    Ok(Json(CreateAgentResponse { version }))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/agent/versions", get(get_versions))
        .route("/agent", post(create_agent))
        .with_state(state)
}
