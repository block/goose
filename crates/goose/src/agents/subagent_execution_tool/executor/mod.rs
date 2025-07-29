use crate::agents::subagent_execution_tool::lib::{
    ExecutionResponse, ExecutionStats, SharedState, Task, TaskResult, TaskStatus,
};
use crate::agents::subagent_execution_tool::task_execution_tracker::{
    DisplayMode, TaskExecutionTracker,
};
use crate::agents::subagent_execution_tool::tasks::process_task;
use crate::agents::subagent_execution_tool::workers::spawn_worker;
use crate::agents::subagent_task_config::TaskConfig;
use rmcp::model::ServerNotification;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

const EXECUTION_STATUS_COMPLETED: &str = "completed";
const DEFAULT_MAX_WORKERS: usize = 10;

pub async fn execute_single_task(
    task: &Task,
    notifier: mpsc::Sender<ServerNotification>,
    task_config: TaskConfig,
    cancellation_token: Option<CancellationToken>,
) -> ExecutionResponse {
    let start_time = Instant::now();
    let task_execution_tracker = Arc::new(TaskExecutionTracker::new(
        vec![task.clone()],
        DisplayMode::SingleTaskOutput,
        notifier,
        cancellation_token.clone(),
    ));
    let result = process_task(
        task,
        task_execution_tracker.clone(),
        task_config,
        cancellation_token.unwrap_or_default(),
    )
    .await;

    // Complete the task in the tracker
    task_execution_tracker
        .complete_task(&result.task_id, result.clone())
        .await;

    let execution_time = start_time.elapsed().as_millis();
    let stats = calculate_stats(&[result.clone()], execution_time);

    ExecutionResponse {
        status: EXECUTION_STATUS_COMPLETED.to_string(),
        results: vec![result],
        stats,
    }
}

pub async fn execute_tasks_in_parallel(
    tasks: Vec<Task>,
    notifier: Sender<ServerNotification>,
    task_config: TaskConfig,
    cancellation_token: Option<CancellationToken>,
) -> ExecutionResponse {
    let task_execution_tracker = Arc::new(TaskExecutionTracker::new(
        tasks.clone(),
        DisplayMode::MultipleTasksOutput,
        notifier,
        cancellation_token.clone(),
    ));
    let start_time = Instant::now();
    let task_count = tasks.len();

    if task_count == 0 {
        return create_empty_response();
    }

    task_execution_tracker.refresh_display().await;

    let (task_tx, task_rx, result_tx, mut result_rx) = create_channels(task_count);

    if let Err(e) = send_tasks_to_channel(tasks, task_tx).await {
        tracing::error!("Task execution failed: {}", e);
        return create_error_response(e);
    }

    let shared_state = create_shared_state(
        task_rx,
        result_tx,
        task_execution_tracker.clone(),
        cancellation_token.unwrap_or_default(),
    );

    let worker_count = std::cmp::min(task_count, DEFAULT_MAX_WORKERS);
    let mut worker_handles = Vec::new();
    for i in 0..worker_count {
        let handle = spawn_worker(shared_state.clone(), i, task_config.clone());
        worker_handles.push(handle);
    }

    let results = collect_results(&mut result_rx, task_execution_tracker.clone(), task_count).await;

    for handle in worker_handles {
        if let Err(e) = handle.await {
            tracing::error!("Worker error: {}", e);
        }
    }

    task_execution_tracker.send_tasks_complete().await;

    let execution_time = start_time.elapsed().as_millis();
    let stats = calculate_stats(&results, execution_time);

    ExecutionResponse {
        status: EXECUTION_STATUS_COMPLETED.to_string(),
        results,
        stats,
    }
}

fn calculate_stats(results: &[TaskResult], execution_time_ms: u128) -> ExecutionStats {
    let completed = results
        .iter()
        .filter(|r| matches!(r.status, TaskStatus::Completed))
        .count();
    let failed = results
        .iter()
        .filter(|r| matches!(r.status, TaskStatus::Failed))
        .count();

    ExecutionStats {
        total_tasks: results.len(),
        completed,
        failed,
        execution_time_ms,
    }
}

fn create_channels(
    task_count: usize,
) -> (
    mpsc::Sender<Task>,
    mpsc::Receiver<Task>,
    mpsc::Sender<TaskResult>,
    mpsc::Receiver<TaskResult>,
) {
    let (task_tx, task_rx) = mpsc::channel::<Task>(task_count);
    let (result_tx, result_rx) = mpsc::channel::<TaskResult>(task_count);
    (task_tx, task_rx, result_tx, result_rx)
}

fn create_shared_state(
    task_rx: mpsc::Receiver<Task>,
    result_tx: mpsc::Sender<TaskResult>,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    cancellation_token: CancellationToken,
) -> Arc<SharedState> {
    Arc::new(SharedState {
        task_receiver: Arc::new(tokio::sync::Mutex::new(task_rx)),
        result_sender: result_tx,
        active_workers: Arc::new(AtomicUsize::new(0)),
        task_execution_tracker,
        cancellation_token,
    })
}

async fn send_tasks_to_channel(
    tasks: Vec<Task>,
    task_tx: mpsc::Sender<Task>,
) -> Result<(), String> {
    for task in tasks {
        task_tx
            .send(task)
            .await
            .map_err(|e| format!("Failed to queue task: {}", e))?;
    }
    Ok(())
}

fn create_empty_response() -> ExecutionResponse {
    ExecutionResponse {
        status: EXECUTION_STATUS_COMPLETED.to_string(),
        results: vec![],
        stats: ExecutionStats {
            total_tasks: 0,
            completed: 0,
            failed: 0,
            execution_time_ms: 0,
        },
    }
}
async fn collect_results(
    result_rx: &mut mpsc::Receiver<TaskResult>,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    expected_count: usize,
) -> Vec<TaskResult> {
    let mut results = Vec::new();
    while let Some(result) = result_rx.recv().await {
        task_execution_tracker
            .complete_task(&result.task_id, result.clone())
            .await;

        results.push(result);
        if results.len() >= expected_count {
            break;
        }
    }
    results
}

fn create_error_response(error: String) -> ExecutionResponse {
    tracing::error!("Creating error response: {}", error);
    ExecutionResponse {
        status: "failed".to_string(),
        results: vec![],
        stats: ExecutionStats {
            total_tasks: 0,
            completed: 0,
            failed: 1,
            execution_time_ms: 0,
        },
    }
}
