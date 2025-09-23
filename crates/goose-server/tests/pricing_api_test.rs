use axum::http::StatusCode;
use axum::Router;
use axum::{body::Body, http::Request};
use serde_json::json;
use tower::ServiceExt;

async fn create_test_app() -> Router {
    let state = goose_server::AppState::new().await.unwrap();
    goose_server::routes::config_management::routes(state)
}

#[tokio::test]
async fn test_pricing_endpoint_basic() {
    // Basic test to ensure pricing endpoint responds correctly
    let app = create_test_app().await;

    let request = Request::builder()
        .uri("/config/pricing")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test")
        .body(Body::from(json!({"configured_only": true}).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
