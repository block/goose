pub use crate::agents::subagent_execution_tool::task_types::{
    ExecutionMode, ExecutionResponse, ExecutionStats, SharedState, Task, TaskResult, TaskStatus,
};
use crate::agents::subagent_execution_tool::{
    executor::{execute_single_task, execute_tasks_in_parallel},
    tasks_manager::TasksManager,
};
use crate::agents::subagent_task_config::TaskConfig;
use rmcp::model::ServerNotification;
use serde_json::{json, Value};
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

pub async fn execute_tasks(
    input: Value,
    execution_mode: ExecutionMode,
    notifier: Sender<ServerNotification>,
    task_config: TaskConfig,
    tasks_manager: &TasksManager,
    cancellation_token: Option<CancellationToken>,
) -> Result<Value, String> {
    let task_ids: Vec<String> = serde_json::from_value(
        input
            .get("task_ids")
            .ok_or("Missing task_ids field")?
            .clone(),
    )
    .map_err(|e| format!("Failed to parse task_ids: {}", e))?;

    let tasks = tasks_manager.get_tasks(&task_ids).await?;

    let task_count = tasks.len();
    match execution_mode {
        ExecutionMode::Sequential => {
            if task_count == 1 {
                let response =
                    execute_single_task(&tasks[0], notifier, task_config, cancellation_token).await;
                handle_response(response)
            } else {
                Err("Sequential execution mode requires exactly one task".to_string())
            }
        }
        ExecutionMode::Parallel => {
            if tasks.iter().any(|task| task.get_sequential_when_repeated()) {
                Ok(json!(
                    {
                        "execution_mode": ExecutionMode::Sequential,
                        "task_ids": task_ids,
                        "results": ["the tasks should be executed sequentially, no matter how user requests it. Please use the subrecipe__execute_task tool to execute the tasks sequentially."]
                    }
                ))
            } else {
                let response: ExecutionResponse = execute_tasks_in_parallel(
                    tasks,
                    notifier.clone(),
                    task_config,
                    cancellation_token,
                )
                .await;
                handle_response(response)
            }
        }
    }
}

fn extract_failed_tasks(results: &[TaskResult]) -> Vec<String> {
    results
        .iter()
        .filter(|r| matches!(r.status, TaskStatus::Failed))
        .map(format_failed_task_error)
        .collect()
}

fn format_failed_task_error(result: &TaskResult) -> String {
    let error_msg = result.error.as_deref().unwrap_or("Unknown error");
    let partial_output = result
        .data
        .as_ref()
        .and_then(|d| d.get("partial_output"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("No output captured");

    format!(
        "Task '{}' ({}): {}\nOutput: {}",
        result.task_id,
        get_task_description(result),
        error_msg,
        partial_output
    )
}

fn format_error_summary(
    failed_count: usize,
    total_count: usize,
    failed_tasks: Vec<String>,
) -> String {
    format!(
        "{}/{} tasks failed:\n{}",
        failed_count,
        total_count,
        failed_tasks.join("\n")
    )
}

fn handle_response(response: ExecutionResponse) -> Result<Value, String> {
    if response.stats.failed > 0 {
        let failed_tasks = extract_failed_tasks(&response.results);
        let error_summary = format_error_summary(
            response.stats.failed,
            response.stats.total_tasks,
            failed_tasks,
        );
        return Err(error_summary);
    }
    serde_json::to_value(response).map_err(|e| format!("Failed to serialize response: {}", e))
}

fn get_task_description(result: &TaskResult) -> String {
    format!("ID: {}", result.task_id)
}
