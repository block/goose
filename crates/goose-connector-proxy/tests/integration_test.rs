//! Integration tests for the connector proxy.
//!
//! These tests spin up both a mock custom LLM server and the proxy server,
//! then send OpenAI-format requests to the proxy and verify the responses.

use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde_json::Value;
use std::sync::Arc;
use tokio::net::TcpListener;

use goose_connector_proxy::models::ProxyConfig;
use goose_connector_proxy::start_proxy_server;

// ---------------------------------------------------------------------------
// Mock Custom LLM Server
// ---------------------------------------------------------------------------

/// State for the mock LLM server
#[derive(Clone)]
struct MockLlmState {
    /// The response the mock server will return
    response: Arc<Value>,
    /// Whether to use SSE streaming
    streaming: bool,
}

/// Handler that accepts custom LLM format requests and returns mock responses.
async fn mock_llm_handler(
    State(state): State<MockLlmState>,
    Json(request): Json<Value>,
) -> impl IntoResponse {
    // Validate the incoming request is in custom LLM format
    let contents = request.get("contents").and_then(|v| v.as_array());
    assert!(contents.is_some(), "Request should have 'contents' array");
    assert!(
        request.get("llmId").is_some(),
        "Request should have 'llmId'"
    );
    assert!(
        request.get("isStream").is_some(),
        "Request should have 'isStream'"
    );
    assert!(
        request.get("llmConfig").is_some(),
        "Request should have 'llmConfig'"
    );

    let is_stream = request
        .get("isStream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_stream && state.streaming {
        // Return SSE streaming response
        let response_content = state.response["content"]
            .as_str()
            .unwrap_or("hello");

        let sse_chunk = serde_json::json!({
            "content": response_content,
            "event_status": "DONE",
            "status": "SUCCESS",
            "prompt_token": 50,
            "completion_token": 10,
        });
        let sse_body = format!("data: {}\n\n", serde_json::to_string(&sse_chunk).unwrap());

        axum::http::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from(sse_body))
            .unwrap()
    } else {
        // Return non-streaming JSON response
        Json(state.response.as_ref().clone()).into_response()
    }
}

/// Start a mock custom LLM server and return its port.
async fn start_mock_llm(response: Value, streaming: bool) -> u16 {
    let state = MockLlmState {
        response: Arc::new(response),
        streaming,
    };

    let app = Router::new()
        .route("/api/v1/completions", post(mock_llm_handler))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    port
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_proxy_basic_non_streaming() {
    // Start mock LLM that returns a simple response
    let mock_response = serde_json::json!({
        "id": "resp-001",
        "status": "SUCCESS",
        "content": "Hello from mock LLM!",
        "promptToken": 50,
        "completionToken": 10,
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    // Start proxy pointing to mock LLM
    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();

    // Allow servers to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send OpenAI-format request to proxy
    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hello"}],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["message"]["content"], "Hello from mock LLM!");
    assert_eq!(body["choices"][0]["message"]["role"], "assistant");
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert_eq!(body["usage"]["prompt_tokens"], 50);
    assert_eq!(body["usage"]["completion_tokens"], 10);
    assert_eq!(body["usage"]["total_tokens"], 60);
    assert_eq!(body["object"], "chat.completion");
}

#[tokio::test]
async fn test_proxy_with_tool_calls() {
    // Mock LLM returns a response with <tool_call> tags
    let mock_response = serde_json::json!({
        "id": "resp-002",
        "status": "SUCCESS",
        "content": "Let me check the weather. <tool_call>{\"name\":\"get_weather\",\"arguments\":{\"city\":\"Seoul\"}}</tool_call>",
        "promptToken": 100,
        "completionToken": 30,
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "What's the weather in Seoul?"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather info",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "city": {"type": "string"}
                        }
                    }
                }
            }],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        body["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
        "get_weather"
    );
    let args_str = body["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
        .as_str()
        .unwrap();
    let args: Value = serde_json::from_str(args_str).unwrap();
    assert_eq!(args["city"], "Seoul");
}

#[tokio::test]
async fn test_proxy_multi_turn_with_tool_results() {
    // Mock LLM returns a response that uses tool result context
    let mock_response = serde_json::json!({
        "id": "resp-003",
        "status": "SUCCESS",
        "content": "The weather in Seoul is 15 degrees.",
        "promptToken": 200,
        "completionToken": 15,
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [
                {"role": "user", "content": "What's the weather in Seoul?"},
                {
                    "role": "assistant",
                    "content": "Let me check.",
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Seoul\"}"
                        }
                    }]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_abc123",
                    "content": "{\"temperature\": 15, \"condition\": \"sunny\"}"
                }
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather info",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "city": {"type": "string"}
                        }
                    }
                }
            }],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["finish_reason"], "stop");
    assert!(body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap()
        .contains("15"));
}

#[tokio::test]
async fn test_proxy_streaming_plain() {
    // Mock LLM returns streaming SSE response
    let mock_response = serde_json::json!({
        "content": "hello world",
    });
    let mock_port = start_mock_llm(mock_response, true).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("text/event-stream"));

    let body = resp.text().await.unwrap();
    // Should contain role chunk, content, finish, and [DONE]
    assert!(body.contains("\"role\":\"assistant\""));
    assert!(body.contains("hello world"));
    assert!(body.contains("[DONE]"));
}

