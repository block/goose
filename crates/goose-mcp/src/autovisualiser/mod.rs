use base64::{engine::general_purpose::STANDARD, Engine as _};
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
        let render_sankey_tool = Tool::new(
            "render_sankey",
            indoc! {r#"
                Renders a Sankey diagram visualization from flow data.
                Returns an interactive HTML visualization using D3.js.
                
                The data should contain:
                - nodes: Array of objects with 'name' and optional 'category' properties
                - links: Array of objects with 'source', 'target', and 'value' properties
                
                Example:
                {
                  "nodes": [
                    {"name": "Source A", "category": "source"},
                    {"name": "Target B", "category": "target"}
                  ],
                  "links": [
                    {"source": "Source A", "target": "Target B", "value": 100}
                  ]
                }
            "#},
            object!({
                "type": "object",
                "required": ["data"],
                "properties": {
                    "data": {
                        "type": "object",
                        "required": ["nodes", "links"],
                        "properties": {
                            "nodes": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["name"],
                                    "properties": {
                                        "name": {"type": "string"},
                                        "category": {"type": "string"}
                                    }
                                }
                            },
                            "links": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["source", "target", "value"],
                                    "properties": {
                                        "source": {"type": "string"},
                                        "target": {"type": "string"},
                                        "value": {"type": "number"}
                                    }
                                }
                            }
                        }
                    }
                }
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
            - **render_sankey**: Creates interactive Sankey diagrams from flow data

            ## Purpose:
            This extension is designed to help generate dynamic visualizations and UI
            components that can be displayed in MCP-compatible interfaces.

            ## Cache Directory:
            Temporary files are stored in: {}
        "#, cache_dir.display()};

        Self {
            tools: vec![render_sankey_tool],
            cache_dir,
            active_resources: Arc::new(Mutex::new(HashMap::new())),
            instructions,
        }
    }


    async fn render_sankey(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        // Extract the data from parameters
        let data = params
            .get("data")
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'data' parameter".to_string()))?;

        // Load all resources at compile time using include_str!
        const TEMPLATE: &str = include_str!("sankey_template.html");
        const D3_MIN: &str = include_str!("d3.min.js");
        const D3_SANKEY: &str = include_str!("d3.sankey.min.js");

        // Convert the data to JSON string
        let data_json = serde_json::to_string(&data)
            .map_err(|e| ToolError::InvalidParameters(format!("Invalid JSON data: {}", e)))?;

        // Replace all placeholders with actual content
        let html_content = TEMPLATE
            .replace("{{D3_MIN}}", D3_MIN)
            .replace("{{D3_SANKY}}", D3_SANKEY) // Note: keeping the typo to match template
            .replace("{{SANKEY_DATA}}", &data_json);

        // Save to /tmp/vis.html for debugging
        let debug_path = std::path::Path::new("/tmp/vis.html");
        if let Err(e) = std::fs::write(debug_path, &html_content) {
            tracing::warn!("Failed to write debug HTML to /tmp/vis.html: {}", e);
        } else {
            tracing::info!("Debug HTML saved to /tmp/vis.html");
        }

        // Use BlobResourceContents with base64 encoding to avoid JSON string escaping issues
        let html_bytes = html_content.as_bytes();
        let base64_encoded = STANDARD.encode(html_bytes);

        let resource_contents = ResourceContents::BlobResourceContents {
            uri: "ui://sankey/diagram".to_string(),
            mime_type: Some("text/html".to_string()),
            blob: base64_encoded,
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
                "render_sankey" => this.render_sankey(arguments).await,
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
