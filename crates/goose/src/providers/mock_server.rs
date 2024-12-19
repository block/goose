use mcp_core::Tool;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const TEST_INPUT_TOKENS: i32 = 12;
pub const TEST_OUTPUT_TOKENS: i32 = 15;
pub const TEST_TOTAL_TOKENS: i32 = 27;
pub const TEST_TOOL_FUNCTION_NAME: &str = "get_weather";
pub const TEST_TOOL_FUNCTION_ARGUMENTS: &str = "{\"location\":\"San Francisco, CA\"}";

pub async fn setup_mock_server(path_url: &str, response_body: Value) -> MockServer {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(path_url))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;
    mock_server
}

pub async fn setup_mock_server_with_response_code(
    path_url: &str,
    response_code: u16,
) -> MockServer {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(path_url))
        .respond_with(ResponseTemplate::new(response_code))
        .mount(&mock_server)
        .await;
    mock_server
}
pub fn create_mock_open_ai_response_with_tools(model_name: &str) -> Value {
    json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_123",
                    "type": "function",
                    "function": {
                        "name": TEST_TOOL_FUNCTION_NAME,
                        "arguments": TEST_TOOL_FUNCTION_ARGUMENTS
                    }
                }]
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": TEST_INPUT_TOKENS,
            "completion_tokens": TEST_OUTPUT_TOKENS,
            "total_tokens": TEST_TOTAL_TOKENS
        },
        "model": model_name
    })
}

pub fn create_mock_google_ai_response_with_tools(model_name: &str) -> Value {
    json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "functionCall": {
                        "name": TEST_TOOL_FUNCTION_NAME,
                        "args":{
                            "location": "San Francisco, CA"
                        }

                    }
                }],
                "role": "model"
            },
            "finishReason": "STOP"
        }],
        "modelVersion": model_name,
        "usageMetadata": {
            "candidatesTokenCount": TEST_OUTPUT_TOKENS,
            "promptTokenCount": TEST_INPUT_TOKENS,
            "totalTokenCount": TEST_TOTAL_TOKENS
        }
    })
}

pub fn create_mock_google_ai_response(model_name: &str, content: &str) -> Value {
    json!({
        "candidates": [{
            "content": {
                "parts": [{
                    "text": content
                }],
                "role": "model"
            },
            "finishReason": "STOP"
        }],
        "modelVersion": model_name,
        "usageMetadata": {
            "candidatesTokenCount": TEST_OUTPUT_TOKENS,
            "promptTokenCount": TEST_INPUT_TOKENS,
            "totalTokenCount": TEST_TOTAL_TOKENS
        }
    })
}

pub fn create_mock_open_ai_response(model_name: &str, content: &str) -> Value {
    json!({
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
                "tool_calls": null
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": TEST_INPUT_TOKENS,
            "completion_tokens": TEST_OUTPUT_TOKENS,
            "total_tokens": TEST_TOTAL_TOKENS
        },
        "model": model_name
    })
}

pub fn create_test_tool() -> Tool {
    Tool::new(
        "get_weather",
        "Gets the current weather for a location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. New York, NY"
                }
            },
            "required": ["location"]
        }),
    )
}

pub fn get_expected_function_call_arguments() -> Value {
    json!({
        "location": "San Francisco, CA"
    })
}
