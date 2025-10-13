#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::collections::HashMap;
    use goose::conversation::message::{Message, MessageContent, ToolResponse};
    use rmcp::model::Content;
    use crate::session::output::{render_message_with_timing, format_elapsed_time};

    #[test]
    fn test_render_tool_response_with_timing() {
        // Create a mock tool response
        let tool_response = ToolResponse {
            id: "test-tool-123".to_string(),
            tool_result: Ok(vec![Content::text("Test output")]),
        };

        // Create timing information
        let mut tool_timings = HashMap::new();
        tool_timings.insert("test-tool-123".to_string(), Duration::from_millis(1500));

        // Create a message with the tool response
        let message = Message::assistant().with_content(MessageContent::ToolResponse(tool_response));

        // This test mainly verifies the function doesn't panic and compiles correctly
        // In a real test environment, we'd capture stdout to verify the timing display
        render_message_with_timing(&message, false, &tool_timings);
    }

    #[test]
    fn test_format_elapsed_time_function_exists() {
        // Test that our format_elapsed_time function works correctly
        let duration = Duration::from_millis(1500);
        let formatted = format_elapsed_time(duration);
        assert_eq!(formatted, "1.50s");

        let duration = Duration::from_secs(75);
        let formatted = format_elapsed_time(duration);
        assert_eq!(formatted, "1m 15s");
    }
}