use axum::http::StatusCode;
use axum::Router;
use axum::{body::Body, http::Request};
use etcetera::AppStrategy;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

async fn create_test_app() -> Router {
    let agent = Arc::new(goose::agents::Agent::default());
    let state = goose_server::AppState::new(agent, "test".to_string()).await;

    // Add scheduler setup like in the existing tests
    let sched_storage_path = etcetera::choose_app_strategy(goose::config::APP_STRATEGY.clone())
        .unwrap()
        .data_dir()
        .join("schedules.json");
    let sched = goose::scheduler_factory::SchedulerFactory::create_legacy(sched_storage_path)
        .await
        .unwrap();
    state.set_scheduler(sched).await;

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
