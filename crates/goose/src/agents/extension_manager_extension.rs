use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager::normalize;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
use crate::config::ExtensionConfigManager;
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, ErrorCode, ErrorData, GetPromptResult, Implementation,
    InitializeResult, JsonObject, ListPromptsResult, ListResourcesResult, ListToolsResult,
    ProtocolVersion, ReadResourceResult, ServerCapabilities, ServerNotification, Tool,
    ToolAnnotations, ToolsCapability,
};
use rmcp::object;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::error;

pub static EXTENSION_NAME: &str = "extension manager";

#[derive(Debug, thiserror::Error)]
pub enum ExtensionManagerToolError {
    #[error("Unknown tool: {tool_name}")]
    UnknownTool { tool_name: String },
    
    #[error("Extension manager not available")]
    ManagerUnavailable,
    
    #[error("Missing required parameter: {param_name}")]
    MissingParameter { param_name: String },
    
    #[error("Invalid action: {action}. Must be 'enable' or 'disable'")]
    InvalidAction { action: String },
    
    #[error("Extension operation failed: {message}")]
    OperationFailed { message: String },
}

// Tool name constants
pub const READ_RESOURCE_TOOL_NAME: &str = "read_resource";
pub const LIST_RESOURCES_TOOL_NAME: &str = "list_resources";
pub const SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME: &str = "search_available_extensions";
pub const MANAGE_EXTENSIONS_TOOL_NAME: &str = "manage_extensions";

pub struct ExtensionManagerClient {
    info: InitializeResult,
    #[allow(dead_code)]
    context: PlatformExtensionContext,
}

