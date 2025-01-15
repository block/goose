use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::agents::SystemConfig;
use http::{HeaderMap, StatusCode};
use serde::Serialize;

#[derive(Serialize)]
struct SystemResponse {
    error: bool,
}

async fn add_system(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SystemConfig>,
) -> Result<Json<SystemResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut agent = state.agent.lock().await;
    let agent = agent.as_mut().ok_or(StatusCode::PRECONDITION_REQUIRED)?;
    let response = agent.add_system(request).await;

    Ok(Json(SystemResponse {
        error: response.is_err(),
    }))
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/systems/add", post(add_system))
        .with_state(state)
}
