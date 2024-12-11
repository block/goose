use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use base64::Engine;
use headless_chrome::{
    Browser, LaunchOptions, Tab,
};
use image::{imageops::FilterType, GenericImageView};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::errors::{AgentError, AgentResult};
use crate::models::content::Content;
use crate::models::tool::Tool;
use crate::models::tool::ToolCall;
use crate::systems::{Resource, System};

pub struct WebBrowserSystem {
    tools: Vec<Tool>,
    browser: Arc<Mutex<Option<Browser>>>,
    tab: Arc<Mutex<Option<Arc<Tab>>>>,
    instructions: String,
}

impl Default for WebBrowserSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WebBrowserSystem {
    pub fn new() -> Self {
        let navigate_tool = Tool::new(
            "navigate",
            "Navigate to a URL in the browser",
            json!({
                "type": "object",
                "required": ["url"],
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to navigate to"
                    },
                    "wait_for": {
                        "type": "string",
                        "default": null,
                        "description": "Optional CSS selector to wait for before continuing"
                    }
                }
            }),
        );

        let screenshot_tool = Tool::new(
            "screenshot",
            "Take a screenshot of the current page or element",
            json!({
                "type": "object",
                "required": [],
                "properties": {
                    "max_width": {
                        "type": "integer",
                        "default": null,
                        "description": "Maximum width of the screenshot in pixels. Aspect ratio will be preserved."
                    }
                }
            }),
        );

        let click_tool = Tool::new(
            "click",
            "Click on an element in the page",
            json!({
                "type": "object",
                "required": ["selector"],
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to click"
                    },
                    "wait_for": {
                        "type": "string",
                        "default": null,
                        "description": "Optional CSS selector to wait for after clicking"
                    }
                }
            }),
        );

        let type_tool = Tool::new(
            "type",
            "Type text into an input element",
            json!({
                "type": "object",
                "required": ["selector", "text"],
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the input element"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type into the element"
                    },
                    "clear_first": {
                        "type": "boolean",
                        "default": true,
                        "description": "Whether to clear the input before typing"
                    }
                }
            }),
        );

        let eval_tool = Tool::new(
            "eval",
            "Evaluate JavaScript in the page context",
            json!({
                "type": "object",
                "required": ["script"],
                "properties": {
                    "script": {
                        "type": "string",
                        "description": "JavaScript code to evaluate"
                    }
                }
            }),
        );

        let wait_for_tool = Tool::new(
            "wait_for",
            "Wait for an element to appear",
            json!({
                "type": "object",
                "required": ["selector"],
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to wait for"
                    },
                    "timeout": {
                        "type": "integer",
                        "default": 30000,
                        "description": "Maximum time to wait in milliseconds"
                    }
                }
            }),
        );

        let instructions = indoc::formatdoc! {r#"
            The web browser system provides automation capabilities using headless Chrome.
            
            Available tools:
            - navigate: Load a URL in the browser with optional wait conditions
            - screenshot: Capture the current page if needed to visually examine
            - click: Click on elements using CSS selectors
            - type: Enter text into input fields
            - eval: Execute JavaScript in the page context
            - wait_for: Wait for elements to appear
            
            Notes:
            - The browser session persists between commands
            - Screenshots are returned as base64-encoded PNG images
            - CSS selectors must be valid and match exactly one element
            - JavaScript evaluation runs in the page context
            - All commands support various wait conditions for reliability
            "#};

        Self {
            tools: vec![
                navigate_tool,
                screenshot_tool,
                click_tool,
                type_tool,
                eval_tool,
                wait_for_tool,
            ],
            browser: Arc::new(Mutex::new(None)),
            tab: Arc::new(Mutex::new(None)),
            instructions,
        }
    }

    async fn ensure_browser(&self) -> AgentResult<()> {
        let mut browser_guard = self.browser.lock().await;
        if browser_guard.is_none() {
            let options = LaunchOptions::default_builder()
                .window_size(Some((1920, 1080)))
                .headless(true)
                .build()
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

            let browser = Browser::new(options)
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

            // Create initial tab
            let tab = browser
                .new_tab()
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

            *browser_guard = Some(browser);
            *self.tab.lock().await = Some(tab);
        }
        Ok(())
    }

    async fn navigate(&self, params: Value) -> AgentResult<Vec<Content>> {
        self.ensure_browser().await?;
        
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("URL parameter is required".into()))?;

        let tab = self.tab.lock().await;
        let tab = tab.as_ref().unwrap();
        
        tab.navigate_to(url)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Wait for page to load
        tab.wait_for_element("body")
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Wait for specific element if requested
        if let Some(wait_for) = params.get("wait_for").and_then(|v| v.as_str()) {
            tab.wait_for_element(wait_for)
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
        }

        Ok(vec![Content::text(format!("Navigated to {}", url))])
    }

    async fn screenshot(&self, params: Value) -> AgentResult<Vec<Content>> {
        self.ensure_browser().await?;

        let tab = self.tab.lock().await;
        let tab = tab.as_ref().unwrap();

        let screenshot_data = tab.capture_screenshot(
            headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        )
        .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Convert the screenshot data to an image
        let img = image::load_from_memory(&screenshot_data)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to load image: {}", e)))?;

        let final_image = if let Some(max_width) = params.get("max_width").and_then(|v| v.as_u64()) {
            let max_width = max_width as u32;
            let (width, height) = img.dimensions();
            
            if width > max_width {
                // Calculate new height while preserving aspect ratio
                let aspect_ratio = width as f32 / height as f32;
                let new_height = (max_width as f32 / aspect_ratio) as u32;
                
                // Resize the image
                img.resize(max_width, new_height, FilterType::Lanczos3)
            } else {
                img
            }
        } else {
            img
        };

        // Convert the image back to PNG format
        let mut png_data = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_data);
        final_image
            .write_to(&mut cursor, image::ImageOutputFormat::Png)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to encode image: {}", e)))?;

        let base64 = base64::prelude::BASE64_STANDARD.encode(&png_data);
        Ok(vec![Content::image(base64, "image/png")])
    }

    async fn click(&self, params: Value) -> AgentResult<Vec<Content>> {
        self.ensure_browser().await?;

        let selector = params
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("selector parameter is required".into()))?;

        let tab = self.tab.lock().await;
        let tab = tab.as_ref().unwrap();

        let element = tab.wait_for_element(selector)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        element.click()
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Wait for specific element after click if requested
        if let Some(wait_for) = params.get("wait_for").and_then(|v| v.as_str()) {
            tab.wait_for_element(wait_for)
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
        }

        Ok(vec![Content::text(format!("Clicked element matching '{}'", selector))])
    }

    async fn type_text(&self, params: Value) -> AgentResult<Vec<Content>> {
        self.ensure_browser().await?;

        let selector = params
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("selector parameter is required".into()))?;

        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("text parameter is required".into()))?;

        let clear_first = params
            .get("clear_first")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let tab = self.tab.lock().await;
        let tab = tab.as_ref().unwrap();

        let element = tab.wait_for_element(selector)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        if clear_first {
            element.click()
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

            // Clear using keyboard shortcuts
            element.focus()
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
            
            // Select all and delete
            element.type_into("a")
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
            
            tab.press_key("Backspace")
                .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
        }

        element.type_into(text)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        Ok(vec![Content::text(format!(
            "Typed text into element matching '{}'",
            selector
        ))])
    }

    async fn eval(&self, params: Value) -> AgentResult<Vec<Content>> {
        self.ensure_browser().await?;

        let script = params
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("script parameter is required".into()))?;

        let tab = self.tab.lock().await;
        let tab = tab.as_ref().unwrap();

        let result = tab.evaluate(script, false)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
        
        Ok(vec![Content::text(format!("Evaluation result: {:?}", result))])
    }

    async fn wait_for(&self, params: Value) -> AgentResult<Vec<Content>> {
        self.ensure_browser().await?;

        let selector = params
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("selector parameter is required".into()))?;

        let timeout = params
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30000);

        let tab = self.tab.lock().await;
        let tab = tab.as_ref().unwrap();

        tab.wait_for_element_with_custom_timeout(selector, Duration::from_millis(timeout))
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        Ok(vec![Content::text(format!(
            "Successfully waited for element matching '{}'",
            selector
        ))])
    }
}

