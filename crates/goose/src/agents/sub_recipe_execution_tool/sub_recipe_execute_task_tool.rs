use mcp_core::{tool::ToolAnnotations, Content, Tool, ToolError};
use serde_json::Value;

use crate::agents::{
    sub_recipe_execution_tool::lib::execute_tasks,
    sub_recipe_execution_tool::task_types::ExecutionMode, tool_execution::ToolCallResult,
};
use mcp_core::protocol::JsonRpcMessage;
use tokio::sync::mpsc;
use tokio_stream;

pub const SUB_RECIPE_EXECUTE_TASK_TOOL_NAME: &str = "sub_recipe__execute_task";
pub fn create_sub_recipe_execute_task_tool() -> Tool {
    Tool::new(
        SUB_RECIPE_EXECUTE_TASK_TOOL_NAME,
        "Only use this tool when you execute sub recipe task.
EXECUTION STRATEGY DECISION:
1. PRE-CREATED TASKS: If tasks were created by subrecipe__create_task_* tools, check the execution_mode in the response:
   - If execution_mode is 'parallel', use parallel execution
   - If execution_mode is 'sequential', use sequential execution
   - Always respect the execution_mode from task creation to maintain consistency

2. USER INTENT: If creating tasks inline or user explicitly specifies:
   - DEFAULT: Execute tasks sequentially unless user explicitly requests parallel execution
   - PARALLEL: When user uses keywords like 'parallel', 'simultaneously', 'at the same time', 'concurrently'

IMPLEMENTATION:
- Sequential execution: Call this tool multiple times, passing exactly ONE task per call
- Parallel execution: Call this tool once, passing an ARRAY of all tasks

EXAMPLES:
User Intent Based:
- User: 'get weather and tell me a joke' → Sequential (2 separate tool calls, 1 task each)
- User: 'get weather and joke in parallel' → Parallel (1 tool call with array of 2 tasks)
- User: 'run these simultaneously' → Parallel (1 tool call with task array)
- User: 'do task A then task B' → Sequential (2 separate tool calls)

Pre-created Task Based:
- subrecipe__create_task_weather returns execution_mode: 'parallel' → Use parallel execution
- subrecipe__create_task_weather returns execution_mode: 'sequential' → Use sequential execution",
        serde_json::json!({
            "type": "object",
            "properties": {
                "execution_mode": {
                    "type": "string",
                    "enum": ["sequential", "parallel"],
                    "default": "sequential",
                    "description": "Execution strategy for multiple tasks. For pre-created tasks, respect the execution_mode from task creation. For user intent, use 'sequential' (default) unless user explicitly requests parallel execution with words like 'parallel', 'simultaneously', 'at the same time', or 'concurrently'."
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
                            "timeout_in_seconds": {
                                "type": "number",
                                "description": "timeout in seconds for the task."
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
    let (notification_tx, notification_rx) = mpsc::channel::<JsonRpcMessage>(100);

    let result_future = async move {
        let execute_data_clone = execute_data.clone();
        let execution_mode = execute_data_clone
            .get("execution_mode")
            .and_then(|v| serde_json::from_value::<ExecutionMode>(v.clone()).ok())
            .unwrap_or_default();

        match execute_tasks(execute_data, execution_mode, notification_tx).await {
            Ok(result) => {
                let output = serde_json::to_string(&result).unwrap();
                Ok(vec![Content::text(output)])
            }
            Err(e) => Err(ToolError::ExecutionError(e.to_string())),
        }
    };

    // Convert receiver to stream
    let notification_stream = tokio_stream::wrappers::ReceiverStream::new(notification_rx);

    ToolCallResult {
        result: Box::new(Box::pin(result_future)),
        notification_stream: Some(Box::new(notification_stream)),
    }
}
