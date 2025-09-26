use goose::conversation::message::MessageContent;
use goose::conversation::message::ToolResponse;
use mcp_core::ToolResult;
use rmcp::model::{Content, ErrorCode, ErrorData};

// This test targets MessageContent::as_tool_response_text
// It verifies that:
// - tool response text contents are concatenated with newlines
// - non-successful tool results (Err) yield None
#[test]
fn test_as_tool_response_text_concatenates_and_handles_errors() {
    // Successful tool response with two text contents
    let resp_ok = ToolResponse {
        id: "resp1".to_string(),
        tool_result: ToolResult::Ok(vec![
            Content::text("First line".to_string()),
            Content::text("Second line".to_string()),
        ]),
    };
    let mc_ok = MessageContent::ToolResponse(resp_ok);

    let extracted = mc_ok.as_tool_response_text();
    assert!(extracted.is_some());
    assert_eq!(extracted.unwrap(), "First line\nSecond line");

    // Error tool response should return None
    let resp_err = ToolResponse {
        id: "resp2".to_string(),
        tool_result: ToolResult::Err(ErrorData {
            code: ErrorCode::INTERNAL_ERROR,
            message: std::borrow::Cow::from("tool failed".to_string()),
            data: None,
        }),
    };
    let mc_err = MessageContent::ToolResponse(resp_err);
    assert!(mc_err.as_tool_response_text().is_none());
}
