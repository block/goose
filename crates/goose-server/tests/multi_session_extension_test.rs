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

    // Set up scheduler
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
async fn test_extension_add_isolation_between_sessions() {
    let (app, state) = create_test_app().await;

    // Create two sessions
    let session1_id = "ext_isolation_1";
    let session2_id = "ext_isolation_2";

    // Ensure agents exist
    let _ = state
        .get_agent(goose::session::Identifier::Name(session1_id.to_string()))
        .await
        .unwrap();
    let _ = state
        .get_agent(goose::session::Identifier::Name(session2_id.to_string()))
        .await
        .unwrap();

    // Add a frontend extension to session 1 only
    let add_ext_request = Request::builder()
        .uri("/extensions/add")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "session_id": session1_id,
                "type": "frontend",
                "name": "test_extension",
                "tools": [
                    {
                        "name": "custom_tool",
                        "description": "A custom test tool",
                        "input_schema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ],
                "instructions": "Test extension for session 1"
            })
            .to_string(),
        ))
        .unwrap();

    let add_response = app.clone().oneshot(add_ext_request).await.unwrap();
    assert_eq!(add_response.status(), StatusCode::OK);

    // Check tools for session 1 - should have the custom tool
    let tools1_request = Request::builder()
        .uri(&format!("/agent/tools?session_id={}", session1_id))
        .method("GET")
        .header("x-secret-key", "test-secret")
        .body(Body::empty())
        .unwrap();

    let tools1_response = app.clone().oneshot(tools1_request).await.unwrap();
    let tools1_body = body::to_bytes(tools1_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tools1: Vec<Value> = serde_json::from_slice(&tools1_body).unwrap();

    // Check tools for session 2 - should NOT have the custom tool
    let tools2_request = Request::builder()
        .uri(&format!("/agent/tools?session_id={}", session2_id))
        .method("GET")
        .header("x-secret-key", "test-secret")
        .body(Body::empty())
        .unwrap();

    let tools2_response = app.oneshot(tools2_request).await.unwrap();
    let tools2_body = body::to_bytes(tools2_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tools2: Vec<Value> = serde_json::from_slice(&tools2_body).unwrap();

    // Session 1 should have custom_tool
    assert!(
        tools1.iter().any(|t| t["name"] == "custom_tool"),
        "Session 1 should have custom_tool"
    );

    // Session 2 should NOT have custom_tool
    assert!(
        !tools2.iter().any(|t| t["name"] == "custom_tool"),
        "Session 2 should not have custom_tool"
    );
}

#[tokio::test]
async fn test_extension_remove_isolation() {
    let (app, state) = create_test_app().await;

    let session1_id = "ext_remove_1";
    let session2_id = "ext_remove_2";

    // Create agents
    let agent1 = state
        .get_agent(goose::session::Identifier::Name(session1_id.to_string()))
        .await
        .unwrap();
    let agent2 = state
        .get_agent(goose::session::Identifier::Name(session2_id.to_string()))
        .await
        .unwrap();

    // Add same extension to both sessions directly via agent
    let ext_config = goose::agents::ExtensionConfig::Frontend {
        name: "shared_ext".to_string(),
        tools: vec![rmcp::model::Tool {
            name: "shared_tool".into(),
            description: Some("Shared tool".into()),
            input_schema: Arc::new(json!({"type": "object"}).as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
        }],
        instructions: Some("Shared extension".to_string()),
        bundled: Some(false),
        available_tools: vec![],
    };

    agent1.add_extension(ext_config.clone()).await.unwrap();
    agent2.add_extension(ext_config).await.unwrap();

    // Verify both have the extension
    let tools1 = agent1.list_tools(None).await;
    let tools2 = agent2.list_tools(None).await;

    assert!(tools1.iter().any(|t| t.name == "shared_tool"));
    assert!(tools2.iter().any(|t| t.name == "shared_tool"));

    // Remove extension from session 1 via API
    let remove_request = Request::builder()
        .uri("/extensions/remove")
        .method("POST")
        .header("content-type", "application/json")
        .header("x-secret-key", "test-secret")
        .body(Body::from(
            json!({
                "session_id": session1_id,
                "name": "shared_ext"
            })
            .to_string(),
        ))
        .unwrap();

    let remove_response = app.oneshot(remove_request).await.unwrap();
    assert_eq!(remove_response.status(), StatusCode::OK);

    // Check tools again
    let tools1_after = agent1.list_tools(None).await;
    let tools2_after = agent2.list_tools(None).await;

    // Session 1 should NOT have the tool anymore
    assert!(!tools1_after.iter().any(|t| t.name == "shared_tool"));

    // Session 2 should still have it
    assert!(tools2_after.iter().any(|t| t.name == "shared_tool"));
}

#[tokio::test]
async fn test_concurrent_extension_operations_via_api() {
    let (app, state) = create_test_app().await;

    // Create multiple sessions
    let mut handles = vec![];

    for i in 0..5 {
        let app_clone = app.clone();
        let state_clone = state.clone();

        let handle = tokio::spawn(async move {
            let session_id = format!("concurrent_session_{}", i);

            // Ensure agent exists
            let _ = state_clone
                .get_agent(goose::session::Identifier::Name(session_id.clone()))
                .await
                .unwrap();

            // Add extension via API
            let add_request = Request::builder()
                .uri("/extensions/add")
                .method("POST")
                .header("content-type", "application/json")
                .header("x-secret-key", "test-secret")
                .body(Body::from(
                    json!({
                        "session_id": &session_id,
                        "type": "frontend",
                        "name": format!("ext_{}", i),
                        "tools": [
                            {
                                "name": format!("tool_{}", i),
                                "description": format!("Tool for session {}", i),
                                "input_schema": {"type": "object"}
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap();

            let add_response = app_clone.clone().oneshot(add_request).await.unwrap();
            assert_eq!(add_response.status(), StatusCode::OK);

            // Verify tool was added
            let tools_request = Request::builder()
                .uri(&format!("/agent/tools?session_id={}", session_id))
                .method("GET")
                .header("x-secret-key", "test-secret")
                .body(Body::empty())
                .unwrap();

            let tools_response = app_clone.oneshot(tools_request).await.unwrap();
            let tools_body = body::to_bytes(tools_response.into_body(), usize::MAX)
                .await
                .unwrap();
            let tools: Vec<Value> = serde_json::from_slice(&tools_body).unwrap();

            // Should have the session-specific tool
            assert!(
                tools.iter().any(|t| t["name"] == format!("tool_{}", i)),
                "Session {} should have tool_{}",
                i,
                i
            );

            // Should NOT have tools from other sessions
            for j in 0..5 {
                if j != i {
                    assert!(
                        !tools.iter().any(|t| t["name"] == format!("tool_{}", j)),
                        "Session {} should not have tool_{}",
                        i,
                        j
                    );
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}
