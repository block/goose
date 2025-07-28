//! MockClient is a mock implementation of the McpClientTrait for testing purposes.
//! add a tool you want to have around and then add the client to the extension router

use mcp_client::client::{ClientCapabilities, ClientInfo, Error, McpClientTrait};
use mcp_core::protocol::{
    CallToolResult, Implementation, InitializeResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, ReadResourceResult, ServerCapabilities, ToolsCapability,
};
use mcp_core::{Tool, ToolError};
use rmcp::model::{Content, GetPromptResult, ServerNotification};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, Receiver};

pub struct MockClient {
    tools: HashMap<String, Tool>,
    handlers: HashMap<String, Box<dyn Fn(&Value) -> Result<Vec<Content>, ToolError> + Send + Sync>>,
}

impl MockClient {
    pub(crate) fn new() -> Self {
        Self {
            tools: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    pub(crate) fn add_tool<F>(mut self, tool: Tool, handler: F) -> Self
    where
        F: Fn(&Value) -> Result<Vec<Content>, ToolError> + Send + Sync + 'static,
    {
        let tool_name = tool.name.to_string();
        self.tools.insert(tool_name.clone(), tool);
        self.handlers.insert(tool_name, Box::new(handler));
        self
    }
}

#[async_trait::async_trait]
impl McpClientTrait for MockClient {
    async fn initialize(
        &mut self,
        _: ClientInfo,
        _: ClientCapabilities,
    ) -> Result<InitializeResult, Error> {
        Ok(InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                prompts: None,
                resources: None,
                tools: Some(ToolsCapability { list_changed: None }),
            },
            server_info: Implementation {
                name: "MockClient".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: None,
        })
    }

    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
    ) -> Result<ListResourcesResult, Error> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(&self, _uri: &str) -> Result<ReadResourceResult, Error> {
        Err(Error::UnexpectedResponse(
            "Resources not supported by mock client".to_string(),
        ))
    }

    async fn list_tools(&self, _: Option<String>) -> Result<ListToolsResult, Error> {
        let rmcp_tools: Vec<rmcp::model::Tool> = self
            .tools
            .values()
            .map(|tool| {
                let input_schema = if let serde_json::Value::Object(obj) = &tool.input_schema {
                    std::sync::Arc::new(obj.clone())
                } else {
                    std::sync::Arc::new(serde_json::Map::new())
                };

                rmcp::model::Tool::new(
                    tool.name.to_string(),
                    tool.description.to_string(),
                    input_schema,
                )
            })
            .collect();

        Ok(ListToolsResult {
            tools: rmcp_tools,
            next_cursor: None,
        })
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<CallToolResult, Error> {
        if let Some(handler) = self.handlers.get(name) {
            match handler(&arguments) {
                Ok(content) => Ok(CallToolResult {
                    content,
                    is_error: None,
                }),
                Err(e) => Err(Error::UnexpectedResponse(e.to_string())),
            }
        } else {
            Err(Error::UnexpectedResponse(format!(
                "Tool '{}' not found",
                name
            )))
        }
    }

    async fn list_prompts(&self, _next_cursor: Option<String>) -> Result<ListPromptsResult, Error> {
        Ok(ListPromptsResult { prompts: vec![] })
    }

    async fn get_prompt(&self, _name: &str, _arguments: Value) -> Result<GetPromptResult, Error> {
        Err(Error::UnexpectedResponse(
            "Prompts not supported by mock client".to_string(),
        ))
    }

    async fn subscribe(&self) -> Receiver<ServerNotification> {
        mpsc::channel(1).1
    }
}

pub const WEATHER_TYPE: &str = "cloudy";

pub fn weather_client() -> MockClient {
    let weather_tool = Tool::new(
        "get_weather",
        "Get the weather for a location",
        serde_json::json!({
            "type": "object",
            "required": ["location"],
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                }
            }
        }),
        None, // ToolAnnotations
    );

    let mock_client = MockClient::new().add_tool(weather_tool, |args| {
        let location = args
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown location");

        Ok(vec![Content::text(format!(
            "The weather in {} is {} and 18°C",
            location, WEATHER_TYPE
        ))])
    });
    mock_client
}
