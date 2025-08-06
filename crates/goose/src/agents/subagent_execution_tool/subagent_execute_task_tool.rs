use mcp_core::ToolError;
use rmcp::model::{Content, ServerNotification, Tool, ToolAnnotations};
use serde_json::Value;

use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::{
    subagent_execution_tool::lib::execute_tasks,
    subagent_execution_tool::task_types::ExecutionMode,
    subagent_execution_tool::tasks_manager::TasksManager, tool_execution::ToolCallResult,
};
use rmcp::object;
use tokio::sync::mpsc;
use tokio_stream;
use tokio_util::sync::CancellationToken;

pub const SUBAGENT_EXECUTE_TASK_TOOL_NAME: &str = "subagent__execute_task";
pub fn create_subagent_execute_task_tool() -> Tool {
    Tool::new(
        SUBAGENT_EXECUTE_TASK_TOOL_NAME,
        "Only use the subagent__execute_task tool when you execute sub recipe task or dynamic task.
        EXECUTION STRATEGY DECISION:
        1. If the tasks are created with execution_mode, use the execution_mode.
        2. Execute tasks sequentially unless user explicitly requests parallel execution. PARALLEL: User uses keywords like 'parallel', 'simultaneously', 'at the same time', 'concurrently'

        IMPLEMENTATION:
        - Sequential execution: Call this tool multiple times, passing exactly ONE task per call
        - Parallel execution: Call this tool once, passing an ARRAY of all tasks

        EXAMPLES:
        User Intent Based:
        - User: 'get weather and tell me a joke' → Sequential (2 separate tool calls, 1 task each)
        - User: 'get weather and joke in parallel' → Parallel (1 tool call with array of 2 tasks)
        - User: 'run these simultaneously' → Parallel (1 tool call with task array)
        - User: 'do task A then task B' → Sequential (2 separate tool calls)",
        object!({
            "type": "object",
            "properties": {
                "execution_mode": {
                    "type": "string",
                    "enum": ["sequential", "parallel"],
                    "default": "sequential",
                    "description": "Execution strategy for multiple tasks. Use 'sequential' (default) unless user explicitly requests parallel execution with words like 'parallel', 'simultaneously', 'at the same time', or 'concurrently'."
                },
                "task_ids": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "description": "Unique identifier for the task"
                    }
                }
            },
            "required": ["task_ids"]
        })
    ).annotate(ToolAnnotations {
        title: Some("Run tasks in parallel".to_string()),
        read_only_hint: Some(false),
        destructive_hint: Some(true),
        idempotent_hint: Some(false),
        open_world_hint: Some(true),
    })
}

pub async fn run_tasks(
    execute_data: Value,
    task_config: TaskConfig,
    tasks_manager: &TasksManager,
    cancellation_token: Option<CancellationToken>,
) -> ToolCallResult {
    let (notification_tx, notification_rx) = mpsc::channel::<ServerNotification>(100);

    let tasks_manager_clone = tasks_manager.clone();
    let result_future = async move {
        let execute_data_clone = execute_data.clone();
        let execution_mode = execute_data_clone
            .get("execution_mode")
            .and_then(|v| serde_json::from_value::<ExecutionMode>(v.clone()).ok())
            .unwrap_or_default();

        match execute_tasks(
            execute_data,
            execution_mode,
            notification_tx,
            task_config,
            &tasks_manager_clone,
            cancellation_token,
        )
        .await
        {
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
