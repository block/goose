use axum::body;
use axum::http::StatusCode;
use axum::Router;
use axum::{body::Body, http::Request};
use etcetera::AppStrategy;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn create_test_app() -> (Router, Arc<goose_server::AppState>) {
    let state = goose_server::AppState::new("test-secret".to_string()).await;

    // Set up scheduler as required
    let sched_storage_path = etcetera::choose_app_strategy(goose::config::APP_STRATEGY.clone())
        .unwrap()
        .data_dir()
        .join("schedules.json");
    let sched = goose::scheduler_factory::SchedulerFactory::create_legacy(sched_storage_path)
        .await
        .unwrap();
    state.set_scheduler(sched).await;

    let app = goose_server::routes::configure(state.clone());
    (app, state)
}

#[tokio::test]
async fn test_start_creates_unique_sessions() {
    let (app, _state) = create_test_app().await;

    // Create first session
    let request1 = Request::builder()
        .uri("/agent/start")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "working_dir": "/tmp/session1"
            })
            .to_string(),
        ))
        .unwrap();

    let response1 = app.clone().oneshot(request1).await.unwrap();
    assert_eq!(response1.status(), StatusCode::OK);

    let body1 = body::to_bytes(response1.into_body(), usize::MAX)
        .await
        .unwrap();
    let json1: Value = serde_json::from_slice(&body1).unwrap();
    let session_id1 = json1["session_id"].as_str().unwrap();

    // Create second session
    let request2 = Request::builder()
        .uri("/agent/start")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "working_dir": "/tmp/session2"
            })
            .to_string(),
        ))
        .unwrap();

    let response2 = app.oneshot(request2).await.unwrap();
    assert_eq!(response2.status(), StatusCode::OK);

    let body2 = body::to_bytes(response2.into_body(), usize::MAX)
        .await
        .unwrap();
    let json2: Value = serde_json::from_slice(&body2).unwrap();
    let session_id2 = json2["session_id"].as_str().unwrap();

    // Session IDs should be different
    assert_ne!(session_id1, session_id2);

    // Both should have metadata
    assert_eq!(
        json1["metadata"]["working_dir"].as_str().unwrap(),
        "/tmp/session1"
    );
    assert_eq!(
        json2["metadata"]["working_dir"].as_str().unwrap(),
        "/tmp/session2"
    );
}

#[tokio::test]
async fn test_resume_retrieves_correct_session() {
    let (app, _state) = create_test_app().await;

    // Create a session first
    let start_request = Request::builder()
        .uri("/agent/start")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "working_dir": "/tmp/resume_test"
            })
            .to_string(),
        ))
        .unwrap();

    let start_response = app.clone().oneshot(start_request).await.unwrap();
    let start_body = body::to_bytes(start_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let start_json: Value = serde_json::from_slice(&start_body).unwrap();
    let session_id = start_json["session_id"].as_str().unwrap();

    // Resume the session
    let resume_request = Request::builder()
        .uri("/agent/resume")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "session_id": session_id
            })
            .to_string(),
        ))
        .unwrap();

    let resume_response = app.oneshot(resume_request).await.unwrap();
    assert_eq!(resume_response.status(), StatusCode::OK);

    let resume_body = body::to_bytes(resume_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let resume_json: Value = serde_json::from_slice(&resume_body).unwrap();

    // Should get back the same session
    assert_eq!(resume_json["session_id"].as_str().unwrap(), session_id);
    assert_eq!(
        resume_json["metadata"]["working_dir"].as_str().unwrap(),
        "/tmp/resume_test"
    );
}

#[tokio::test]
async fn test_tools_endpoint_with_session_isolation() {
    let (app, state) = create_test_app().await;

    // Create two sessions
    let session1_id = "test_session_tools_1";
    let session2_id = "test_session_tools_2";

    // Get agents for both sessions to ensure they exist
    let _ = state
        .get_agent(goose::session::Identifier::Name(session1_id.to_string()))
        .await
        .unwrap();
    let _ = state
        .get_agent(goose::session::Identifier::Name(session2_id.to_string()))
        .await
        .unwrap();

    // Get tools for session 1
    let tools1_request = Request::builder()
        .uri(&format!("/agent/tools?session_id={}", session1_id))
        .method("GET")
        .header("x-secret-key", "test-secret")
        .body(Body::empty())
        .unwrap();

    let tools1_response = app.clone().oneshot(tools1_request).await.unwrap();
    assert_eq!(tools1_response.status(), StatusCode::OK);

    let tools1_body = body::to_bytes(tools1_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tools1: Vec<Value> = serde_json::from_slice(&tools1_body).unwrap();

    // Get tools for session 2
    let tools2_request = Request::builder()
        .uri(&format!("/agent/tools?session_id={}", session2_id))
        .method("GET")
        .header("x-secret-key", "test-secret")
        .body(Body::empty())
        .unwrap();

    let tools2_response = app.oneshot(tools2_request).await.unwrap();
    assert_eq!(tools2_response.status(), StatusCode::OK);

    let tools2_body = body::to_bytes(tools2_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tools2: Vec<Value> = serde_json::from_slice(&tools2_body).unwrap();

    // Both should have base tools (at least platform tools)
    assert!(!tools1.is_empty());
    assert!(!tools2.is_empty());

    // Should have similar base tools
    let base_tool_names = [
        "platform__manage_extensions",
        "platform__search_available_extensions",
    ];
    for tool_name in &base_tool_names {
        assert!(tools1.iter().any(|t| t["name"] == *tool_name));
        assert!(tools2.iter().any(|t| t["name"] == *tool_name));
    }
}

#[tokio::test]
async fn test_update_provider_per_session() {
    let (app, state) = create_test_app().await;

    // Create two sessions
    let session1_id = "provider_test_1";
    let session2_id = "provider_test_2";

    // Ensure agents exist
    let _ = state
        .get_agent(goose::session::Identifier::Name(session1_id.to_string()))
        .await
        .unwrap();
    let _ = state
        .get_agent(goose::session::Identifier::Name(session2_id.to_string()))
        .await
        .unwrap();

    // Update provider for session 1 (this will fail but that's ok for the test)
    let update1_request = Request::builder()
        .uri("/agent/update_provider")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "session_id": session1_id,
                "provider": "openai",
                "model": "gpt-4"
            })
            .to_string(),
        ))
        .unwrap();

    let update1_response = app.clone().oneshot(update1_request).await.unwrap();
    // May fail due to missing API key, but the routing should work
    assert!(
        update1_response.status() == StatusCode::OK
            || update1_response.status() == StatusCode::BAD_REQUEST
    );

    // Update provider for session 2
    let update2_request = Request::builder()
        .uri("/agent/update_provider")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "session_id": session2_id,
                "provider": "anthropic",
                "model": "claude-3-sonnet"
            })
            .to_string(),
        ))
        .unwrap();

    let update2_response = app.oneshot(update2_request).await.unwrap();
    assert!(
        update2_response.status() == StatusCode::OK
            || update2_response.status() == StatusCode::BAD_REQUEST
    );
}
