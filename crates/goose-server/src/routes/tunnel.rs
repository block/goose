use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Start the tunnel
#[utoipa::path(
    post,
    path = "/api/tunnel/start",
    responses(
        (status = 200, description = "Tunnel started successfully", body = TunnelStatus),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn start_tunnel(State(state): State<Arc<AppState>>) -> Response {
    match state.tunnel_manager.start().await {
        Ok(_info) => {
            let status = state.tunnel_manager.get_status().await;
            (StatusCode::OK, Json(status)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to start tunnel: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Stop the tunnel
///
/// Stops the currently running tunnel and optionally clears the auto-start setting.
#[utoipa::path(
    post,
    path = "/api/tunnel/stop",
    responses(
        (status = 200, description = "Tunnel stopped successfully"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn stop_tunnel(State(state): State<Arc<AppState>>) -> Response {
    state.tunnel_manager.stop(true).await;
    StatusCode::OK.into_response()
}

/// Get tunnel status
///
/// Returns the current tunnel state and connection information if running.
#[utoipa::path(
    get,
    path = "/api/tunnel/status",
    responses(
        (status = 200, description = "Tunnel status", body = TunnelStatus)
    )
)]
pub async fn get_tunnel_status(State(state): State<Arc<AppState>>) -> Response {
    let status = state.tunnel_manager.get_status().await;
    (StatusCode::OK, Json(status)).into_response()
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/tunnel/start", post(start_tunnel))
        .route("/api/tunnel/stop", post(stop_tunnel))
        .route("/api/tunnel/status", get(get_tunnel_status))
        .with_state(state)
}
