use crate::state::AppState;
use axum::{extract::State, routing::post, Json, Router};
use goose::key_manager::save_to_keyring;
use http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SecretResponse {
    error: bool,
}

#[derive(Deserialize)]
struct SecretRequest {
    key: String,
    value: String,
}

async fn store_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SecretRequest>,
) -> Result<Json<SecretResponse>, StatusCode> {
    // Verify secret key
    let secret_key = headers
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if secret_key != state.secret_key {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match save_to_keyring(&request.key, &request.value) {
        Ok(_) => Ok(Json(SecretResponse { error: false })),
        Err(_) => Ok(Json(SecretResponse { error: true })),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/secrets/store", post(store_secret))
        .with_state(state)
}
