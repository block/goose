use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use async_trait::async_trait;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListResourcesResult,
    ListToolsResult, Meta, ProtocolVersion, ReadResourceResult, ServerCapabilities,
    Tool as McpTool, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "browser";

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserOpenParams {
    /// URL to open in the browser
    url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserNavigateParams {
    /// URL to navigate to
    url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserClickParams {
    /// CSS selector for the element to click
    selector: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserTypeParams {
    /// CSS selector for the input element
    selector: String,
    /// Text to type into the element
    text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserGetTextParams {
    /// CSS selector (defaults to "body" if not provided)
    selector: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserGetHtmlParams {
    /// CSS selector (defaults to "body" if not provided)
    selector: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserEvaluateParams {
    /// JavaScript code to execute in the page context
    script: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserWaitParams {
    /// CSS selector to wait for
    selector: String,
    /// Timeout in milliseconds (default: 5000)
    timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserScrollParams {
    /// Direction to scroll: "up", "down", "top", or "bottom"
    direction: String,
    /// Number of pixels to scroll (optional, defaults to one viewport)
    amount: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserScreenshotParams {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserCloseParams {}

pub struct BrowserClient {
    #[allow(dead_code)]
    context: PlatformExtensionContext,
    info: InitializeResult,
}

impl BrowserClient {
    pub fn new(context: PlatformExtensionContext) -> Self {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                tasks: None,
                resources: None,
                extensions: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                description: None,
                title: Some("Browser".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Control an embedded browser window for web navigation and content extraction."
                    .to_string(),
            ),
        };
        Self { context, info }
    }

    fn schema<T: JsonSchema>() -> JsonObject {
        let schema = schema_for!(T);
        let schema_value =
            serde_json::to_value(schema).expect("Failed to serialize schema");
        schema_value.as_object().unwrap().clone()
    }

    fn forward_to_client(command: &str, arguments: Option<JsonObject>) -> CallToolResult {
        let params = arguments
            .map(serde_json::Value::Object)
            .unwrap_or(json!({}));

        let content = json!({
            "command": command,
            "params": params,
        });

        let mut meta_map = serde_json::Map::new();
        meta_map.insert("forward_to_client".to_string(), json!(true));

        CallToolResult {
            content: vec![Content::text(content.to_string())],
            structured_content: None,
            is_error: Some(false),
            meta: Some(Meta(meta_map)),
        }
    }
}

#[async_trait]
impl McpClientTrait for BrowserClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let tools = vec![
            McpTool::new(
                "browser_open".to_string(),
                "Open a URL in the embedded browser. This will display a browser panel in the UI."
                    .to_string(),
                Self::schema::<BrowserOpenParams>(),
            ),
            McpTool::new(
                "browser_close".to_string(),
                "Close the embedded browser panel.".to_string(),
                Self::schema::<BrowserCloseParams>(),
            ),
            McpTool::new(
                "browser_navigate".to_string(),
                "Navigate to a different URL in the browser.".to_string(),
                Self::schema::<BrowserNavigateParams>(),
            ),
            McpTool::new(
                "browser_screenshot".to_string(),
                "Take a screenshot of the current browser view.".to_string(),
                Self::schema::<BrowserScreenshotParams>(),
            ),
            McpTool::new(
                "browser_click".to_string(),
                "Click an element in the page by CSS selector.".to_string(),
                Self::schema::<BrowserClickParams>(),
            ),
            McpTool::new(
                "browser_type".to_string(),
                "Type text into an input element.".to_string(),
                Self::schema::<BrowserTypeParams>(),
            ),
            McpTool::new(
                "browser_get_text".to_string(),
                "Get the text content of an element (default: body).".to_string(),
                Self::schema::<BrowserGetTextParams>(),
            ),
            McpTool::new(
                "browser_get_html".to_string(),
                "Get the HTML content of an element (default: body).".to_string(),
                Self::schema::<BrowserGetHtmlParams>(),
            ),
            McpTool::new(
                "browser_evaluate".to_string(),
                "Execute JavaScript in the page context and return the result.".to_string(),
                Self::schema::<BrowserEvaluateParams>(),
            ),
            McpTool::new(
                "browser_wait".to_string(),
                "Wait for an element to appear in the page.".to_string(),
                Self::schema::<BrowserWaitParams>(),
            ),
            McpTool::new(
                "browser_scroll".to_string(),
                "Scroll the page. Direction can be 'up', 'down', 'top', or 'bottom'.".to_string(),
                Self::schema::<BrowserScrollParams>(),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        _session_id: &str,
        name: &str,
        arguments: Option<JsonObject>,
        _working_dir: Option<&str>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let command = name.strip_prefix("browser_").unwrap_or(name);
        Ok(Self::forward_to_client(command, arguments))
    }

    async fn list_resources(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        _session_id: &str,
        _uri: &str,
        _cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
