use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::tool_router_index_manager::ToolRouterIndexManager;
use crate::config::get_extension_by_name;
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, ErrorCode, ErrorData, GetPromptResult, Implementation,
    InitializeResult, JsonObject, ListPromptsResult, ListResourcesResult, ListToolsResult,
    ProtocolVersion, ReadResourceResult, ServerCapabilities, ServerNotification, Tool,
    ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::error;

pub static EXTENSION_NAME: &str = "Extension Manager";
// pub static DISPLAY_NAME: &str = "Extension Manager";

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

    #[error("Failed to deserialize parameters: {0}")]
    DeserializationError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ManageExtensionAction {
    Enable,
    Disable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ManageExtensionsParams {
    pub action: ManageExtensionAction,
    pub extension_name: String,
}

pub const SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME: &str = "search_available_extensions";
pub const MANAGE_EXTENSIONS_TOOL_NAME: &str = "manage_extensions";
pub const MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE: &str = "extensionmanager__manage_extensions";

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
                name: EXTENSION_NAME.to_string(),
                title: Some(EXTENSION_NAME.to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(
                indoc! {r#"
                Extension Management

                Use these tools to discover, enable, and disable extensions.

                Available tools:
                - search_available_extensions: Find extensions available to enable/disable
                - manage_extensions: Enable or disable extensions

                Use search_available_extensions when you need to find what extensions are available.
                Use manage_extensions to enable or disable specific extensions by name.
            "#}
                .to_string(),
            ),
        };

        Ok(Self { info, context })
    }

    async fn handle_search_available_extensions(
        &self,
    ) -> Result<Vec<Content>, ExtensionManagerToolError> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                match extension_manager.search_available_extensions().await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(ExtensionManagerToolError::OperationFailed {
                        message: format!("Failed to search available extensions: {}", e.message),
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

        let params: ManageExtensionsParams =
            serde_json::from_value(serde_json::Value::Object(arguments))?;

        match self
            .manage_extensions_impl(params.action, params.extension_name)
            .await
        {
            Ok(content) => Ok(content),
            Err(error_data) => Err(ExtensionManagerToolError::OperationFailed {
                message: error_data.message.to_string(),
            }),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn manage_extensions_impl(
        &self,
        action: ManageExtensionAction,
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
                    let selector_action = if action == ManageExtensionAction::Disable {
                        "remove"
                    } else {
                        "add"
                    };
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

        if action == ManageExtensionAction::Disable {
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

        let config = match get_extension_by_name(&extension_name) {
            Some(config) => config,
            None => {
                return Err(ErrorData::new(
                    ErrorCode::RESOURCE_NOT_FOUND,
                    format!(
                        "Extension '{}' not found. Please check the extension name and try again.",
                        extension_name
                    ),
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
                        let llm_action = if action == ManageExtensionAction::Disable {
                            "remove"
                        } else {
                            "add"
                        };
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

    #[allow(clippy::too_many_lines)]
    async fn get_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME.to_string(),
                "Searches for additional extensions available to help complete tasks.
        Use this tool when you're unable to find a specific feature or functionality you need to complete your task, or when standard approaches aren't working.
        These extensions might provide the exact tools needed to solve your problem.
        If you find a relevant one, consider using your tools to enable it.".to_string(),
                Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "required": [],
                        "properties": {}
                    })
                    .as_object()
                    .expect("Schema must be an object")
                    .clone()
                ),
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
                Arc::new(
                    serde_json::to_value(schema_for!(ManageExtensionsParams))
                        .expect("Failed to serialize schema")
                        .as_object()
                        .expect("Schema must be an object")
                        .clone()
                ),
            ).annotate(ToolAnnotations {
                title: Some("Enable or disable an extension".to_string()),
                read_only_hint: Some(false),
                destructive_hint: Some(false),
                idempotent_hint: Some(false),
                open_world_hint: Some(false),
            }),
        ]
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
            }
            MANAGE_EXTENSIONS_TOOL_NAME => self.handle_manage_extensions(arguments).await,
            _ => Err(ExtensionManagerToolError::UnknownTool {
                tool_name: name.to_string(),
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
                    is_error: Some(true), // ✅ Properly mark as error
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
