use goose::conversation::message::ToolRequest;
use mcp_core::ToolCall;
use rmcp::model::{ErrorCode, ErrorData};
use serde_json::json;

#[test]
fn tool_request_to_readable_string_formats_success_and_error() {
    // Success case: pretty prints tool name and JSON arguments
    let ok_request = ToolRequest {
        id: "req_ok".to_string(),
        tool_call: Ok(ToolCall {
            name: "example_tool".to_string(),
            arguments: json!({
                "x": 1,
                "y": "z"
            }),
        }),
    };

    let readable_ok = ok_request.to_readable_string();
    // Should contain the tool name
    assert!(readable_ok.starts_with("Tool: example_tool, Args: "));
    // Should contain pretty-printed JSON keys and values
    assert!(
        readable_ok.contains("\"x\": 1"),
        "readable_ok was: {}",
        readable_ok
    );
    assert!(
        readable_ok.contains("\"y\": \"z\""),
        "readable_ok was: {}",
        readable_ok
    );

    // Error case: includes error display with code and message
    let err_request = ToolRequest {
        id: "req_err".to_string(),
        tool_call: Err(ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: std::borrow::Cow::from("Oops"),
            data: None,
        }),
    };

    let readable_err = err_request.to_readable_string();
    assert!(readable_err.starts_with("Invalid tool call: "));
    assert!(
        readable_err.contains("Oops"),
        "readable_err was: {}",
        readable_err
    );
    // INTERNAL_ERROR is JSON-RPC -32603; ensure code is surfaced
    assert!(
        readable_err.contains("-32603"),
        "readable_err was: {}",
        readable_err
    );
}
