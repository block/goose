use goose::conversation::message::Message;
use goose::providers::create_with_named_model;
use goose::session_context;
use serde_json::json;
use serial_test::serial;
use std::sync::Arc;
use std::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Helper to capture headers from requests
#[derive(Clone, Default)]
struct HeaderCapture {
    captured_headers: Arc<Mutex<Vec<Option<String>>>>,
}

impl HeaderCapture {
    fn new() -> Self {
        Self {
            captured_headers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn capture_session_header(&self, req: &Request) {
        let session_id = req
            .headers
            .get("goose-session-id")
            .map(|v| v.to_str().unwrap().to_string());
        self.captured_headers.lock().unwrap().push(session_id);
    }

    fn get_captured(&self) -> Vec<Option<String>> {
        self.captured_headers.lock().unwrap().clone()
    }
}

/// Test that session ID is propagated via HTTP header when session exists
#[tokio::test]
#[serial]
async fn test_session_id_propagation_to_llm() {
    // Create mock server
    let mock_server = MockServer::start().await;
    let capture = HeaderCapture::new();
    let capture_clone = capture.clone();

    // Set up mock response for LLM API call
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |req: &Request| {
            capture_clone.capture_session_header(req);
            ResponseTemplate::new(200).set_body_json(json!({
                "id": "test-response-id",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4o-mini",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello from mock!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }))
        })
        .mount(&mock_server)
        .await;

    // Configure OpenAI provider to use mock server
    std::env::set_var("OPENAI_HOST", mock_server.uri());
    std::env::set_var("OPENAI_API_KEY", "test-key");

    // Create provider
    let provider = create_with_named_model("openai", "gpt-4o-mini")
        .await
        .expect("Should create provider");

    // Test: With session ID set
    let test_session_id = "integration-test-session-123";
    session_context::with_session_id(Some(test_session_id.to_string()), async {
        let message = Message::user().with_text("test message");

        let _response = provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await
            .expect("Request should succeed");

        // Verify the session ID header was captured
        let captured = capture.get_captured();
        assert_eq!(
            captured.len(),
            1,
            "Should have captured exactly one request"
        );
        assert_eq!(
            captured[0].as_ref(),
            Some(&test_session_id.to_string()),
            "Captured session ID should match"
        );
    })
    .await;

    // Clean up env vars
    std::env::remove_var("OPENAI_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}

/// Test that no session ID header is sent when session is not set
#[tokio::test]
#[serial]
async fn test_no_session_id_when_absent() {
    // Create mock server
    let mock_server = MockServer::start().await;
    let capture = HeaderCapture::new();
    let capture_clone = capture.clone();

    // Set up mock response
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |req: &Request| {
            capture_clone.capture_session_header(req);
            ResponseTemplate::new(200).set_body_json(json!({
                "id": "test-response-id",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4o-mini",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }))
        })
        .mount(&mock_server)
        .await;

    // Configure OpenAI provider to use mock server
    std::env::set_var("OPENAI_HOST", mock_server.uri());
    std::env::set_var("OPENAI_API_KEY", "test-key");

    // Create provider
    let provider = create_with_named_model("openai", "gpt-4o-mini")
        .await
        .expect("Should create provider");

    // Test: Without session ID set (not inside with_session_id scope)
    let message = Message::user().with_text("test message");

    let _response = provider
        .complete("You are a helpful assistant.", &[message], &[])
        .await
        .expect("Request should succeed");

    // Verify no session ID header was captured
    let captured = capture.get_captured();
    assert!(
        captured.is_empty() || captured[0].is_none(),
        "Should not have captured session ID when not set"
    );

    // Clean up env vars
    std::env::remove_var("OPENAI_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}

/// Test that session ID remains consistent across multiple calls
#[tokio::test]
#[serial]
async fn test_session_id_matches_across_calls() {
    // Create mock server
    let mock_server = MockServer::start().await;
    let capture = HeaderCapture::new();
    let capture_clone = capture.clone();

    // Set up mock response
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |req: &Request| {
            capture_clone.capture_session_header(req);
            ResponseTemplate::new(200).set_body_json(json!({
                "id": "test-response-id",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4o-mini",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }))
        })
        .mount(&mock_server)
        .await;

    // Configure OpenAI provider to use mock server
    std::env::set_var("OPENAI_HOST", mock_server.uri());
    std::env::set_var("OPENAI_API_KEY", "test-key");

    // Create provider
    let provider = create_with_named_model("openai", "gpt-4o-mini")
        .await
        .expect("Should create provider");

    // Test: Multiple calls within same session context
    let test_session_id = "consistent-session-456";
    session_context::with_session_id(Some(test_session_id.to_string()), async {
        let message = Message::user().with_text("test message");

        // Make first call
        let _response1 = provider
            .complete("You are a helpful assistant.", &[message.clone()], &[])
            .await
            .expect("First request should succeed");

        // Make second call
        let _response2 = provider
            .complete("You are a helpful assistant.", &[message.clone()], &[])
            .await
            .expect("Second request should succeed");

        // Make third call
        let _response3 = provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await
            .expect("Third request should succeed");

        // Verify all calls had the same session ID
        let captured = capture.get_captured();
        assert_eq!(captured.len(), 3, "Should have captured three requests");
        assert_eq!(
            captured[0].as_ref(),
            Some(&test_session_id.to_string()),
            "First call should have correct session ID"
        );
        assert_eq!(
            captured[1].as_ref(),
            Some(&test_session_id.to_string()),
            "Second call should have correct session ID"
        );
        assert_eq!(
            captured[2].as_ref(),
            Some(&test_session_id.to_string()),
            "Third call should have correct session ID"
        );
    })
    .await;

    // Clean up env vars
    std::env::remove_var("OPENAI_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}

/// Test that different session ID contexts have different session IDs
#[tokio::test]
#[serial]
async fn test_different_sessions_have_different_ids() {
    // Create mock server
    let mock_server = MockServer::start().await;
    let capture = HeaderCapture::new();
    let capture_clone = capture.clone();

    // Set up mock response
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |req: &Request| {
            capture_clone.capture_session_header(req);
            ResponseTemplate::new(200).set_body_json(json!({
                "id": "test-response-id",
                "object": "chat.completion",
                "created": 1234567890,
                "model": "gpt-4o-mini",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }))
        })
        .mount(&mock_server)
        .await;

    // Configure OpenAI provider to use mock server
    std::env::set_var("OPENAI_HOST", mock_server.uri());
    std::env::set_var("OPENAI_API_KEY", "test-key");

    // Create provider
    let provider = create_with_named_model("openai", "gpt-4o-mini")
        .await
        .expect("Should create provider");

    let message = Message::user().with_text("test message");

    // First session
    let session_id_1 = "session-one";
    session_context::with_session_id(Some(session_id_1.to_string()), async {
        let _response = provider
            .complete("You are a helpful assistant.", &[message.clone()], &[])
            .await
            .expect("Request should succeed");
    })
    .await;

    // Second session
    let session_id_2 = "session-two";
    session_context::with_session_id(Some(session_id_2.to_string()), async {
        let _response = provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await
            .expect("Request should succeed");
    })
    .await;

    // Verify different session IDs were captured
    let captured = capture.get_captured();
    assert_eq!(captured.len(), 2, "Should have captured two requests");
    assert_eq!(
        captured[0].as_ref(),
        Some(&session_id_1.to_string()),
        "First session ID should match"
    );
    assert_eq!(
        captured[1].as_ref(),
        Some(&session_id_2.to_string()),
        "Second session ID should match"
    );
    assert_ne!(captured[0], captured[1], "Session IDs should be different");

    // Clean up env vars
    std::env::remove_var("OPENAI_HOST");
    std::env::remove_var("OPENAI_API_KEY");
}