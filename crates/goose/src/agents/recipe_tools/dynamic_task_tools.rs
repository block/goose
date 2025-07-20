// =======================================
// Module: Dynamic Task Tools
// Handles creation of tasks dynamically without sub-recipes
// =======================================
use crate::agents::subagent_execution_tool::tasks_manager::TasksManager;
use crate::agents::subagent_execution_tool::{lib::ExecutionMode, task_types::Task};
use crate::agents::tool_execution::ToolCallResult;
use mcp_core::{tool::ToolAnnotations, Tool, ToolError};
use rmcp::model::Content;
use serde_json::{json, Value};

pub const DYNAMIC_TASK_TOOL_NAME_PREFIX: &str = "dynamic_task__create_task";

pub fn create_dynamic_task_tool() -> Tool {
    Tool::new(
        DYNAMIC_TASK_TOOL_NAME_PREFIX.to_string(),
        "Use this tool to create one or more dynamic tasks from a shared text instruction and varying parameters.\
            How it works:
            - Provide a single text instruction
            - Use the 'task_parameters' field to pass an array of parameter sets
            - Each resulting task will use the same instruction with different parameter values
            This is useful when performing the same operation across many inputs (e.g., getting weather for multiple cities, searching multiple slack channels, iterating through various linear tickets, etc).
            Once created, these tasks should be passed to the 'subagent__execute_task' tool for execution. Tasks can run sequentially or in parallel.
            ---
            What is a 'subagent'?
            A 'subagent' is a stateless sub-process that executes a single task independently. Use subagents when:
            - You want to parallelize similar work across different inputs
            - You are not sure your search or operation will succeed on the first try
            Each subagent receives a task with a defined payload and returns a result, which is not visible to the user unless explicitly summarized by the system.
            ---
            Examples of 'task_parameters' for a single task:
                text_instruction: Search for the config file in the root directory.
            Examples of 'task_parameters' for multiple tasks:
                text_instruction: Get weather for Melbourne.
                timeout_seconds: 300
                text_instruction: Get weather for Los Angeles.
                timeout_seconds: 300
                text_instruction: Get weather for San Francisco.
                timeout_seconds: 300
            ".to_string(),
        json!({
            "type": "object",
            "properties": {
                "task_parameters": {
                    "type": "array",
                    "description": "Array of parameter sets for creating tasks. \
                        For a single task, provide an array with one element. \
                        For multiple tasks, provide an array with multiple elements, each with different parameter values. \
                        If there is no parameter set, provide an empty array.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "text_instruction": {
                                "type": "string",
                                "description": "The text instruction to execute"
                            },
                            "timeout_seconds": {
                                "type": "integer",
                                "description": "Optional timeout for the task in seconds (default: 300)",
                                "minimum": 1
                            }
                        },
                        "required": ["text_instruction"]
                    }
                }
            }
        }),
        Some(ToolAnnotations {
            title: Some("Dynamic Task Creation".to_string()),
            read_only_hint: false,
            destructive_hint: true,
            idempotent_hint: false,
            open_world_hint: true,
        }),
    )
}

fn extract_task_parameters(params: &Value) -> Vec<Value> {
    params
        .get("task_parameters")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn create_text_instruction_tasks_from_params(task_params: &[Value]) -> Vec<Task> {
    task_params
        .iter()
        .map(|task_param| {
            let text_instruction = task_param
                .get("text_instruction")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let payload = json!({
                "text_instruction": text_instruction
            });

            Task {
                id: uuid::Uuid::new_v4().to_string(),
                task_type: "text_instruction".to_string(),
                payload,
            }
        })
        .collect()
}

fn create_task_execution_payload(tasks: Vec<Task>, execution_mode: ExecutionMode) -> Value {
    let task_ids: Vec<String> = tasks.iter().map(|task| task.id.clone()).collect();
    json!({
        "task_ids": task_ids,
        "execution_mode": execution_mode
    })
}

pub async fn create_dynamic_task(params: Value, tasks_manager: &TasksManager) -> ToolCallResult {
    let task_params_array = extract_task_parameters(&params);

    if task_params_array.is_empty() {
        return ToolCallResult::from(Err(ToolError::ExecutionError(
            "No task parameters provided".to_string(),
        )));
    }

    let tasks = create_text_instruction_tasks_from_params(&task_params_array);

    // Use parallel execution if there are multiple tasks, sequential for single task
    let execution_mode = if tasks.len() > 1 {
        ExecutionMode::Parallel
    } else {
        ExecutionMode::Sequential
    };

    let task_execution_payload = create_task_execution_payload(tasks.clone(), execution_mode);

    let tasks_json = match serde_json::to_string(&task_execution_payload) {
        Ok(json) => json,
        Err(e) => {
            return ToolCallResult::from(Err(ToolError::ExecutionError(format!(
                "Failed to serialize task list: {}",
                e
            ))))
        }
    };
    tasks_manager.save_tasks(tasks.clone()).await;
    ToolCallResult::from(Ok(vec![Content::text(tasks_json)]))
}
