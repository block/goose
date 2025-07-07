use mcp_core::{tool::ToolAnnotations, Content, Tool, ToolError};
use serde_json::Value;

use crate::agents::{
    sub_recipe_execution_tool::lib::execute_tasks, tool_execution::ToolCallResult,
};

pub const SUB_RECIPE_EXECUTE_TASK_TOOL_NAME: &str = "sub_recipe__execute_task";
pub fn create_sub_recipe_execute_task_tool() -> Tool {
    Tool::new(
        SUB_RECIPE_EXECUTE_TASK_TOOL_NAME,
        "Only use this tool when you execute sub recipe task.
EXECUTION STRATEGY:
- DEFAULT: Execute tasks sequentially (one at a time) unless user explicitly requests parallel execution
- PARALLEL: Only when user explicitly uses keywords like 'parallel', 'simultaneously', 'at the same time', 'concurrently'

IMPLEMENTATION:
- Sequential execution: Call this tool multiple times, passing exactly ONE task per call
- Parallel execution: Call this tool once, passing an ARRAY of all tasks

EXAMPLES:
- User: 'get weather and tell me a joke' → Sequential (2 separate tool calls, 1 task each)
- User: 'get weather and joke in parallel' → Parallel (1 tool call with array of 2 tasks)
- User: 'run these simultaneously' → Parallel (1 tool call with task array)
- User: 'do task A then task B' → Sequential (2 separate tool calls)",
        serde_json::json!({
            "type": "object",
            "properties": {
                "execution_mode": {
                    "type": "string",
                    "enum": ["sequential", "parallel"],
                    "default": "sequential",
                    "description": "Execution strategy for multiple tasks. Use 'sequential' (default) unless user explicitly requests parallel execution with words like 'parallel', 'simultaneously', 'at the same time', or 'concurrently'."
                },
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Unique identifier for the task"
                            },
                            "task_type": {
                                "type": "string",
                                "enum": ["sub_recipe", "text_instruction"],
                                "default": "sub_recipe",
                                "description": "the type of task to execute, can be one of: sub_recipe, text_instruction"
                            },
                            "payload": {
                                "type": "object",
                                "properties": {
                                    "sub_recipe": {
                                        "type": "object",
                                        "description": "sub recipe to execute",
                                        "properties": {
                                            "name": {
                                                "type": "string",
                                                "description": "name of the sub recipe to execute"
                                            },
                                            "recipe_path": {
                                                "type": "string",
                                                "description": "path of the sub recipe file"
                                            },
                                            "command_parameters": {
                                                "type": "object",
                                                "description": "parameters to pass to run recipe command with sub recipe file"
                                            }
                                        }
                                    },
                                    "text_instruction": {
                                        "type": "string",
                                        "description": "text instruction to execute"
                                    }
                                }
                            }
                        },
                        "required": ["id", "payload"]
                    },
                    "description": "The tasks to run in parallel"
                },
                "config": {
                    "type": "object",
                    "properties": {
                        "timeout_seconds": {
                            "type": "number"
                        },
                        "max_workers": {
                            "type": "number"
                        },
                        "initial_workers": {
                            "type": "number"
                        }
                    }
                }
            },
            "required": ["tasks"]
        }),
        Some(ToolAnnotations {
            title: Some("Run tasks in parallel".to_string()),
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: true,
        }),
    )
}

pub async fn run_tasks(execute_data: Value) -> ToolCallResult {
    let execute_data_clone = execute_data.clone();
    let default_execution_mode_value = Value::String("sequential".to_string());
    let execution_mode = execute_data_clone
        .get("execution_mode")
        .unwrap_or(&default_execution_mode_value)
        .as_str()
        .unwrap_or("sequential");
    match execute_tasks(execute_data, execution_mode).await {
        Ok(result) => {
            let output = serde_json::to_string(&result).unwrap();
            ToolCallResult::from(Ok(vec![Content::text(output)]))
        }
        Err(e) => ToolCallResult::from(Err(ToolError::ExecutionError(e.to_string()))),
    }
}
