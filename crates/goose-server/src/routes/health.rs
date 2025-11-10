use axum::{routing::get, Router};

#[utoipa::path(get, path = "/status",
    responses(
        (status = 200, description = "ok", body = String),
    )
)]
async fn status() -> String {
    "ok".to_string()
}

pub fn routes() -> Router {
    Router::new().route("/status", get(status))
}
