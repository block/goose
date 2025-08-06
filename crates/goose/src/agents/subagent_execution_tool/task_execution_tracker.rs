use rmcp::model::{
    LoggingLevel, LoggingMessageNotification, LoggingMessageNotificationMethod,
    LoggingMessageNotificationParam, ServerNotification,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::notification_events::{
    FailedTaskInfo, TaskCompletionStats, TaskExecutionNotificationEvent, TaskExecutionStats,
    TaskInfo as EventTaskInfo,
};
use crate::agents::subagent_execution_tool::task_types::{Task, TaskInfo, TaskResult, TaskStatus};
use crate::agents::subagent_execution_tool::utils::{count_by_status, get_task_name};
use crate::utils::is_token_cancelled;
use serde_json::Value;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayMode {
    MultipleTasksOutput,
    SingleTaskOutput,
}

const THROTTLE_INTERVAL_MS: u64 = 250;
const COMPLETION_NOTIFICATION_DELAY_MS: u64 = 500;

fn format_task_metadata(task_info: &TaskInfo) -> String {
    if let Some(params) = task_info.task.get_command_parameters() {
        if params.is_empty() {
            return String::new();
        }

        params
            .iter()
            .map(|(key, value)| {
                let value_str = match value {
                    Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                format!("{}={}", key, value_str)
            })
            .collect::<Vec<_>>()
            .join(",")
    } else if let Some(text_instruction) = task_info.task.get_text_instruction() {
        format!("instruction={}", text_instruction)
    } else {
        String::new()
    }
}

pub struct TaskExecutionTracker {
    tasks: Arc<RwLock<HashMap<String, TaskInfo>>>,
    last_refresh: Arc<RwLock<Instant>>,
    notifier: mpsc::Sender<ServerNotification>,
    display_mode: DisplayMode,
    cancellation_token: Option<CancellationToken>,
}

impl TaskExecutionTracker {
    pub fn new(
        tasks: Vec<Task>,
        display_mode: DisplayMode,
        notifier: Sender<ServerNotification>,
        cancellation_token: Option<CancellationToken>,
    ) -> Self {
        let task_map = tasks
            .into_iter()
            .map(|task| {
                let task_id = task.id.clone();
                (
                    task_id,
                    TaskInfo {
                        task,
                        status: TaskStatus::Pending,
                        start_time: None,
                        end_time: None,
                        result: None,
                        current_output: String::new(),
                    },
                )
            })
            .collect();

        Self {
            tasks: Arc::new(RwLock::new(task_map)),
            last_refresh: Arc::new(RwLock::new(Instant::now())),
            notifier,
            display_mode,
            cancellation_token,
        }
    }

    fn is_cancelled(&self) -> bool {
        is_token_cancelled(&self.cancellation_token)
    }

    fn log_notification_error<T>(&self, error: &mpsc::error::TrySendError<T>, context: &str) {
        if !self.is_cancelled() {
            tracing::warn!("Failed to send {} notification: {}", context, error);
        }
    }

    fn try_send_notification(&self, event: TaskExecutionNotificationEvent, context: &str) {
        if let Err(e) = self
            .notifier
            .try_send(ServerNotification::LoggingMessageNotification(
                LoggingMessageNotification {
                    method: LoggingMessageNotificationMethod,
                    params: LoggingMessageNotificationParam {
                        data: event.to_notification_data(),
                        level: LoggingLevel::Info,
                        logger: None,
                    },
                    extensions: Default::default(),
                },
            ))
        {
            self.log_notification_error(&e, context);
        }
    }

    pub async fn start_task(&self, task_id: &str) {
        let mut tasks = self.tasks.write().await;
        if let Some(task_info) = tasks.get_mut(task_id) {
            task_info.status = TaskStatus::Running;
            task_info.start_time = Some(Instant::now());
        }
        drop(tasks);
        self.force_refresh_display().await;
    }

    pub async fn complete_task(&self, task_id: &str, result: TaskResult) {
        let mut tasks = self.tasks.write().await;
        if let Some(task_info) = tasks.get_mut(task_id) {
            task_info.status = result.status.clone();
            task_info.end_time = Some(Instant::now());
            task_info.result = Some(result);
        }
        drop(tasks);
        self.force_refresh_display().await;
    }

    pub async fn get_current_output(&self, task_id: &str) -> Option<String> {
        let tasks = self.tasks.read().await;
        tasks
            .get(task_id)
            .map(|task_info| task_info.current_output.clone())
    }

    async fn format_line(&self, task_info: Option<&TaskInfo>, line: &str) -> String {
        if let Some(task_info) = task_info {
            let task_name = get_task_name(task_info);
            let task_type = task_info.task.task_type.clone();
            let metadata = format_task_metadata(task_info);

            if metadata.is_empty() {
                format!("[{} ({})] {}", task_name, task_type, line)
            } else {
                format!("[{} ({}) {}] {}", task_name, task_type, metadata, line)
            }
        } else {
            line.to_string()
        }
    }

    pub async fn send_live_output(&self, task_id: &str, line: &str) {
        match self.display_mode {
            DisplayMode::SingleTaskOutput => {
                let tasks = self.tasks.read().await;
                let task_info = tasks.get(task_id);

                let formatted_line = self.format_line(task_info, line).await;
                drop(tasks);
                let event = TaskExecutionNotificationEvent::line_output(
                    task_id.to_string(),
                    formatted_line,
                );

                self.try_send_notification(event, "live output");
            }
            DisplayMode::MultipleTasksOutput => {
                let mut tasks = self.tasks.write().await;
                if let Some(task_info) = tasks.get_mut(task_id) {
                    task_info.current_output.push_str(line);
                    task_info.current_output.push('\n');
                }
                drop(tasks);

                if !self.should_throttle_refresh().await {
                    self.refresh_display().await;
                }
            }
        }
    }

    async fn should_throttle_refresh(&self) -> bool {
        let now = Instant::now();
        let mut last_refresh = self.last_refresh.write().await;

        if now.duration_since(*last_refresh) > Duration::from_millis(THROTTLE_INTERVAL_MS) {
            *last_refresh = now;
            false
        } else {
            true
        }
    }

    async fn send_tasks_update(&self) {
        if self.is_cancelled() {
            return;
        }

        let tasks = self.tasks.read().await;
        let task_list: Vec<_> = tasks.values().collect();
        let (total, pending, running, completed, failed) = count_by_status(&tasks);

        let stats = TaskExecutionStats::new(total, pending, running, completed, failed);

        let event_tasks: Vec<EventTaskInfo> = task_list
            .iter()
            .map(|task_info| {
                let now = Instant::now();
                EventTaskInfo {
                    id: task_info.task.id.clone(),
                    status: task_info.status.clone(),
                    duration_secs: task_info.start_time.map(|start| {
                        if let Some(end) = task_info.end_time {
                            end.duration_since(start).as_secs_f64()
                        } else {
                            now.duration_since(start).as_secs_f64()
                        }
                    }),
                    current_output: task_info.current_output.clone(),
                    task_type: task_info.task.task_type.clone(),
                    task_name: get_task_name(task_info).to_string(),
                    task_metadata: format_task_metadata(task_info),
                    error: task_info.error().cloned(),
                    result_data: task_info.data().cloned(),
                }
            })
            .collect();

        let event = TaskExecutionNotificationEvent::tasks_update(stats, event_tasks);

        self.try_send_notification(event, "tasks update");
    }

    pub async fn refresh_display(&self) {
        match self.display_mode {
            DisplayMode::MultipleTasksOutput => {
                self.send_tasks_update().await;
            }
            DisplayMode::SingleTaskOutput => {
                // No dashboard display needed for single task output mode
                // Live output is handled via send_live_output method
            }
        }
    }

    // Force refresh without throttling - used for important status changes
    async fn force_refresh_display(&self) {
        match self.display_mode {
            DisplayMode::MultipleTasksOutput => {
                // Reset throttle timer to allow immediate update
                let mut last_refresh = self.last_refresh.write().await;
                *last_refresh = Instant::now() - Duration::from_millis(THROTTLE_INTERVAL_MS + 1);
                drop(last_refresh);

                self.send_tasks_update().await;
            }
            DisplayMode::SingleTaskOutput => {
                // No dashboard display needed for single task output mode
            }
        }
    }

    pub async fn send_tasks_complete(&self) {
        if self.is_cancelled() {
            return;
        }

        let tasks = self.tasks.read().await;
        let (total, _, _, completed, failed) = count_by_status(&tasks);

        let stats = TaskCompletionStats::new(total, completed, failed);

        let failed_tasks: Vec<FailedTaskInfo> = tasks
            .values()
            .filter(|task_info| matches!(task_info.status, TaskStatus::Failed))
            .map(|task_info| FailedTaskInfo {
                id: task_info.task.id.clone(),
                name: get_task_name(task_info).to_string(),
                error: task_info.error().cloned(),
            })
            .collect();

        let event = TaskExecutionNotificationEvent::tasks_complete(stats, failed_tasks);
        self.try_send_notification(event, "tasks complete");
        // Wait for the notification to be recieved and displayed before clearing the tasks
        sleep(Duration::from_millis(COMPLETION_NOTIFICATION_DELAY_MS)).await;
    }
}