#[tokio::test]
async fn test_proxy_streaming_with_tool_calls() {
    // Mock LLM returns streaming response with tool call
    let mock_response = serde_json::json!({
        "content": "<tool_call>{\"name\":\"read_file\",\"arguments\":{\"path\":\"/tmp/test.txt\"}}</tool_call>",
    });
    let mock_port = start_mock_llm(mock_response, true).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "read test file"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read a file",
                    "parameters": {
                        "type": "object",
                        "properties": {"path": {"type": "string"}}
                    }
                }
            }],
            "stream": true,
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    assert!(body.contains("\"role\":\"assistant\""));
    assert!(body.contains("read_file"));
    assert!(body.contains("tool_calls"));
    assert!(body.contains("[DONE]"));
}

#[tokio::test]
async fn test_proxy_models_endpoint() {
    let config = ProxyConfig {
        llm_url: "http://127.0.0.1:1/unused".to_string(),
        api_key: "test<|>test<|>test".to_string(),
        llm_id: "my-custom-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/v1/models", proxy_port))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["object"], "list");
    assert_eq!(body["data"][0]["id"], "my-custom-model");
    assert_eq!(body["data"][0]["object"], "model");
}

#[tokio::test]
async fn test_proxy_fail_response() {
    // Mock LLM returns FAIL status
    let mock_response = serde_json::json!({
        "status": "FAIL",
        "responseCode": "RATE_LIMIT",
        "content": "",
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hello"}],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 500);

    let body: Value = resp.json().await.unwrap();
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("FAIL"));
}

#[tokio::test]
async fn test_proxy_connection_error() {
    // Point to a non-existent server
    let config = ProxyConfig {
        llm_url: "http://127.0.0.1:1/api/v1/completions".to_string(),
        api_key: "test<|>test<|>test".to_string(),
        llm_id: "test-model".to_string(),
        timeout_secs: 2,
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hello"}],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 502);

    let body: Value = resp.json().await.unwrap();
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_proxy_structured_output() {
    // Mock LLM returns response with markdown code fences (structured output)
    let mock_response = serde_json::json!({
        "id": "resp-004",
        "status": "SUCCESS",
        "content": "```json\n{\"answer\": \"hello world\"}\n```",
        "promptToken": 30,
        "completionToken": 10,
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "respond in JSON"}],
            "response_format": {"type": "json_object"},
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    let content = body["choices"][0]["message"]["content"].as_str().unwrap();
    // Code fences should be stripped
    assert!(!content.contains("```"));
    // Should be valid JSON
    let parsed: Value = serde_json::from_str(content).unwrap();
    assert_eq!(parsed["answer"], "hello world");
}

#[tokio::test]
async fn test_proxy_system_message_augmentation() {
    // We need to verify that tool definitions get injected into the system prompt.
    // The mock LLM handler already validates the request format,
    // so this test just ensures the proxy handles the full flow.
    let mock_response = serde_json::json!({
        "id": "resp-005",
        "status": "SUCCESS",
        "content": "I see the tools available.",
        "promptToken": 100,
        "completionToken": 8,
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "hello"},
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "search",
                    "description": "Search the web",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string"}
                        }
                    }
                }
            }],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["message"]["content"], "I see the tools available.");
}

#[tokio::test]
async fn test_proxy_multiple_tool_calls() {
    // Mock LLM returns multiple tool calls
    let mock_response = serde_json::json!({
        "id": "resp-006",
        "status": "SUCCESS",
        "content": "I'll do both. <tool_call>{\"name\":\"read_file\",\"arguments\":{\"path\":\"a.txt\"}}</tool_call>\n<tool_call>{\"name\":\"write_file\",\"arguments\":{\"path\":\"b.txt\",\"content\":\"hello\"}}</tool_call>",
        "promptToken": 150,
        "completionToken": 40,
    });
    let mock_port = start_mock_llm(mock_response, false).await;

    let config = ProxyConfig {
        llm_url: format!("http://127.0.0.1:{}/api/v1/completions", mock_port),
        api_key: "test-client<|>test-token<|>test-user".to_string(),
        llm_id: "test-model".to_string(),
        ..Default::default()
    };
    let proxy_port = start_proxy_server(config).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            proxy_port
        ))
        .json(&serde_json::json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "read a.txt and write to b.txt"}],
            "tools": [
                {
                    "type": "function",
                    "function": {"name": "read_file", "description": "Read a file", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}}
                },
                {
                    "type": "function",
                    "function": {"name": "write_file", "description": "Write a file", "parameters": {"type": "object", "properties": {"path": {"type": "string"}, "content": {"type": "string"}}}}
                }
            ],
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["finish_reason"], "tool_calls");
    let tool_calls = body["choices"][0]["message"]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0]["function"]["name"], "read_file");
    assert_eq!(tool_calls[1]["function"]["name"], "write_file");
}
