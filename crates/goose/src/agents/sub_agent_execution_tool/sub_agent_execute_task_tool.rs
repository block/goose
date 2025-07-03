use mcp_core::{tool::ToolAnnotations, Content, Tool, ToolError};
use serde_json::Value;

use crate::agents::{sub_agent_execution_tool::lib::execute_tasks, tool_execution::ToolCallResult};

pub const SUB_AGENT_EXECUTE_TASK_TOOL_NAME: &str = "sub_recipe__execute_task";
pub fn create_sub_agent_execute_task_tool() -> Tool {
    Tool::new(
        SUB_AGENT_EXECUTE_TASK_TOOL_NAME,
        "Only use this tool when you want to execute sub recipe task. **DO NOT** use this tool when you want to execute sub agent task.   
        If the tasks are not specified to be executed in parallel, you should use this tool to run each task immediately by passing a single task to the tool for each run.
        If you want to execute tasks in parallel, you should pass a list of tasks to the tool.",
        serde_json::json!({
            "type": "object",
            "properties": {
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
    match execute_tasks(execute_data).await {
        Ok(result) => {
            let output = serde_json::to_string(&result).unwrap();
            ToolCallResult::from(Ok(vec![Content::text(output)]))
        }
        Err(e) => ToolCallResult::from(Err(ToolError::ExecutionError(e.to_string()))),
    }
}
