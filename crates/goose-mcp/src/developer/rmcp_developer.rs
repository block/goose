use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{Content, Role, CallToolResult, ErrorData, ErrorCode},
    schemars::JsonSchema,
    tool, tool_handler, tool_router, ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, io::Cursor};
use base64::Engine;
use xcap::{Monitor, Window};

/// Parameters for the screen_capture tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ScreenCaptureParams {
    /// The display number to capture (0 is main display)
    #[serde(default)]
    pub display: Option<u64>,
    
    /// Optional: the exact title of the window to capture. 
    /// Use the list_windows tool to find the available windows.
    pub window_title: Option<String>,
}

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
        let windows = Window::all().map_err(|_| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                "Failed to list windows".to_string(),
                None,
            )
        })?;

        let window_titles: Vec<String> = windows
            .into_iter()
            .map(|w| w.title().to_string())
            .collect();

        let content_text = format!("Available windows:\n{}", window_titles.join("\n"));

        Ok(CallToolResult::success(vec![
            Content::text(content_text.clone())
                .with_audience(vec![Role::Assistant]),
            Content::text(content_text)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    /// Capture a screenshot of a specified display or window.
    /// You can capture either:
    /// 1. A full display (monitor) using the display parameter
    /// 2. A specific window by its title using the window_title parameter
    ///
    /// Only one of display or window_title should be specified.
    #[tool(
        name = "screen_capture",
        description = "Capture a screenshot of a specified display or window. You can capture either: 1. A full display (monitor) using the display parameter 2. A specific window by its title using the window_title parameter. Only one of display or window_title should be specified."
    )]
    pub async fn screen_capture(
        &self,
        params: Parameters<ScreenCaptureParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let params = params.0;

        let mut image = if let Some(window_title) = &params.window_title {
            // Try to find and capture the specified window
            let windows = Window::all().map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to list windows".to_string(),
                    None,
                )
            })?;

            let window = windows
                .into_iter()
                .find(|w| w.title() == window_title)
                .ok_or_else(|| {
                    ErrorData::new(
                        ErrorCode::INTERNAL_ERROR,
                        format!("No window found with title '{}'", window_title),
                        None,
                    )
                })?;

            window.capture_image().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to capture window '{}': {}", window_title, e),
                    None,
                )
            })?
        } else {
            // Default to display capture if no window title is specified
            let display = params.display.unwrap_or(0) as usize;

            let monitors = Monitor::all().map_err(|_| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to access monitors".to_string(),
                    None,
                )
            })?;
            
            let monitor = monitors.get(display).ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!(
                        "{} was not an available monitor, {} found.",
                        display,
                        monitors.len()
                    ),
                    None,
                )
            })?;

            monitor.capture_image().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to capture display {}: {}", display, e),
                    None,
                )
            })?
        };

        // Resize the image to a reasonable width while maintaining aspect ratio
        let max_width = 768;
        if image.width() > max_width {
            let scale = max_width as f32 / image.width() as f32;
            let new_height = (image.height() as f32 * scale) as u32;
            image = xcap::image::imageops::resize(
                &image,
                max_width,
                new_height,
                xcap::image::imageops::FilterType::Lanczos3,
            );
        }

        let mut bytes: Vec<u8> = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), xcap::image::ImageFormat::Png)
            .map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to write image buffer {}", e),
                    None,
                )
            })?;

        // Convert to base64
        let data = base64::prelude::BASE64_STANDARD.encode(bytes);

        // Return two Content objects like the old implementation:
        // one text for Assistant, one image with priority 0.0
        Ok(CallToolResult::success(vec![
            Content::text("Screenshot captured")
                .with_audience(vec![Role::Assistant]),
            Content::image(data, "image/png")
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
        assert_eq!(tools.len(), 2); // Now has both list_windows and screen_capture
        
        // Find the tools by name
        let list_windows_tool = tools.iter().find(|t| t.name == "list_windows").unwrap();
        let screen_capture_tool = tools.iter().find(|t| t.name == "screen_capture").unwrap();
        
        // Verify list_windows tool
        assert!(list_windows_tool.description.as_ref().unwrap().contains("window"));
        assert!(list_windows_tool.description.as_ref().unwrap().contains("screen_capture"));
        
        // Verify screen_capture tool  
        assert!(screen_capture_tool.description.as_ref().unwrap().contains("screenshot"));
        assert!(screen_capture_tool.description.as_ref().unwrap().contains("display"));
        assert!(screen_capture_tool.description.as_ref().unwrap().contains("window_title"));
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

    #[tokio::test]
    async fn test_screen_capture_default_display() {
        let server = DeveloperServer::new();
        let params = Parameters(ScreenCaptureParams {
            display: None, // Should default to 0
            window_title: None,
        });
        
        // Note: This test may fail on systems without a display/in CI
        // but it tests the parameter handling and basic structure
        let result = server.screen_capture(params).await;
        
        match result {
            Ok(tool_result) => {
                // Verify successful result structure
                assert_eq!(tool_result.is_error, Some(false));
                assert!(tool_result.validate().is_ok());
                assert!(tool_result.content.is_some());
                
                let content_vec = tool_result.content.unwrap();
                assert_eq!(content_vec.len(), 2);
                
                // First should be text "Screenshot captured"
                if let Some(text_content) = content_vec[0].as_text() {
                    assert_eq!(text_content.text, "Screenshot captured");
                } else {
                    panic!("Expected first content to be text");
                }
                
                // Second should be image content with PNG MIME type
                if let Some(image_content) = content_vec[1].as_image() {
                    assert_eq!(image_content.mime_type, "image/png");
                    assert!(!image_content.data.is_empty());
                } else {
                    panic!("Expected second content to be image");
                }
            }
            Err(error) => {
                // If it fails, verify the error format matches the old implementation
                assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
                // Could be "Failed to access monitors" or capture-related error
                assert!(error.message.contains("Failed to") || error.message.contains("monitor"));
            }
        }
    }

    #[tokio::test]
    async fn test_screen_capture_specific_display() {
        let server = DeveloperServer::new();
        let params = Parameters(ScreenCaptureParams {
            display: Some(0), // Explicit display 0
            window_title: None,
        });
        
        let result = server.screen_capture(params).await;
        
        match result {
            Ok(tool_result) => {
                // Same validation as default display test
                assert_eq!(tool_result.is_error, Some(false));
                assert!(tool_result.validate().is_ok());
                
                let content_vec = tool_result.content.unwrap();
                assert_eq!(content_vec.len(), 2);
            }
            Err(error) => {
                // Verify error structure
                assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
            }
        }
    }

    #[tokio::test]
    async fn test_screen_capture_invalid_display() {
        let server = DeveloperServer::new();
        let params = Parameters(ScreenCaptureParams {
            display: Some(999), // Invalid display number
            window_title: None,
        });
        
        let result = server.screen_capture(params).await;
        
        // This should fail with a specific error about the monitor not being available
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert!(error.message.contains("was not an available monitor") || 
               error.message.contains("Failed to access monitors"));
    }

    #[tokio::test]
    async fn test_screen_capture_invalid_window() {
        let server = DeveloperServer::new();
        let params = Parameters(ScreenCaptureParams {
            display: None,
            window_title: Some("NonExistentWindow12345".to_string()),
        });
        
        let result = server.screen_capture(params).await;
        
        // This should fail with a specific error about the window not being found
        match result {
            Ok(_) => panic!("Expected error for non-existent window"),
            Err(error) => {
                assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
                assert!(error.message.contains("No window found with title") ||
                       error.message.contains("Failed to list windows"));
            }
        }
    }

    #[tokio::test]
    async fn test_screen_capture_parameter_validation() {
        let server = DeveloperServer::new();
        
        // Test with both parameters provided (should work - window_title takes precedence)
        let params = Parameters(ScreenCaptureParams {
            display: Some(0),
            window_title: Some("SomeWindow".to_string()),
        });
        
        let result = server.screen_capture(params).await;
        
        // If it fails, it should be because the window doesn't exist, not parameter validation
        match result {
            Ok(_) => {
                // Window was found and captured successfully
            }
            Err(error) => {
                // Should be window-not-found or capture error, not parameter error
                assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
                assert!(error.message.contains("No window found") || 
                       error.message.contains("Failed to") ||
                       error.message.contains("list windows"));
            }
        }
    }

    #[test]
    fn test_screen_capture_params_serialization() {
        // Test parameter structure serialization/deserialization
        use serde_json;
        
        // Test with display only
        let params1 = ScreenCaptureParams {
            display: Some(1),
            window_title: None,
        };
        let json1 = serde_json::to_string(&params1).unwrap();
        let parsed1: ScreenCaptureParams = serde_json::from_str(&json1).unwrap();
        assert_eq!(parsed1.display, Some(1));
        assert_eq!(parsed1.window_title, None);
        
        // Test with window_title only
        let params2 = ScreenCaptureParams {
            display: None,
            window_title: Some("Test Window".to_string()),
        };
        let json2 = serde_json::to_string(&params2).unwrap();
        let parsed2: ScreenCaptureParams = serde_json::from_str(&json2).unwrap();
        assert_eq!(parsed2.display, None);
        assert_eq!(parsed2.window_title, Some("Test Window".to_string()));
        
        // Test with empty params (should use defaults)
        let params3 = ScreenCaptureParams {
            display: None,
            window_title: None,
        };
        let json3 = serde_json::to_string(&params3).unwrap();
        let parsed3: ScreenCaptureParams = serde_json::from_str(&json3).unwrap();
        assert_eq!(parsed3.display, None);
        assert_eq!(parsed3.window_title, None);
    }
}
