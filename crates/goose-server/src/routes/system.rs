use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::agents::SystemConfig;
use serde::Serialize;

#[derive(Serialize)]
struct SystemResponse {
    error: bool,
}

async fn add_system(
    State(state): State<AppState>,
    Json(request): Json<SystemConfig>,
) -> Json<SystemResponse> {
    let mut agent = state.agent.lock().await;
    let response = agent.add_system(request).await;

    Json(SystemResponse {
        error: response.is_err(),
    })
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/systems/add", post(add_system))
        .with_state(state)
}
