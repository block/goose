use crate::agents::extension::PlatformExtensionContext;
use crate::agents::extension_manager_extension::{
    MANAGE_EXTENSIONS_TOOL_NAME, SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
};
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    ServerCapabilities, ServerNotification, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::error;

pub static EXTENSION_NAME: &str = "Core";

#[derive(Debug, thiserror::Error)]
pub enum CoreToolError {
    #[error("Unknown tool: {tool_name}")]
    UnknownTool { tool_name: String },

    #[error("Extension manager not available")]
    ManagerUnavailable,

    #[error("Tool route manager not available")]
    ToolRouteManagerUnavailable,

    #[error("Operation failed: {message}")]
    OperationFailed { message: String },

    #[error("Failed to deserialize parameters: {0}")]
    DeserializationError(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadResourceParams {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListResourcesParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmSearchParams {
    pub extension_name: String,
    pub query: String,
    #[serde(default = "default_k")]
    pub k: usize,
}

fn default_k() -> usize {
    5
}

pub const READ_RESOURCE_TOOL_NAME: &str = "read_resource";
pub const LIST_RESOURCES_TOOL_NAME: &str = "list_resources";
pub const SEARCH_TOOLS_TOOL_NAME: &str = "search_tools";

pub fn llm_search_tool_prompt() -> String {
    format!(
        r#"# LLM Tool Selection Instructions
    Important: the user has opted to dynamically enable tools, so although an extension could be enabled, \
    please invoke the llm search tool to actually retrieve the most relevant tools to use according to the user's messages.
    For example, if the user has 3 extensions enabled, but they are asking for a tool to read a pdf file, \
    you would invoke the llm_search tool to find the most relevant read pdf tool.
    By dynamically enabling tools, you (goose) as the agent save context window space and allow the user to dynamically retrieve the most relevant tools.
    Be sure to format a query packed with relevant keywords to search for the most relevant tools.
    In addition to the extension names available to you, you also have platform extension tools available to you.
    The platform extensions contains the following tools:
    - {}
    - {}
    - {}
    - {}
    - {}
    "#,
        SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME,
        MANAGE_EXTENSIONS_TOOL_NAME,
        READ_RESOURCE_TOOL_NAME,
        LIST_RESOURCES_TOOL_NAME,
        SEARCH_TOOLS_TOOL_NAME
    )
}

pub struct CoreClient {
    info: InitializeResult,
    #[allow(dead_code)]
    context: PlatformExtensionContext,
}

impl CoreClient {
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
                Core Extension

                This extension provides tools to review MCP resources and tool discovery capabilities.

                Available tools:
                - list_resources: List resources from extensions. This tool is only available if any of the extensions supports resources.
                - read_resource: Read specific resources from extensions. This tool is only available if any of the extensions supports resources.
                - search_tools: Search for relevant tools based on user messages

                Use list_resources and read_resource to work with extension data and resources.
                Use search_tools to dynamically discover and retrieve the most relevant tools for a given task.
            "#}
                .to_string(),
            ),
        };

        Ok(Self { info, context })
    }

    async fn handle_list_resources(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, CoreToolError> {
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
                    Err(e) => Err(CoreToolError::OperationFailed {
                        message: format!("Failed to list resources: {}", e.message),
                    }),
                }
            } else {
                Err(CoreToolError::ManagerUnavailable)
            }
        } else {
            Err(CoreToolError::ManagerUnavailable)
        }
    }

    async fn handle_llm_search(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, CoreToolError> {
        if let Some(weak_ref) = &self.context.tool_route_manager {
            if let Some(tool_route_manager) = weak_ref.upgrade() {
                match tool_route_manager
                    .dispatch_route_search_tool(arguments.unwrap_or_default())
                    .await
                {
                    Ok(tool_result) => {
                        tool_result
                            .result
                            .await
                            .map_err(|e| CoreToolError::OperationFailed {
                                message: e.message.to_string(),
                            })
                    }
                    Err(e) => Err(CoreToolError::OperationFailed {
                        message: e.message.to_string(),
                    }),
                }
            } else {
                Err(CoreToolError::ToolRouteManagerUnavailable)
            }
        } else {
            Err(CoreToolError::ToolRouteManagerUnavailable)
        }
    }

    async fn handle_read_resource(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, CoreToolError> {
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
                    Err(e) => Err(CoreToolError::OperationFailed {
                        message: format!("Failed to read resource: {}", e.message),
                    }),
                }
            } else {
                Err(CoreToolError::ManagerUnavailable)
            }
        } else {
            Err(CoreToolError::ManagerUnavailable)
        }
    }

    async fn get_tools(&self) -> Vec<Tool> {
        let mut tools = vec![];

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
                            Arc::new(
                                serde_json::to_value(schema_for!(ListResourcesParams))
                                    .expect("Failed to serialize schema")
                                    .as_object()
                                    .expect("Schema must be an object")
                                    .clone()
                            ),
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
                            Arc::new(
                                serde_json::to_value(schema_for!(ReadResourceParams))
                                    .expect("Failed to serialize schema")
                                    .as_object()
                                    .expect("Schema must be an object")
                                    .clone()
                            ),
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

        // Add search_tools tool if tool route manager is available
        if let Some(weak_ref) = &self.context.tool_route_manager {
            if weak_ref.upgrade().is_some() {
                tools.push(
                    Tool::new(
                        SEARCH_TOOLS_TOOL_NAME.to_string(),
                        indoc! {r#"
            Searches for relevant tools based on the user's messages.
            Format a query to search for the most relevant tools based on the user's messages.
            Pay attention to the keywords in the user's messages, especially the last message and potential tools they are asking for.
            This tool should be invoked when the user's messages suggest they are asking for a tool to be run.
            Use the extension_name parameter to filter tools by the appropriate extension.
            For example, if the user is asking to list the files in the current directory, you filter for the "developer" extension.
            Example: {"User": "list the files in the current directory", "Query": "list files in current directory", "Extension Name": "developer", "k": 5}
            Extension name is not optional, it is required.
            The returned result will be a list of tool names, descriptions, and schemas from which you, the agent can select the most relevant tool to invoke.
        "#}.to_string(),
                        Arc::new(
                            serde_json::to_value(schema_for!(LlmSearchParams))
                                .expect("Failed to serialize schema")
                                .as_object()
                                .expect("Schema must be an object")
                                .clone()
                        ),
                    ).annotate(ToolAnnotations {
                        title: Some("LLM search for relevant tools".to_string()),
                        read_only_hint: Some(true),
                        destructive_hint: Some(false),
                        idempotent_hint: Some(false),
                        open_world_hint: Some(false),
                    })
                );
            }
        }

        tools
    }
}

#[async_trait]
impl McpClientTrait for CoreClient {
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
        // Core extension doesn't expose resources directly
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
            LIST_RESOURCES_TOOL_NAME => self.handle_list_resources(arguments).await,
            READ_RESOURCE_TOOL_NAME => self.handle_read_resource(arguments).await,
            SEARCH_TOOLS_TOOL_NAME => self.handle_llm_search(arguments).await,
            _ => Err(CoreToolError::UnknownTool {
                tool_name: name.to_string(),
            }),
        };

        match result {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => {
                // Log the error for debugging
                error!("Core tool '{}' failed: {}", name, error);

                // Return proper error result with is_error flag set
                Ok(CallToolResult {
                    content: vec![Content::text(error.to_string())],
                    is_error: Some(true),
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
