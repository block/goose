use etcetera::{choose_app_strategy, AppStrategy};
use indoc::{formatdoc, indoc};
use serde_json::Value;
use std::{collections::HashMap, future::Future, path::PathBuf, pin::Pin, sync::Arc, sync::Mutex};
use tokio::sync::mpsc;

use mcp_core::{
    handler::{PromptError, ResourceError, ToolError},
    protocol::ServerCapabilities,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use rmcp::model::{Content, JsonRpcMessage, Prompt, Resource, ResourceContents, Tool};
use rmcp::object;

/// An extension for automatic data visualization and UI generation
#[derive(Clone)]
pub struct AutoVisualiserRouter {
    tools: Vec<Tool>,
    #[allow(dead_code)]
    cache_dir: PathBuf,
    active_resources: Arc<Mutex<HashMap<String, Resource>>>,
    instructions: String,
}

impl Default for AutoVisualiserRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoVisualiserRouter {
    pub fn new() -> Self {
        let mcp_ui_hello_tool = Tool::new(
            "mcp_ui_hello",
            indoc! {r#"
                Returns a simple HTML hello message using the MCP UI resource format.
                This is a demonstration tool that shows how to return HTML content through the MCP UI resource protocol.
            "#},
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        // choose_app_strategy().cache_dir()
        // - macOS/Linux: ~/.cache/goose/autovisualiser/
        // - Windows:     ~\AppData\Local\Block\goose\cache\autovisualiser\
        let cache_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
            .unwrap()
            .cache_dir()
            .join("autovisualiser");

        // Create cache directory if it doesn't exist
        let _ = std::fs::create_dir_all(&cache_dir);

        let instructions = formatdoc! {r#"
            The AutoVisualiser extension provides tools for automatic data visualization
            and UI generation using MCP UI resources.

            ## Available Tools:
            - **mcp_ui_hello**: A simple demonstration tool that returns HTML content

            ## Purpose:
            This extension is designed to help generate dynamic visualizations and UI
            components that can be displayed in MCP-compatible interfaces.

            ## Cache Directory:
            Temporary files are stored in: {}
        "#, cache_dir.display()};

        Self {
            tools: vec![mcp_ui_hello_tool],
            cache_dir,
            active_resources: Arc::new(Mutex::new(HashMap::new())),
            instructions,
        }
    }

    async fn mcp_ui_hello(&self, _params: Value) -> Result<Vec<Content>, ToolError> {
        // Create an MCP UI resource with HTML content
        let html_content = "<html><body><h1>Hello from MCP UI!</h1></body></html>";

        // Create a proper ResourceContents::TextResourceContents
        let resource_contents = ResourceContents::TextResourceContents {
            uri: "ui://hello/greeting".to_string(),
            mime_type: Some("text/html".to_string()),
            text: html_content.to_string(),
        };

        Ok(vec![Content::resource(resource_contents)])
    }
}

impl Router for AutoVisualiserRouter {
    fn name(&self) -> String {
        "AutoVisualiserExtension".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
        _notifier: mpsc::Sender<JsonRpcMessage>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "mcp_ui_hello" => this.mcp_ui_hello(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        let active_resources = self.active_resources.lock().unwrap();
        let resources = active_resources.values().cloned().collect();
        tracing::info!("Listing resources: {:?}", resources);
        resources
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            Err(ResourceError::NotFound(format!(
                "Resource not found: {}",
                uri
            )))
        })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
}
