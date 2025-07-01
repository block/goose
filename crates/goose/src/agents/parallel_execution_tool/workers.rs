use crate::agents::parallel_execution_tool::tasks::process_task;
use crate::agents::parallel_execution_tool::types::{Task, TaskResult};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub struct SharedState {
    pub task_sender: mpsc::Sender<Task>,
    pub task_receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<Task>>>,
    pub result_sender: mpsc::Sender<TaskResult>,
    pub active_workers: Arc<AtomicUsize>,
    pub should_stop: Arc<AtomicBool>,
    pub total_tasks: Arc<AtomicUsize>,
    pub completed_tasks: Arc<AtomicUsize>,
}

// Spawn a worker task
pub fn spawn_worker(state: Arc<SharedState>, worker_id: usize, timeout_seconds: u64) {
    state.active_workers.fetch_add(1, Ordering::SeqCst);

    tokio::spawn(async move {
        worker_loop(state, worker_id, timeout_seconds).await;
    });
}

async fn worker_loop(state: Arc<SharedState>, _worker_id: usize, timeout_seconds: u64) {
    loop {
        // Try to receive a task
        let task = {
            let mut receiver = state.task_receiver.lock().await;
            receiver.recv().await
        };

        match task {
            Some(task) => {
                // Process the task
                let result = process_task(&task, timeout_seconds).await;

                // Send result
                let _ = state.result_sender.send(result).await;

                // Update completed count
                state.completed_tasks.fetch_add(1, Ordering::SeqCst);
            }
            None => {
                // Channel closed, exit worker
                break;
            }
        }

        // Check if we should stop
        if state.should_stop.load(Ordering::SeqCst) {
            break;
        }
    }

    // Worker is exiting
    state.active_workers.fetch_sub(1, Ordering::SeqCst);
}

// Scaling controller that monitors queue and spawns workers
pub async fn run_scaler(
    state: Arc<SharedState>,
    task_count: usize,
    max_workers: usize,
    timeout_seconds: u64,
) {
    let mut worker_count = 0;

    loop {
        sleep(Duration::from_millis(100)).await;

        let active = state.active_workers.load(Ordering::SeqCst);
        let completed = state.completed_tasks.load(Ordering::SeqCst);
        let pending = task_count.saturating_sub(completed);

        // Simple scaling logic: spawn worker if many pending tasks and under limit
        if pending > active * 2 && active < max_workers && worker_count < max_workers {
            spawn_worker(state.clone(), worker_count, timeout_seconds);
            worker_count += 1;
        }

        // If all tasks completed, signal stop
        if completed >= task_count {
            state.should_stop.store(true, Ordering::SeqCst);
            break;
        }

        // If no active workers and tasks remaining, spawn one
        if active == 0 && pending > 0 {
            spawn_worker(state.clone(), worker_count, timeout_seconds);
            worker_count += 1;
        }
    }
}
