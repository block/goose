use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::agents::sub_recipe_execution_tool::lib::{
    Config, ExecutionResponse, ExecutionStats, Task, TaskResult,
};
use crate::agents::sub_recipe_execution_tool::tasks::process_task;
use crate::agents::sub_recipe_execution_tool::workers::{run_scaler, spawn_worker, SharedState};

pub async fn execute_single_task(task: &Task, config: Config) -> ExecutionResponse {
    let start_time = Instant::now();
    let result = process_task(task, config.timeout_seconds).await;

    let execution_time = start_time.elapsed().as_millis();
    let completed = if result.status == "success" { 1 } else { 0 };
    let failed = if result.status == "failed" { 1 } else { 0 };

    ExecutionResponse {
        status: "completed".to_string(),
        results: vec![result],
        stats: ExecutionStats {
            total_tasks: 1,
            completed,
            failed,
            execution_time_ms: execution_time,
        },
    }
}

// Main parallel execution function
pub async fn parallel_execute(tasks: Vec<Task>, config: Config) -> ExecutionResponse {
    let start_time = Instant::now();
    let task_count = tasks.len();

    // Create channels
    let (task_tx, task_rx) = mpsc::channel::<Task>(task_count);
    let (result_tx, mut result_rx) = mpsc::channel::<TaskResult>(task_count);

    // Initialize shared state
    let shared_state = Arc::new(SharedState {
        task_receiver: Arc::new(tokio::sync::Mutex::new(task_rx)),
        result_sender: result_tx,
        active_workers: Arc::new(AtomicUsize::new(0)),
        should_stop: Arc::new(AtomicBool::new(false)),
        completed_tasks: Arc::new(AtomicUsize::new(0)),
    });

    // Send all tasks to the queue
    for task in tasks.clone() {
        let _ = task_tx.send(task).await;
    }
    // Close sender so workers know when queue is empty
    drop(task_tx);

    // Start initial workers
    let mut worker_handles = Vec::new();
    for i in 0..config.initial_workers {
        let handle = spawn_worker(shared_state.clone(), i, config.timeout_seconds);
        worker_handles.push(handle);
    }

    // Start the scaler
    let scaler_state = shared_state.clone();
    let scaler_handle = tokio::spawn(async move {
        run_scaler(
            scaler_state,
            task_count,
            config.max_workers,
            config.timeout_seconds,
        )
        .await;
    });

    // Collect results
    let mut results = Vec::new();
    while let Some(result) = result_rx.recv().await {
        results.push(result);
        if results.len() >= task_count {
            break;
        }
    }

    // Wait for scaler to finish
    let _ = scaler_handle.await;

    // Calculate stats
    let execution_time = start_time.elapsed().as_millis();
    let completed = results.iter().filter(|r| r.status == "success").count();
    let failed = results.iter().filter(|r| r.status == "failed").count();

    ExecutionResponse {
        status: "completed".to_string(),
        results,
        stats: ExecutionStats {
            total_tasks: task_count,
            completed,
            failed,
            execution_time_ms: execution_time,
        },
    }
}
