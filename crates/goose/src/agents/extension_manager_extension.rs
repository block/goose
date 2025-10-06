use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    ServerCapabilities, ServerNotification, Tool, ToolAnnotations, ToolsCapability,
};
use rmcp::object;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "extensionmanager";

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
                resources: Some(rmcp::model::ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Extension Manager".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {r#"
                Extension Management

                Use these tools to discover, enable, and disable extensions, as well as manage resources.

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

    async fn handle_search_available_extensions(&self) -> Result<Vec<Content>, String> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                match extension_manager.search_available_extensions().await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(format!("Failed to search available extensions: {}", e.message)),
                }
            } else {
                Err("Extension manager is no longer available".to_string())
            }
        } else {
            Err("Extension manager not available in context".to_string())
        }
    }

    async fn handle_manage_extensions(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let action = arguments
            .as_ref()
            .ok_or("Missing arguments")?
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: action")?;

        let extension_name = arguments
            .as_ref()
            .ok_or("Missing arguments")?
            .get("extension_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: extension_name")?;

        // Validate action
        if !matches!(action, "enable" | "disable") {
            return Err("Action must be either 'enable' or 'disable'".to_string());
        }

        // This would normally call the extension manager's manage_extensions method
        Ok(vec![Content::text(format!(
            "Extension management functionality would {} extension '{}' here",
            action, extension_name
        ))])
    }

    async fn handle_list_resources(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                let params = arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                
                match extension_manager.list_resources(params, tokio_util::sync::CancellationToken::default()).await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(format!("Failed to list resources: {}", e.message)),
                }
            } else {
                Err("Extension manager is no longer available".to_string())
            }
        } else {
            Err("Extension manager not available in context".to_string())
        }
    }

    async fn handle_read_resource(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        if let Some(weak_ref) = &self.context.extension_manager {
            if let Some(extension_manager) = weak_ref.upgrade() {
                let params = arguments
                    .map(serde_json::Value::Object)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                
                match extension_manager.read_resource(params, tokio_util::sync::CancellationToken::default()).await {
                    Ok(content) => Ok(content),
                    Err(e) => Err(format!("Failed to read resource: {}", e.message)),
                }
            } else {
                Err("Extension manager is no longer available".to_string())
            }
        } else {
            Err("Extension manager not available in context".to_string())
        }
    }

    fn get_tools() -> Vec<Tool> {
        vec![
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
            tools: Self::get_tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME => {
                self.handle_search_available_extensions().await
            }
            MANAGE_EXTENSIONS_TOOL_NAME => self.handle_manage_extensions(arguments).await,
            LIST_RESOURCES_TOOL_NAME => self.handle_list_resources(arguments).await,
            READ_RESOURCE_TOOL_NAME => self.handle_read_resource(arguments).await,
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
