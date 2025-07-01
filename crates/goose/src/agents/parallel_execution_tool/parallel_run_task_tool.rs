use mcp_core::{tool::ToolAnnotations, Content, Tool, ToolError};
use serde_json::Value;

use crate::agents::{parallel_execution_tool::lib::llm_parallel_execute, tool_execution::ToolCallResult};

pub const PARALLEL_RUN_TASK_TOOL_NAME_PREFIX: &str = "parallel__run_task";
pub fn create_parallel_run_task_tool() -> Tool {
    Tool::new(
        PARALLEL_RUN_TASK_TOOL_NAME_PREFIX,
        "Run tasks in parallel",
        serde_json::json!({
            "type": "object",
            "properties": {
                "tasks": {
                    "type": "array",
                    "items": {
                        "type": "string",
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
            }
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
    match llm_parallel_execute(execute_data).await {
        Ok(result) => {
            let output = serde_json::to_string(&result).unwrap();
            ToolCallResult::from(Ok(vec![Content::text(output)]))
        },
        Err(e) => ToolCallResult::from(Err(ToolError::ExecutionError(e.to_string()))),
    }
}