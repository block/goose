use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ProxyQuery {
    secret: String,
}

const MCP_APP_PROXY_HTML: &str = include_str!("templates/mcp_app_proxy.html");

#[utoipa::path(
    get,
    path = "/mcp-app-proxy",
    params(
        ("secret" = String, Query, description = "Secret key for authentication")
    ),
    responses(
        (status = 200, description = "MCP App proxy HTML page", content_type = "text/html"),
        (status = 401, description = "Unauthorized - invalid or missing secret"),
    )
)]
async fn mcp_app_proxy(
    axum::extract::State(secret_key): axum::extract::State<String>,
    Query(params): Query<ProxyQuery>,
) -> Response {
    if params.secret != secret_key {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::HeaderName::from_static("referrer-policy"),
                "no-referrer",
            ),
        ],
        Html(MCP_APP_PROXY_HTML),
    )
        .into_response()
}

pub fn routes(secret_key: String) -> Router {
    Router::new()
        .route("/mcp-app-proxy", get(mcp_app_proxy))
        .with_state(secret_key)
}
