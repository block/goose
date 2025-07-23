use crate::agents::subagent_execution_tool::task_types::{SharedState, Task};
use crate::agents::subagent_execution_tool::tasks::process_task;
use crate::agents::subagent_task_config::TaskConfig;
use std::sync::Arc;

async fn receive_task(state: &SharedState) -> Option<Task> {
    let mut receiver = state.task_receiver.lock().await;
    receiver.recv().await
}

pub fn spawn_worker(
    state: Arc<SharedState>,
    worker_id: usize,
    task_config: TaskConfig,
) -> tokio::task::JoinHandle<()> {
    state.increment_active_workers();

    tokio::spawn(async move {
        worker_loop(state, worker_id, task_config).await;
    })
}

async fn worker_loop(state: Arc<SharedState>, _worker_id: usize, task_config: TaskConfig) {
    while let Some(task) = receive_task(&state).await {
        state.task_execution_tracker.start_task(&task.id).await;
        let result = process_task(
            &task,
            state.task_execution_tracker.clone(),
            task_config.clone(),
        )
        .await;

        if let Err(e) = state.result_sender.send(result).await {
            tracing::error!("Worker failed to send result: {}", e);
            break;
        }
    }

    state.decrement_active_workers();
}
