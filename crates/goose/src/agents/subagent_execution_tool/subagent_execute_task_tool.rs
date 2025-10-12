use std::borrow::Cow;

use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::{
    subagent_execution_tool::lib::execute_tasks,
    subagent_execution_tool::task_types::ExecutionMode,
    subagent_execution_tool::tasks_manager::TasksManager, tool_execution::ToolCallResult,
};
use rmcp::model::{Content, ErrorCode, ErrorData, ServerNotification, Tool, ToolAnnotations};
use rmcp::object;
use tokio::sync::mpsc;
use tokio_stream;
use tokio_util::sync::CancellationToken;

pub const SUBAGENT_EXECUTE_TASK_TOOL_NAME: &str = "subagent__execute_task";
pub fn create_subagent_execute_task_tool() -> Tool {
    Tool::new(
        SUBAGENT_EXECUTE_TASK_TOOL_NAME,
        "Execute tasks. Mode: task's mode>SEQ(default)>PAR(if parallel/simultaneous/concurrent). SEQ: multi-call/1-task. PAR: 1-call/array.",
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
    task_ids: Vec<String>,
    execution_mode: ExecutionMode,
    task_config: TaskConfig,
    tasks_manager: &TasksManager,
    cancellation_token: Option<CancellationToken>,
) -> ToolCallResult {
    let (notification_tx, notification_rx) = mpsc::channel::<ServerNotification>(100);

    let tasks_manager_clone = tasks_manager.clone();
    let result_future = async move {
        match execute_tasks(
            task_ids,
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
            Err(e) => Err(ErrorData {
                code: ErrorCode::INTERNAL_ERROR,
                message: Cow::from(e.to_string()),
                data: None,
            }),
        }
    };

    // Convert receiver to stream
    let notification_stream = tokio_stream::wrappers::ReceiverStream::new(notification_rx);

    ToolCallResult {
        result: Box::new(Box::pin(result_future)),
        notification_stream: Some(Box::new(notification_stream)),
    }
}
