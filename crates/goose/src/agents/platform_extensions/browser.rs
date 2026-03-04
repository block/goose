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
struct BrowserNavigateParams {
    /// URL to navigate to. Opens the browser panel if not already open.
    url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserClickParams {
    /// Element to click — use [index] from inspect results or a CSS selector
    target: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserTypeParams {
    /// Element to type into — use [index] from inspect results or a CSS selector
    target: String,
    /// Text to type into the element
    text: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserInspectParams {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserGetParams {
    /// CSS selector (defaults to "body" if not provided)
    selector: Option<String>,
    /// Output format: "text", "html", or "markdown" (default: "text")
    format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BrowserEvaluateParams {
    /// JavaScript code to execute in the page context
    script: String,
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
                "navigate".to_string(),
                "Navigate to a URL. Opens the browser panel if not already open. Returns a page summary with interactive elements.".to_string(),
                Self::schema::<BrowserNavigateParams>(),
            ),
            McpTool::new(
                "inspect".to_string(),
                "Get a summary of the current page including interactive elements (links, buttons, inputs) with indexed references. Use after interactions that change the page.".to_string(),
                Self::schema::<BrowserInspectParams>(),
            ),
            McpTool::new(
                "click".to_string(),
                "Click an element. Use [index] from inspect results (e.g., '[3]') or a CSS selector.".to_string(),
                Self::schema::<BrowserClickParams>(),
            ),
            McpTool::new(
                "type".to_string(),
                "Type text into an input element. Use [index] from inspect results (e.g., '[3]') or a CSS selector.".to_string(),
                Self::schema::<BrowserTypeParams>(),
            ),
            McpTool::new(
                "get".to_string(),
                "Get page content. Format: 'text' (default), 'html', or 'markdown'. Selector defaults to 'body'.".to_string(),
                Self::schema::<BrowserGetParams>(),
            ),
            McpTool::new(
                "screenshot".to_string(),
                "Take a screenshot of the current browser view.".to_string(),
                Self::schema::<BrowserScreenshotParams>(),
            ),
            McpTool::new(
                "scroll".to_string(),
                "Scroll the page. Direction: 'up', 'down', 'top', or 'bottom'.".to_string(),
                Self::schema::<BrowserScrollParams>(),
            ),
            McpTool::new(
                "evaluate".to_string(),
                "Execute JavaScript in the page context. Use as a last resort for complex interactions.".to_string(),
                Self::schema::<BrowserEvaluateParams>(),
            ),
            McpTool::new(
                "close".to_string(),
                "Close the embedded browser panel.".to_string(),
                Self::schema::<BrowserCloseParams>(),
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
        let params = arguments
            .map(serde_json::Value::Object)
            .unwrap_or(json!({}));
        let content = json!({ "command": name, "params": params });
        let mut meta_map = serde_json::Map::new();
        meta_map.insert("forward_to_client".to_string(), json!(true));
        Ok(CallToolResult {
            content: vec![Content::text(content.to_string())],
            structured_content: None,
            is_error: Some(false),
            meta: Some(Meta(meta_map)),
        })
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
