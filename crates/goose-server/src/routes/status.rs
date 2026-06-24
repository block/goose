use axum::extract::{Path, Query, State};
use axum::{http::StatusCode, routing::get, Json, Router};
use goose::session::{
    generate_diagnostics, get_system_info, DiagnosticsLevel, DiagnosticsReport, SystemInfo,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[utoipa::path(get, path = "/status",
    responses(
        (status = 200, description = "ok", body = String),
    )
)]
async fn status() -> String {
    "ok".to_string()
}

#[utoipa::path(get, path = "/system_info",
    responses(
        (status = 200, description = "System information", body = SystemInfo),
    )
)]
async fn system_info() -> Json<SystemInfo> {
    Json(get_system_info())
}

#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
struct DiagnosticsQuery {
    level: Option<DiagnosticsLevel>,
}

#[utoipa::path(get, path = "/diagnostics/{session_id}",
    params(
        DiagnosticsQuery,
    ),
    responses(
        (status = 200, description = "Diagnostics report", body = DiagnosticsReport),
        (status = 500, description = "Failed to generate diagnostics"),
    )
)]
async fn diagnostics(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(query): Query<DiagnosticsQuery>,
) -> Result<Json<DiagnosticsReport>, StatusCode> {
    generate_diagnostics(
        state.session_manager(),
        &session_id,
        query.level.unwrap_or(DiagnosticsLevel::Full),
    )
    .await
    .map(Json)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/system_info", get(system_info))
        .route("/diagnostics/{session_id}", get(diagnostics))
        .with_state(state)
}