impl ExtensionManagerClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
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
                name: normalize(EXTENSION_NAME.to_string()),
                title: Some("Extension Manager".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {r#"
                Extension Management

                Use these tools to discover, enable, and disable extensions, as well as review resources.

                Available tools:
                - search_available_extensions: Find extensions available to enable/disable
                - manage_extensions: Enable or disable extensions
                - list_resources: List resources from extensions
                - read_resource: Read specific resources from extensions

                Use search_available_extensions when you need to find what extensions are available.
                Use manage_extensions to enable or disable specific extensions by name.
                Use list_resources and read_resource to work with extension data and resources.
            "#}.to_string()),
        };

        Ok(Self { info, context })
    }

    async fn handle_search_available_extensions(&self) -> Result<Vec<Content>, ExtensionManagerToolError> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                match extension_manager.search_available_extensions().await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(ExtensionManagerToolError::OperationFailed {
                        message: format!("Failed to search available extensions: {}", e.message)
                    }),
                }
            } else {
                Err(ExtensionManagerToolError::ManagerUnavailable)
            }
        } else {
            Err(ExtensionManagerToolError::ManagerUnavailable)
        }
    }

    async fn handle_manage_extensions(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, ExtensionManagerToolError> {
        let arguments = arguments.ok_or(ExtensionManagerToolError::MissingParameter {
            param_name: "arguments".to_string(),
        })?;

        let action = arguments
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or(ExtensionManagerToolError::MissingParameter {
                param_name: "action".to_string(),
            })?;

        let extension_name = arguments
            .get("extension_name")
            .and_then(|v| v.as_str())
            .ok_or(ExtensionManagerToolError::MissingParameter {
                param_name: "extension_name".to_string(),
            })?;

        if !matches!(action, "enable" | "disable") {
            return Err(ExtensionManagerToolError::InvalidAction {
                action: action.to_string(),
            });
        }

        match self
            .manage_extensions_impl(action.to_string(), extension_name.to_string())
            .await
        {
            Ok(content) => Ok(content),
            Err(error_data) => Err(ExtensionManagerToolError::OperationFailed {
                message: error_data.message.to_string(),
            }),
        }
    }

    async fn manage_extensions_impl(
        &self,
        action: String,
        extension_name: String,
    ) -> Result<Vec<Content>, ErrorData> {
        let extension_manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Extension manager is no longer available".to_string(),
                    None,
                )
            })?;

        let tool_route_manager = self
            .context
            .tool_route_manager
            .as_ref()
            .and_then(|weak| weak.upgrade());

        // Update tool router index if router is functional
        if let Some(tool_route_manager) = &tool_route_manager {
            if tool_route_manager.is_router_functional().await {
                let selector = tool_route_manager.get_router_tool_selector().await;
                if let Some(selector) = selector {
                    let selector_action = if action == "disable" { "remove" } else { "add" };
                    let selector = Arc::new(selector);
                    if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                        &selector,
                        &extension_manager,
                        &extension_name,
                        selector_action,
                    )
                    .await
                    {
                        return Err(ErrorData::new(
                            ErrorCode::INTERNAL_ERROR,
                            format!("Failed to update LLM index: {}", e),
                            None,
                        ));
                    }
                }
            }
        }

        if action == "disable" {
            let result = extension_manager
                .remove_extension(&extension_name)
                .await
                .map(|_| {
                    vec![Content::text(format!(
                        "The extension '{}' has been disabled successfully",
                        extension_name
                    ))]
                })
                .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None));
            return result;
        }

        let config = match ExtensionConfigManager::get_config_by_name(&extension_name) {
            Ok(Some(config)) => config,
            Ok(None) => {
                return Err(ErrorData::new(
                    ErrorCode::RESOURCE_NOT_FOUND,
                    format!(
                        "Extension '{}' not found. Please check the extension name and try again.",
                        extension_name
                    ),
                    None,
                ));
            }
            Err(e) => {
                return Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to get extension config: {}", e),
                    None,
                ));
            }
        };

        let result = extension_manager
            .add_extension(config)
            .await
            .map(|_| {
                vec![Content::text(format!(
                    "The extension '{}' has been installed successfully",
                    extension_name
                ))]
            })
            .map_err(|e| ErrorData::new(ErrorCode::INTERNAL_ERROR, e.to_string(), None));

        // Update LLM index if operation was successful and LLM routing is functional
        if result.is_ok() {
            if let Some(tool_route_manager) = &tool_route_manager {
                if tool_route_manager.is_router_functional().await {
                    let selector = tool_route_manager.get_router_tool_selector().await;
                    if let Some(selector) = selector {
                        let llm_action = if action == "disable" { "remove" } else { "add" };
                        let selector = Arc::new(selector);
                        if let Err(e) = ToolRouterIndexManager::update_extension_tools(
                            &selector,
                            &extension_manager,
                            &extension_name,
                            llm_action,
                        )
                        .await
                        {
                            return Err(ErrorData::new(
                                ErrorCode::INTERNAL_ERROR,
                                format!("Failed to update LLM index: {}", e),
                                None,
                            ));
                        }
                    }
                }
            }
        }

        result
    }

    async fn handle_list_resources(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, ExtensionManagerToolError> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                let params = arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                match extension_manager
                    .list_resources(params, tokio_util::sync::CancellationToken::default())
                    .await
                {
                    Ok(content) => Ok(content),
                    Err(e) => Err(ExtensionManagerToolError::OperationFailed {
                        message: format!("Failed to list resources: {}", e.message)
                    }),
                }
            } else {
                Err(ExtensionManagerToolError::ManagerUnavailable)
            }
        } else {
            Err(ExtensionManagerToolError::ManagerUnavailable)
        }
    }

    async fn handle_read_resource(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, ExtensionManagerToolError> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                let params = arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                match extension_manager
                    .read_resource(params, tokio_util::sync::CancellationToken::default())
                    .await
                {
                    Ok(content) => Ok(content),
                    Err(e) => Err(ExtensionManagerToolError::OperationFailed {
                        message: format!("Failed to read resource: {}", e.message)
                    }),
                }
            } else {
                Err(ExtensionManagerToolError::ManagerUnavailable)
            }
        } else {
            Err(ExtensionManagerToolError::ManagerUnavailable)
        }
    }

    async fn get_tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            Tool::new(
                SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME.to_string(),
                "Searches for additional extensions available to help complete tasks.
        Use this tool when you're unable to find a specific feature or functionality you need to complete your task, or when standard approaches aren't working.
        These extensions might provide the exact tools needed to solve your problem.
        If you find a relevant one, consider using your tools to enable it.".to_string(),
                object!({
                    "type": "object",
                    "required": [],
                    "properties": {}
                }),
            ).annotate(ToolAnnotations {
                title: Some("Discover extensions".to_string()),
                read_only_hint: Some(true),
                destructive_hint: Some(false),
                idempotent_hint: Some(false),
                open_world_hint: Some(false),
            }),
            Tool::new(
                MANAGE_EXTENSIONS_TOOL_NAME.to_string(),
                "Tool to manage extensions and tools in goose context.
            Enable or disable extensions to help complete tasks.
            Enable or disable an extension by providing the extension name.
            ".to_string(),
                object!({
                    "type": "object",
                    "required": ["action", "extension_name"],
                    "properties": {
                        "action": {"type": "string", "description": "The action to perform", "enum": ["enable", "disable"]},
                        "extension_name": {"type": "string", "description": "The name of the extension to enable"}
                    }
                }),
            ).annotate(ToolAnnotations {
                title: Some("Enable or disable an extension".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(false),
                idempotent_hint: Some(false),
                open_world_hint: Some(false),
            }),
        ];

        // Only add resource tools if extension manager supports resources
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                if extension_manager.supports_resources().await {
                    tools.extend([
                        Tool::new(
                            LIST_RESOURCES_TOOL_NAME.to_string(),
                            indoc! {r#"
            List resources from an extension(s).

            Resources allow extensions to share data that provide context to LLMs, such as
            files, database schemas, or application-specific information. This tool lists resources
            in the provided extension, and returns a list for the user to browse. If no extension
            is provided, the tool will search all extensions for the resource.
        "#}.to_string(),
                            object!({
                                "type": "object",
                                "properties": {
                                    "extension_name": {"type": "string", "description": "Optional extension name"}
                                }
                            }),
                        ).annotate(ToolAnnotations {
                            title: Some("List resources".to_string()),
                            read_only_hint: Some(true),
                            destructive_hint: Some(false),
                            idempotent_hint: Some(false),
                            open_world_hint: Some(false),
                        }),
                        Tool::new(
                            READ_RESOURCE_TOOL_NAME.to_string(),
                            indoc! {r#"
            Read a resource from an extension.

            Resources allow extensions to share data that provide context to LLMs, such as
            files, database schemas, or application-specific information. This tool searches for the
            resource URI in the provided extension, and reads in the resource content. If no extension
            is provided, the tool will search all extensions for the resource.
        "#}.to_string(),
                            object!({
                                "type": "object",
                                "required": ["uri"],
                                "properties": {
                                    "uri": {"type": "string", "description": "Resource URI"},
                                    "extension_name": {"type": "string", "description": "Optional extension name"}
                                }
                            }),
                        ).annotate(ToolAnnotations {
                            title: Some("Read a resource".to_string()),
                            read_only_hint: Some(true),
                            destructive_hint: Some(false),
                            idempotent_hint: Some(false),
                            open_world_hint: Some(false),
                        }),
                    ]);
                }
            }
        }

        tools
    }
}

