use anyhow::Result;
use async_trait::async_trait;
use mcp_client::client::{Error, McpClientTrait};
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, ListPromptsResult,
    ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult, ServerCapabilities,
    ServerNotification, Tool, ToolsCapability,
};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::service::{goose_app_from_json, GooseAppUpdates, GooseAppsService};

pub struct GooseAppsClient {
    service: GooseAppsService,
    info: InitializeResult,
}

impl GooseAppsClient {
    pub fn new() -> Result<Self> {
        let service = GooseAppsService::new()?;

        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: "goose-apps".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Manage Goose Apps - create, update, list JavaScript apps that extend Goose functionality.".to_string()),
        };

        Ok(Self { service, info })
    }

    async fn handle_create_app(&self, arguments: Value) -> Result<Vec<Content>, String> {
        let app = goose_app_from_json(&arguments).map_err(|e| e.to_string())?;
        let result = self
            .service
            .create_app(&app)
            .await
            .map_err(|e| e.to_string())?;
        Ok(vec![Content::text(result)])
    }

    async fn handle_update_app(&self, arguments: Value) -> Result<Vec<Content>, String> {
        let name = arguments
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: name")?;

        let updates = GooseAppUpdates::from_json(&arguments);
        let result = self
            .service
            .update_app(name, &updates)
            .await
            .map_err(|e| e.to_string())?;
        Ok(vec![Content::text(result)])
    }

    async fn handle_list_apps(&self) -> Result<Vec<Content>, String> {
        let apps = self.service.list_apps().await.map_err(|e| e.to_string())?;
        let formatted = GooseAppsService::format_app_list(&apps);
        Ok(vec![Content::text(formatted)])
    }

    async fn handle_get_app(&self, arguments: Value) -> Result<Vec<Content>, String> {
        let name = arguments
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: name")?;

        let app = self
            .service
            .get_app(name)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("App '{}' not found", name))?;

        let formatted = GooseAppsService::format_app_details(&app);
        Ok(vec![Content::text(formatted)])
    }

    fn get_tools() -> Vec<Tool> {
        fn create_schema(json_value: Value) -> Arc<Map<String, Value>> {
            Arc::new(json_value.as_object().unwrap().clone())
        }

        vec![
            Tool {
                name: "create_goose_app".into(),
                description: Some("Create a new Goose App with JavaScript implementation".into()),
                input_schema: create_schema(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the Goose App"
                        },
                        "js_implementation": {
                            "type": "string",
                            "description": "JavaScript implementation containing a class extending GooseWidget"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description of the app"
                        },
                        "width": {
                            "type": "integer",
                            "description": "Optional window width in pixels"
                        },
                        "height": {
                            "type": "integer",
                            "description": "Optional window height in pixels"
                        },
                        "resizable": {
                            "type": "boolean",
                            "description": "Whether the window should be resizable"
                        }
                    },
                    "required": ["name", "js_implementation"]
                })),
                annotations: None,
                output_schema: None,
            },
            Tool {
                name: "update_goose_app".into(),
                description: Some("Update an existing Goose App".into()),
                input_schema: create_schema(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the Goose App to update"
                        },
                        "js_implementation": {
                            "type": "string",
                            "description": "New JavaScript implementation containing a class extending GooseWidget"
                        },
                        "description": {
                            "type": "string",
                            "description": "Updated description of the app"
                        },
                        "width": {
                            "type": "integer",
                            "description": "Updated window width in pixels"
                        },
                        "height": {
                            "type": "integer",
                            "description": "Updated window height in pixels"
                        },
                        "resizable": {
                            "type": "boolean",
                            "description": "Whether the window should be resizable"
                        }
                    },
                    "required": ["name"]
                })),
                annotations: None,
                output_schema: None,
            },
            Tool {
                name: "list_goose_apps".into(),
                description: Some("List all available Goose Apps".into()),
                input_schema: create_schema(json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                })),
                annotations: None,
                output_schema: None,
            },
            Tool {
                name: "get_goose_app".into(),
                description: Some("Get detailed information about a specific Goose App including its implementation".into()),
                input_schema: create_schema(json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the Goose App"
                        }
                    },
                    "required": ["name"]
                })),
                annotations: None,
                output_schema: None,
            }
        ]
    }
}

#[async_trait]
impl McpClientTrait for GooseAppsClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancellation_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "create_goose_app" => self.handle_create_app(arguments).await,
            "update_goose_app" => self.handle_update_app(arguments).await,
            "list_goose_apps" => self.handle_list_apps().await,
            "get_goose_app" => self.handle_get_app(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match content {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Value,
        _cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}