#[async_trait]
impl System for WebBrowserSystem {
    fn name(&self) -> &str {
        "WebBrowserSystem"
    }

    fn description(&self) -> &str {
        "Browser automation system using headless Chrome"
    }

    fn instructions(&self) -> &str {
        &self.instructions
    }

    fn tools(&self) -> &[Tool] {
        &self.tools
    }

    async fn status(&self) -> AnyhowResult<Vec<Resource>> {
        Ok(Vec::new())
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        match tool_call.name.as_str() {
            "navigate" => self.navigate(tool_call.arguments).await,
            "screenshot" => self.screenshot(tool_call.arguments).await,
            "click" => self.click(tool_call.arguments).await,
            "type" => self.type_text(tool_call.arguments).await,
            "eval" => self.eval(tool_call.arguments).await,
            "wait_for" => self.wait_for(tool_call.arguments).await,
            _ => Err(AgentError::ToolNotFound(tool_call.name)),
        }
    }

    async fn read_resource(&self, _uri: &str) -> AgentResult<String> {
        Err(AgentError::InvalidParameters("Resource reading not supported".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::OnceCell;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    static WEB_SYSTEM: OnceCell<WebBrowserSystem> = OnceCell::const_new();

    async fn get_system() -> &'static WebBrowserSystem {
        WEB_SYSTEM
            .get_or_init(|| async { WebBrowserSystem::new() })
            .await
    }

    #[tokio::test]
    async fn test_navigate_and_screenshot() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(r#"<html><body><h1>Test Page</h1></body></html>"#))
            .mount(&mock_server)
            .await;

        let system = get_system().await;
        
        // Navigate to mock server
        let navigate_result = system.call(ToolCall::new(
            "navigate",
            json!({
                "url": mock_server.uri(),
            })
        )).await.unwrap();

        assert!(navigate_result[0].as_text().unwrap().contains("Navigated to"));

        // Take a screenshot
        let screenshot_result = system.call(ToolCall::new(
            "screenshot",
            json!({
                "max_width": 768
            })
        )).await.unwrap();

        assert!(screenshot_result[0].as_image().is_some());
    }

    
}