use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
pub use goose::acp::transport::auth::check_acp_token;
use goose::acp::transport::auth::token_matches;

pub async fn check_token(
    State(state): State<String>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.uri().path() == "/status"
        || request.uri().path() == "/features"
        || request.uri().path() == "/mcp-ui-proxy"
        || request.uri().path() == "/mcp-app-proxy"
        || request.uri().path() == "/mcp-app-guest"
    {
        return Ok(next.run(request).await);
    }
    let secret_key = request
        .headers()
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok());

    if token_matches(secret_key, &state) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
