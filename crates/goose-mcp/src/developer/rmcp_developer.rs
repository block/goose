use rmcp::{
    handler::server::router::tool::ToolRouter,
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

    /// List all available windows that can be used with screen_capture
    #[tool(name = "list_windows", description = "List all available window titles that can be used with screen_capture. Returns a list of window titles that can be used with the window_title parameter of the screen_capture tool.")]
    pub async fn list_windows(&self) -> Result<String, String> {
        // Get all windows using xcap
        let windows = Window::all().map_err(|e| {
            format!("Failed to list windows: {}", e)
        })?;

        // Extract window titles
        let window_titles: Vec<String> = windows
            .into_iter()
            .map(|w| w.title().to_string())
            .collect();

        // Create response content
        let window_list = if window_titles.is_empty() {
            "No windows available".to_string()
        } else {
            format!("Available windows:\n{}", window_titles.join("\n"))
        };

        Ok(window_list)
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
        
        let window_list = result.unwrap();
        assert!(window_list.contains("windows") || 
               window_list.contains("Windows") || 
               window_list.contains("available"));
    }

    #[test]
    fn test_server_basics() {
        let server = DeveloperServer::new();
        
        // Test that we can get tools from the router
        let tools = server.tool_router.list_all();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "list_windows");
        assert!(tools[0].description.as_ref().unwrap().contains("window"));
    }
}
