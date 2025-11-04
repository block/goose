use crate::state::AppState;
use crate::tunnel::{TunnelMode, TunnelStatus};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TunnelStartRequest {
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TunnelModeRequest {
    pub mode: TunnelMode,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

/// Start the tunnel
///
/// Starts a tunnel with the specified port. The tunnel mode (lapstone or tailscale)
/// is determined by the current configuration.
#[utoipa::path(
    post,
    path = "/api/tunnel/start",
    request_body = TunnelStartRequest,
    responses(
        (status = 200, description = "Tunnel started successfully", body = TunnelStatus),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn start_tunnel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TunnelStartRequest>,
) -> Response {
    match state.tunnel_manager.start(req.port).await {
        Ok(info) => {
            let status = TunnelStatus {
                state: crate::tunnel::TunnelState::Running,
                info: Some(info),
            };
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

/// Get tunnel mode
///
/// Returns the current tunnel mode (lapstone or tailscale).
#[utoipa::path(
    get,
    path = "/api/tunnel/mode",
    responses(
        (status = 200, description = "Tunnel mode", body = TunnelModeRequest)
    )
)]
pub async fn get_tunnel_mode(State(state): State<Arc<AppState>>) -> Response {
    let mode = state.tunnel_manager.get_mode().await;
    (StatusCode::OK, Json(TunnelModeRequest { mode })).into_response()
}

/// Set tunnel mode
///
/// Sets the tunnel mode to either lapstone or tailscale.
/// The tunnel must be stopped before changing modes.
#[utoipa::path(
    post,
    path = "/api/tunnel/mode",
    request_body = TunnelModeRequest,
    responses(
        (status = 200, description = "Tunnel mode updated"),
        (status = 400, description = "Bad request - tunnel is running", body = ErrorResponse)
    )
)]
pub async fn set_tunnel_mode(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TunnelModeRequest>,
) -> Response {
    let status = state.tunnel_manager.get_status().await;
    if status.state != crate::tunnel::TunnelState::Idle {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Cannot change tunnel mode while tunnel is running".to_string(),
            }),
        )
            .into_response();
    }

    state.tunnel_manager.set_mode(req.mode).await;
    StatusCode::OK.into_response()
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/tunnel/start", post(start_tunnel))
        .route("/api/tunnel/stop", post(stop_tunnel))
        .route("/api/tunnel/status", get(get_tunnel_status))
        .route("/api/tunnel/mode", get(get_tunnel_mode))
        .route("/api/tunnel/mode", post(set_tunnel_mode))
        .with_state(state)
}
