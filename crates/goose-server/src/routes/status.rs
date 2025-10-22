use axum::body::Body;
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::{extract::Path, http::StatusCode, routing::get, Router};
use goose::session::generate_diagnostics;

#[utoipa::path(get, path = "/status",
    responses(
        (status = 200, description = "ok", body = String),
    )
)]
async fn status() -> String {
    "ok".to_string()
}

#[utoipa::path(get, path = "/diagnostics/{session_id}",
    responses(
        (status = 200, description = "Diagnostics zip file", content_type = "application/zip", body = Vec<u8>),
        (status = 500, description = "Failed to generate diagnostics"),
    )
)]
async fn diagnostics(Path(session_id): Path<String>) -> impl IntoResponse {
    match generate_diagnostics(&session_id).await {
        Ok(zip_data) => {
            let filename = format!("attachment; filename=\"diagnostics_{}.zip\"", session_id);
            let headers = [
                (
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/zip"),
                ),
                (
                    http::header::CONTENT_DISPOSITION,
                    HeaderValue::from_str(&filename).map_err(|_e| StatusCode::BAD_REQUEST)?,
                ),
            ];

            Ok((headers, Body::from(zip_data)))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
pub fn routes() -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/diagnostics/{session_id}", get(diagnostics))
}
