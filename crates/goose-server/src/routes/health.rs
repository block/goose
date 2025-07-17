use axum::{Json, Router, routing::get};
use serde::Serialize;

#[derive(Serialize)]
struct StatusResponse {
    status: &'static str,
}

/// Simple status endpoint that returns 200 OK when the server is running
async fn status() -> Json<StatusResponse> {
    Json(StatusResponse { status: "ok" })
}

/// Configure health check routes
pub fn routes() -> Router {
    Router::new().route("/status", get(status))
}
