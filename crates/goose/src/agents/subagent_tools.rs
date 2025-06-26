use indoc::indoc;
use mcp_core::tool::{Tool, ToolAnnotations};
use serde_json::json;

pub const SUBAGENT_RUN_TASK_TOOL_NAME: &str = "subagent__run_task";

pub fn run_task_subagent_tool() -> Tool {
    Tool::new(
        SUBAGENT_RUN_TASK_TOOL_NAME.to_string(),
        indoc! {r#"
            Spawn a specialized subagent to handle a specific task completely and automatically.
            
            This tool creates a subagent, processes your task through a complete conversation,
            and returns the final result. The subagent is automatically cleaned up after completion.
            
            You can configure the subagent in two ways:
            1. Using a recipe file that defines instructions, extensions, and behavior
            2. Providing direct instructions for ad-hoc tasks
            
            The subagent will work autonomously until the task is complete, it reaches max_turns,
            or it encounters an error. You'll get the final result without needing to manage
            the subagent lifecycle manually.
            
            Examples:
            - "Convert these unittest files to pytest format: file1.py, file2.py"
            - "Research the latest developments in AI and provide a comprehensive summary"
            - "Review this code for security vulnerabilities and suggest fixes"
            - "Refactor this legacy code to use modern Python patterns"
        "#}
        .to_string(),
        json!({
            "type": "object",
            "required": ["task"],
            "properties": {
                "recipe_name": {
                    "type": "string",
                    "description": "Name of the recipe file to configure the subagent (e.g., 'research_assistant_recipe.yaml'). Either this or 'instructions' must be provided."
                },
                "instructions": {
                    "type": "string",
                    "description": "Direct instructions for the subagent's task. Either this or 'recipe_name' must be provided. Example: 'You are a code refactoring assistant. Help convert unittest tests to pytest format.'"
                },
                "task": {
                    "type": "string",
                    "description": "The task description or initial message for the subagent to work on"
                },
                "max_turns": {
                    "type": "integer",
                    "description": "Maximum number of conversation turns before auto-completion (default: 10)",
                    "minimum": 1,
                    "default": 10
                },
                "timeout_seconds": {
                    "type": "integer",
                    "description": "Optional timeout for the entire task in seconds",
                    "minimum": 1
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Run subagent task".to_string()),
            read_only_hint: false,
            destructive_hint: false,
            idempotent_hint: false,
            open_world_hint: false,
        }),
    )
}
