use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn check_token(
    State(state): State<String>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.uri().path() == "/status" || request.uri().path() == "/mcp-ui-proxy" {
        return Ok(next.run(request).await);
    }
    let secret_key = request
        .headers()
        .get("X-Secret-Key")
        .and_then(|value| value.to_str().ok());

    match secret_key {
        Some(key) if key == state => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
