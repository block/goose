use crate::agents::sub_recipe_execution_tool::task_types::{SharedState, Task};
use crate::agents::sub_recipe_execution_tool::tasks::process_task;
use std::sync::Arc;

async fn receive_task(state: &SharedState) -> Option<Task> {
    let mut receiver = state.task_receiver.lock().await;
    receiver.recv().await
}

pub fn spawn_worker(state: Arc<SharedState>, worker_id: usize) -> tokio::task::JoinHandle<()> {
    state.increment_active_workers();

    tokio::spawn(async move {
        worker_loop(state, worker_id).await;
    })
}

async fn worker_loop(state: Arc<SharedState>, _worker_id: usize) {
    while let Some(task) = receive_task(&state).await {
        state.task_execution_tracker.start_task(&task.id).await;
        let result = process_task(&task, state.task_execution_tracker.clone()).await;

        if let Err(e) = state.result_sender.send(result).await {
            tracing::error!("Worker failed to send result: {}", e);
            break;
        }
    }

    state.decrement_active_workers();
}
