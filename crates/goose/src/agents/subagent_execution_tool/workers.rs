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
    loop {
        tokio::select! {
            task_option = receive_task(&state) => {
                match task_option {
                    Some(task) => {
                        state.task_execution_tracker.start_task(&task.id).await;
                        let result = process_task(
                            &task,
                            state.task_execution_tracker.clone(),
                            task_config.clone(),
                            state.cancellation_token.clone(),
                        )
                        .await;

                        if let Err(e) = state.result_sender.send(result).await {
                            // Only log error if not cancelled (channel close is expected during cancellation)
                            if !state.cancellation_token.is_cancelled() {
                                tracing::error!("Worker failed to send result: {}", e);
                            }
                            break;
                        }
                    }
                    None => break, // No more tasks
                }
            }
            _ = state.cancellation_token.cancelled() => {
                tracing::debug!("Worker cancelled");
                break;
            }
        }
    }

    state.decrement_active_workers();
}
