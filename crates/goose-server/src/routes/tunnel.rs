use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

/// Get tunnel info
#[utoipa::path(
    get,
    path = "/tunnel/status",
    responses(
        (status = 200, description = "Tunnel info", body = TunnelInfo)
    )
)]
pub async fn get_tunnel_status(State(state): State<Arc<AppState>>) -> Response {
    let info = state.tunnel_manager.get_info().await;
    (StatusCode::OK, Json(info)).into_response()
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/tunnel/status", get(get_tunnel_status))
        .with_state(state)
}
