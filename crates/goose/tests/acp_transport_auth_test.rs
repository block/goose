use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use goose::acp::server_factory::{AcpServer, AcpServerFactoryConfig};
use goose::acp::transport::create_router;
use goose::agents::GoosePlatform;
use tower::ServiceExt;

const SECRET: &str = "test-secret-token";

fn test_router(require_token: bool, dir: &tempfile::TempDir) -> Router {
    let server = Arc::new(AcpServer::new(AcpServerFactoryConfig {
        builtins: vec![],
        data_dir: dir.path().join("data"),
        config_dir: dir.path().join("config"),
        goose_platform: GoosePlatform::GooseCli,
        additional_source_roots: Vec::new(),
    }));
    create_router(server, SECRET.to_string(), require_token)
}

async fn send(router: &Router, method: Method, uri: &str, headers: &[(&str, &str)]) -> StatusCode {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let request = builder.body(Body::empty()).unwrap();
    router.clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn acp_requests_without_token_are_unauthorized() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(true, &dir);

    for method in [Method::GET, Method::POST, Method::DELETE] {
        let status = send(&router, method.clone(), "/acp", &[]).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "method: {method}");
    }
}

#[tokio::test]
async fn websocket_handshake_without_token_is_unauthorized() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(true, &dir);

    let status = send(
        &router,
        Method::GET,
        "/acp",
        &[
            ("connection", "upgrade"),
            ("upgrade", "websocket"),
            ("sec-websocket-version", "13"),
            ("sec-websocket-key", "dGVzdGtleTEyMzQ1Njc4OQ=="),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn header_token_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(true, &dir);

    // 406 (missing Accept: text/event-stream) proves the request passed auth.
    let status = send(&router, Method::GET, "/acp", &[("X-Secret-Key", SECRET)]).await;
    assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn query_token_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(true, &dir);

    let uri = format!("/acp?token={SECRET}");
    let status = send(&router, Method::GET, &uri, &[]).await;
    assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn wrong_token_is_unauthorized() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(true, &dir);

    let status = send(&router, Method::GET, "/acp", &[("X-Secret-Key", "nope")]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let status = send(&router, Method::GET, "/acp?token=nope", &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn health_endpoints_skip_token_check() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(true, &dir);

    for path in ["/health", "/status"] {
        let status = send(&router, Method::GET, path, &[]).await;
        assert_eq!(status, StatusCode::OK, "path: {path}");
    }
}

#[tokio::test]
async fn acp_open_when_no_secret_configured() {
    let dir = tempfile::tempdir().unwrap();
    let router = test_router(false, &dir);

    let status = send(&router, Method::GET, "/acp", &[]).await;
    assert_eq!(status, StatusCode::NOT_ACCEPTABLE);
}
