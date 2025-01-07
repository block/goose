use crate::state::AppState;
use axum::{extract::State, routing::get, Json, Router};
use goose::agents::AgentFactory;
use serde::Serialize;

#[derive(Serialize)]
struct VersionsResponse {
    current_version: String,
    available_versions: Vec<String>,
    default_version: String,
}

async fn get_versions(State(state): State<AppState>) -> Json<VersionsResponse> {
    let versions = AgentFactory::available_versions();
    let default_version = AgentFactory::default_version().to_string();

    Json(VersionsResponse {
        current_version: state.agent_version.clone(),
        available_versions: versions.iter().map(|v| v.to_string()).collect(),
        default_version,
    })
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/agent/versions", get(get_versions))
        .with_state(state)
}
