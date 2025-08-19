use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{Content, Role, CallToolResult, ErrorData, ErrorCode},
    tool, tool_handler, tool_router, ServerHandler,
};
use std::future::Future;
use xcap::Window;

/// Developer MCP Server using official RMCP SDK
#[derive(Debug, Clone)]
pub struct DeveloperServer {
    tool_router: ToolRouter<Self>,
}

impl Default for DeveloperServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DeveloperServer {}

#[tool_router(router = tool_router)]
impl DeveloperServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// List all available windows that can be used with screen_capture.
    /// Returns a list of window titles that can be used with the window_title parameter
    /// of the screen_capture tool.
    #[tool(
        name = "list_windows", 
        description = "List all available window titles that can be used with screen_capture. Returns a list of window titles that can be used with the window_title parameter of the screen_capture tool."
    )]
    pub async fn list_windows(&self) -> Result<CallToolResult, ErrorData> {
        // Get all windows using xcap
        let windows = Window::all().map_err(|_| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to list windows".to_string(),
                None,
            )
        })?;

        // Extract window titles - identical to old implementation
        let window_titles: Vec<String> = windows
            .into_iter()
            .map(|w| w.title().to_string())
            .collect();

        // Create response content - identical format to old implementation
        let content_text = format!("Available windows:\n{}", window_titles.join("\n"));

        // Return two Content objects like the old implementation:
        // one for Assistant, one for User with priority 0.0
        Ok(CallToolResult::success(vec![
            Content::text(content_text.clone())
                .with_audience(vec![Role::Assistant]),
            Content::text(content_text)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_windows_tool() {
        let server = DeveloperServer::new();
        
        let result = server.list_windows().await;
        assert!(result.is_ok());
        
        let tool_result = result.unwrap();
        
        // Check that it's a successful result (not an error)
        assert_eq!(tool_result.is_error, Some(false));
        
        // Verify that validation passes
        assert!(tool_result.validate().is_ok());
        
        // Should have content (not structured_content)
        assert!(tool_result.content.is_some());
        assert!(tool_result.structured_content.is_none());
        
        let content_vec = tool_result.content.unwrap();
        
        // Should return exactly 2 content objects like the old implementation
        assert_eq!(content_vec.len(), 2);
        
        // Both should be text content with "Available windows:" format
        for content in &content_vec {
            if let Some(text_content) = content.as_text() {
                assert!(text_content.text.contains("Available windows:"));
            } else {
                panic!("Expected text content");
            }
        }
        
        // Verify both content objects have the same text (like the old implementation)
        let first_content = &content_vec[0];
        let second_content = &content_vec[1];
        
        if let (Some(first_text), Some(second_text)) = (first_content.as_text(), second_content.as_text()) {
            assert_eq!(first_text.text, second_text.text);
        } else {
            panic!("Expected both contents to be text");
        }
    }

    #[test]
    fn test_server_basics() {
        let server = DeveloperServer::new();
        
        // Test that we can get tools from the router
        let tools = server.tool_router.list_all();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "list_windows");
        assert!(tools[0].description.as_ref().unwrap().contains("window"));
        assert!(tools[0].description.as_ref().unwrap().contains("screen_capture"));
    }

    #[tokio::test]
    async fn test_list_windows_error_handling() {
        // This test verifies that the error handling matches the old implementation
        // The actual Window::all() call might succeed, but we're testing the error format
        let server = DeveloperServer::new();
        
        // Even if this succeeds, we verify the function signature and error type match
        let result = server.list_windows().await;
        
        // If it fails, it should be an ErrorData with INTERNAL_ERROR code
        if let Err(error) = result {
            assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
            assert_eq!(error.message, "Failed to list windows");
        }
        // If it succeeds, that's fine too - we just want to ensure compatibility
    }

    #[tokio::test]
    async fn test_list_windows_content_format() {
        let server = DeveloperServer::new();
        
        let result = server.list_windows().await;
        assert!(result.is_ok());
        
        let tool_result = result.unwrap();
        
        // Get the content vector and check the first item
        if let Some(content_vec) = &tool_result.content {
            let content = &content_vec[0];
            
            if let Some(text_content) = content.as_text() {
                // Should always start with "Available windows:" (matching old implementation)
                assert!(text_content.text.starts_with("Available windows:"));
                
                // Should contain newline-separated window titles
                assert!(text_content.text.contains('\n'));
            }
        } else {
            panic!("Expected content to be present");
        }
    }
}