#[async_trait]
impl McpClientTrait for ExtensionManagerClient {
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
        // Extension manager doesn't expose resources directly
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: self.get_tools().await,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let result = match name {
            SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME => {
                self.handle_search_available_extensions().await
                    .map_err(|e| ExtensionManagerToolError::OperationFailed { 
                        message: e.to_string() 
                    })
            }
            MANAGE_EXTENSIONS_TOOL_NAME => {
                self.handle_manage_extensions(arguments).await
                    .map_err(|e| ExtensionManagerToolError::OperationFailed { 
                        message: e.to_string() 
                    })
            }
            LIST_RESOURCES_TOOL_NAME => {
                self.handle_list_resources(arguments).await.map_err(|e| ExtensionManagerToolError::OperationFailed { 
                    message: e.to_string() 
                })
            }
            READ_RESOURCE_TOOL_NAME => {
                self.handle_read_resource(arguments).await.map_err(|e| ExtensionManagerToolError::OperationFailed { 
                    message: e.to_string() 
                })
            }
            _ => Err(ExtensionManagerToolError::UnknownTool { 
                tool_name: name.to_string() 
            }),
        };

        match result {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => {
                // Log the error for debugging
                error!("Extension manager tool '{}' failed: {}", name, error);
                
                // Return proper error result with is_error flag set
                Ok(CallToolResult {
                    content: vec![Content::text(error.to_string())],
                    is_error: Some(true),  // ✅ Properly mark as error
                    structured_content: None,
                    meta: None,
                })
            }
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
