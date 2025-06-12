use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const SUBAGENT_SPAWN_INTERACTIVE_TOOL_NAME: &str =
    "subagent__spawn_interactive";
pub const SUBAGENT_LIST_TOOL_NAME: &str = "subagent__list";
pub const SUBAGENT_CHECK_PROGRESS_TOOL_NAME: &str = "subagent__check_progress";
pub const SUBAGENT_SEND_MESSAGE_TOOL_NAME: &str = "subagent__send_message";

pub fn spawn_interactive_subagent_tool() -> Tool {
    Tool::new(
        SUBAGENT_SPAWN_INTERACTIVE_TOOL_NAME.to_string(),
        indoc! {r#"
            Spawn a specialized subagent to handle specific tasks independently.
            
            Subagents are configured using recipes that define their instructions, extensions, and behavior.
            Each subagent maintains its own conversation history and can be used for specialized tasks
            like research, code review, or interactive assistance.
            
            The subagent will process the initial message and be ready for further interaction.
            Use other subagent tools to manage, communicate with, or terminate the subagent.
        "#}.to_string(),
        json!({
            "type": "object",
            "required": ["recipe_name", "message"],
            "properties": {
                "recipe_name": {
                    "type": "string", 
                    "description": "Name of the recipe file to configure the subagent (e.g., 'research_assistant_recipe.yaml')"
                },
                "message": {
                    "type": "string", 
                    "description": "Initial message to send to the subagent"
                },
                "max_turns": {
                    "type": "integer", 
                    "description": "Optional maximum number of conversation turns (default: unlimited)",
                    "minimum": 1
                },
                "timeout_seconds": {
                    "type": "integer", 
                    "description": "Optional timeout for the subagent in seconds",
                    "minimum": 1
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Spawn interactive subagent".to_string()),
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}

pub fn list_subagents_tool() -> Tool {
    Tool::new(
        SUBAGENT_LIST_TOOL_NAME.to_string(),
        "List all active subagents and their basic information.
        Returns a list of subagent IDs and their current status."
            .to_string(),
        json!({
            "type": "object",
            "required": [],
            "properties": {}
        }),
        Some(ToolAnnotations {
            title: Some("List active subagents".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }),
    )
}

pub fn check_subagent_progress_tool() -> Tool {
    Tool::new(
        SUBAGENT_CHECK_PROGRESS_TOOL_NAME.to_string(),
        indoc! {r#"
            Check the progress and status of subagents.
            
            If subagent_id is provided, returns detailed progress information for that specific subagent.
            If no subagent_id is provided, returns progress for all active subagents.
            
            Progress information includes current state, turn count, and optionally the full conversation history.
        "#}.to_string(),
        json!({
            "type": "object",
            "properties": {
                "subagent_id": {
                    "type": "string", 
                    "description": "Optional ID of specific subagent to check progress for"
                },
                "include_conversation": {
                    "type": "boolean", 
                    "description": "Whether to include full conversation history (default: false)",
                    "default": false
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Check subagent progress".to_string()),
            read_only_hint: true,
            destructive_hint: false,
            idempotent_hint: true,
            open_world_hint: false,
        }),
    )
}

pub fn send_message_to_subagent_tool() -> Tool {
    Tool::new(
        SUBAGENT_SEND_MESSAGE_TOOL_NAME.to_string(),
        indoc! {r#"
            Send a message to an existing subagent.
            
            This tool allows you to continue interacting with a previously spawned subagent.
            The subagent will process the message and maintain its conversation history.
            
            Use subagent__list to see available subagents and subagent__check_progress to monitor their status.
        "#}.to_string(),
        json!({
            "type": "object",
            "required": ["subagent_id", "message"],
            "properties": {
                "subagent_id": {
                    "type": "string", 
                    "description": "ID of the subagent to send the message to"
                },
                "message": {
                    "type": "string", 
                    "description": "Message to send to the subagent"
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Send message to subagent".to_string()),
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}
