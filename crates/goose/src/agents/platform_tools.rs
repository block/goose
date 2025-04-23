use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const PLATFORM_READ_RESOURCE_TOOL_NAME: &str = "platform__read_resource";
pub const PLATFORM_LIST_RESOURCES_TOOL_NAME: &str = "platform__list_resources";
pub const PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME: &str =
    "platform__search_available_extensions";
pub const PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME: &str = "platform__manage_extensions";

pub fn read_resource_tool() -> Tool {
    Tool::new(
        PLATFORM_READ_RESOURCE_TOOL_NAME.to_string(),
        indoc! {r#"
            Read a resource from an extension.

            Resources allow extensions to share data that provide context to LLMs, such as
            files, database schemas, or application-specific information. This tool searches for the
            resource URI in the provided extension, and reads in the resource content. If no extension
            is provided, the tool will search all extensions for the resource.
        "#}.to_string(),
        json!({
            "type": "object",
            "required": ["uri"],
            "properties": {
                "uri": {"type": "string", "description": "Resource URI"},
                "extension_name": {"type": "string", "description": "Optional extension name"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Read a resource".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn list_resources_tool() -> Tool {
    Tool::new(
        PLATFORM_LIST_RESOURCES_TOOL_NAME.to_string(),
        indoc! {r#"
            List resources from an extension(s).

            Resources allow extensions to share data that provide context to LLMs, such as
            files, database schemas, or application-specific information. This tool lists resources
            in the provided extension, and returns a list for the user to browse. If no extension
            is provided, the tool will search all extensions for the resource.
        "#}
        .to_string(),
        json!({
            "type": "object",
            "properties": {
                "extension_name": {"type": "string", "description": "Optional extension name"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("List resources".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn search_available_extensions_tool() -> Tool {
    Tool::new(
        PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME.to_string(),
        "Searches for additional extensions available to help complete tasks.
        Use this tool when you're unable to find a specific feature or functionality you need to complete your task, or when standard approaches aren't working.
        These extensions might provide the exact tools needed to solve your problem.
        If you find a relevant one, consider using your tools to enable it.".to_string(),
        json!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
        Some(ToolAnnotations {
            title: Some("Discover extensions".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn manage_extensions_tool() -> Tool {
    Tool::new(
        PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME.to_string(),
        "Tool to manage extensions and tools in goose context.
            Enable or disable extensions to help complete tasks.
            Enable or disable an extension by providing the extension name.
            "
        .to_string(),
        json!({
            "type": "object",
            "required": ["action", "extension_name"],
            "properties": {
                "action": {"type": "string", "description": "The action to perform", "enum": ["enable", "disable"]},
                "extension_name": {"type": "string", "description": "The name of the extension to enable"}
            }
        }),
        Some(ToolAnnotations {
            title: Some("Enable or disable an extension".to_string()),
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}
