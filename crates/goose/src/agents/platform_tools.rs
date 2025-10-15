use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;

pub const PLATFORM_READ_RESOURCE_TOOL_NAME: &str = "platform__read_resource";
pub const PLATFORM_LIST_RESOURCES_TOOL_NAME: &str = "platform__list_resources";
pub const PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME: &str =
    "platform__search_available_extensions";
pub const PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME: &str = "platform__manage_extensions";
pub const PLATFORM_MANAGE_SCHEDULE_TOOL_NAME: &str = "platform__manage_schedule";

pub fn read_resource_tool() -> Tool {
    Tool::new(
        PLATFORM_READ_RESOURCE_TOOL_NAME.to_string(),
        "Read extension resource (files/schemas/data). Searches URI in extension or all if none specified.".to_string(),
        object!({
            "type": "object",
            "required": ["uri"],
            "properties": {
                "uri": {"type": "string", "description": "Resource URI"},
                "extension_name": {"type": "string", "description": "Optional extension name"}
            }
        })
    ).annotate(ToolAnnotations {
        title: Some("Read a resource".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

pub fn list_resources_tool() -> Tool {
    Tool::new(
        PLATFORM_LIST_RESOURCES_TOOL_NAME.to_string(),
        "List extension resources. Returns browsable list. Searches all if no extension specified."
            .to_string(),
        object!({
            "type": "object",
            "properties": {
                "extension_name": {"type": "string", "description": "Optional extension name"}
            }
        }),
    )
    .annotate(ToolAnnotations {
        title: Some("List resources".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

pub fn search_available_extensions_tool() -> Tool {
    Tool::new(
        PLATFORM_SEARCH_AVAILABLE_EXTENSIONS_TOOL_NAME.to_string(),
        "Find missing functionality/extensions. Enable found items via manage_extensions."
            .to_string(),
        object!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
    )
    .annotate(ToolAnnotations {
        title: Some("Discover extensions".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

pub fn manage_extensions_tool() -> Tool {
    Tool::new(
        PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME.to_string(),
        "Enable or disable extensions. Actions: enable|disable.".to_string(),
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
    })
}

pub fn manage_schedule_tool() -> Tool {
    Tool::new(
        PLATFORM_MANAGE_SCHEDULE_TOOL_NAME.to_string(),
        "Schedule recipes (this goose). Jobs: list|create[file]|delete. Control: run_now|pause|unpause|kill[proc]. View: inspect[job]|sessions|content.".to_string(),
        object!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "create", "run_now", "pause", "unpause", "delete", "kill", "inspect", "sessions", "session_content"]
                },
                "job_id": {"type": "string", "description": "Job identifier for operations on existing jobs"},
                "recipe_path": {"type": "string", "description": "Path to recipe file for create action"},
                "cron_expression": {"type": "string", "description": "A cron expression for create action. Supports both 5-field (minute hour day month weekday) and 6-field (second minute hour day month weekday) formats. 5-field expressions are automatically converted to 6-field by prepending '0' for seconds."},
                "execution_mode": {"type": "string", "description": "Execution mode for create action: 'foreground' or 'background'", "enum": ["foreground", "background"], "default": "background"},
                "limit": {"type": "integer", "description": "Limit for sessions list", "default": 50},
                "session_id": {"type": "string", "description": "Session identifier for session_content action"}
            }
        }),
    ).annotate(ToolAnnotations {
        title: Some("Manage scheduled recipes".to_string()),
        read_only_hint: Some(false),
        destructive_hint: Some(true), // Can kill jobs
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}
