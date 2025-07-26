use indoc::indoc;
use mcp_core::{ToolError, ToolResult};
use rmcp::model::{Content, Tool, ToolAnnotations};
use rmcp::object;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const TASK_TRACKER_TOOL_NAME: &str = "taskTracker";

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Todo,
    Wip,
    Done,
}

impl TaskStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "to do",
            TaskStatus::Wip => "wip",
            TaskStatus::Done => "done",
        }
    }
}

#[derive(Debug)]
pub struct TaskTracker {
    tasks: Arc<Mutex<HashMap<String, TaskStatus>>>,
}

impl TaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn tool() -> Tool {
        Tool::new(
            TASK_TRACKER_TOOL_NAME.to_string(),
            indoc! {r#"
                Use the task tracker to keep state of what you have to do, especially for large complex tasks.
                Use this when starting an activity or resuming or shifting activities
                This is an ESSENTIAL tool for breaking down your work into chunks and ensuring it is completed 
                Check the list often, and update it (one by one) as you complete tasks

                When starting out, you SHOULD plan your tasks in advance in very short description for each 
                
                use wip action when you start on one task at a time and done action when finished with it 
                
                By default (no parameters), returns a list of all tasks with their status.

                for example, 
                    user: "build me a time machine". 
                    task list: "establish a view of quantum physics", "solve causality paradoxes", "research negative energy", "test time machine" 
                
                Actions:
                - No action (default): List all tasks with their current status
                - "add": Add a new task (status will be "to do")
                - "wip": Mark a task as work in progress
                - "done": Mark a task as completed IMPORTANT:do this as soon as finished
                - "clear": Clear all tasks from the list if you are all done and ready to start fresh
                
                Task statuses:
                - "to do": Task has been added but not started
                - "wip": Task is currently being worked on
                - "done": Task has been completed
            "#}
            .to_string(),
            object!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action to perform",
                        "enum": ["add", "wip", "done", "clear"]
                    },
                    "task": {
                        "type": "string",
                        "description": "Task description (required for add, wip, done actions)"
                    }
                }
            }),
        )
        .annotate(ToolAnnotations {
            title: Some("Task Tracker".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        })
    }

    pub async fn execute(&self, arguments: Value) -> ToolResult<Vec<Content>> {
        let action = arguments.get("action").and_then(|v| v.as_str());
        let task = arguments.get("task").and_then(|v| v.as_str());

        match action {
            None => {
                // List all tasks
                let tasks = self.tasks.lock().await;
                if tasks.is_empty() {
                    Ok(vec![Content::text("No tasks tracked in current session.")])
                } else {
                    let mut task_list = Vec::new();
                    for (task, status) in tasks.iter() {
                        task_list.push(format!("- {} [{}]", task, status.as_str()));
                    }
                    Ok(vec![Content::text(format!(
                        "Current tasks:\n{}",
                        task_list.join("\n")
                    ))])
                }
            }
            Some("add") => {
                if let Some(task_str) = task {
                    let mut tasks = self.tasks.lock().await;
                    tasks.insert(task_str.to_string(), TaskStatus::Todo);
                    Ok(vec![Content::text(format!(
                        "Added task: \"{}\" [to do]",
                        task_str
                    ))])
                } else {
                    Err(ToolError::InvalidParameters(
                        "Task description required for 'add' action".to_string(),
                    ))
                }
            }
            Some("wip") => {
                if let Some(task_str) = task {
                    let mut tasks = self.tasks.lock().await;
                    if tasks.contains_key(task_str) {
                        tasks.insert(task_str.to_string(), TaskStatus::Wip);
                        Ok(vec![Content::text(format!(
                            "Marked task as work in progress: \"{}\" [wip]",
                            task_str
                        ))])
                    } else {
                        Err(ToolError::InvalidParameters(format!(
                            "Task not found: \"{}\"",
                            task_str
                        )))
                    }
                } else {
                    Err(ToolError::InvalidParameters(
                        "Task description required for 'wip' action".to_string(),
                    ))
                }
            }
            Some("done") => {
                if let Some(task_str) = task {
                    let mut tasks = self.tasks.lock().await;
                    if tasks.contains_key(task_str) {
                        tasks.insert(task_str.to_string(), TaskStatus::Done);
                        Ok(vec![Content::text(format!(
                            "Marked task as completed: \"{}\" [done]",
                            task_str
                        ))])
                    } else {
                        Err(ToolError::InvalidParameters(format!(
                            "Task not found: \"{}\"",
                            task_str
                        )))
                    }
                } else {
                    Err(ToolError::InvalidParameters(
                        "Task description required for 'done' action".to_string(),
                    ))
                }
            }
            Some("clear") => {
                let mut tasks = self.tasks.lock().await;
                let count = tasks.len();
                tasks.clear();
                Ok(vec![Content::text(format!("Cleared {} tasks.", count))])
            }
            Some(action) => Err(ToolError::InvalidParameters(format!(
                "Unknown action: \"{}\"",
                action
            ))),
        }
    }
}

impl Default for TaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_empty_task_list() {
        let tracker = TaskTracker::new();
        let result = tracker.execute(json!({})).await.unwrap();
        assert_eq!(result.len(), 1);
        if let Some(text) = result[0].as_text() {
            assert_eq!(text.text.as_str(), "No tasks tracked in current session.");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_add_task() {
        let tracker = TaskTracker::new();
        let result = tracker
            .execute(json!({
                "action": "add",
                "task": "Write unit tests"
            }))
            .await
            .unwrap();

        if let Some(text) = result[0].as_text() {
            assert_eq!(
                text.text.as_str(),
                "Added task: \"Write unit tests\" [to do]"
            );
        } else {
            panic!("Expected text content");
        }

        // Verify task was added
        let list_result = tracker.execute(json!({})).await.unwrap();
        if let Some(text) = list_result[0].as_text() {
            assert!(text.text.contains("Write unit tests [to do]"));
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_mark_task_wip() {
        let tracker = TaskTracker::new();

        // Add task first
        tracker
            .execute(json!({
                "action": "add",
                "task": "Fix bug"
            }))
            .await
            .unwrap();

        // Mark as WIP
        let result = tracker
            .execute(json!({
                "action": "wip",
                "task": "Fix bug"
            }))
            .await
            .unwrap();

        if let Some(text) = result[0].as_text() {
            assert_eq!(
                text.text.as_str(),
                "Marked task as work in progress: \"Fix bug\" [wip]"
            );
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_mark_task_done() {
        let tracker = TaskTracker::new();

        // Add task first
        tracker
            .execute(json!({
                "action": "add",
                "task": "Review PR"
            }))
            .await
            .unwrap();

        // Mark as done
        let result = tracker
            .execute(json!({
                "action": "done",
                "task": "Review PR"
            }))
            .await
            .unwrap();

        if let Some(text) = result[0].as_text() {
            assert_eq!(
                text.text.as_str(),
                "Marked task as completed: \"Review PR\" [done]"
            );
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_clear_tasks() {
        let tracker = TaskTracker::new();

        // Add some tasks
        tracker
            .execute(json!({
                "action": "add",
                "task": "Task 1"
            }))
            .await
            .unwrap();
        tracker
            .execute(json!({
                "action": "add",
                "task": "Task 2"
            }))
            .await
            .unwrap();

        // Clear all
        let result = tracker
            .execute(json!({
                "action": "clear"
            }))
            .await
            .unwrap();

        if let Some(text) = result[0].as_text() {
            assert_eq!(text.text.as_str(), "Cleared 2 tasks.");
        } else {
            panic!("Expected text content");
        }

        // Verify empty
        let list_result = tracker.execute(json!({})).await.unwrap();
        if let Some(text) = list_result[0].as_text() {
            assert_eq!(text.text.as_str(), "No tasks tracked in current session.");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_error_missing_task() {
        let tracker = TaskTracker::new();

        let result = tracker
            .execute(json!({
                "action": "add"
            }))
            .await;

        assert!(result.is_err());
        if let Err(ToolError::InvalidParameters(msg)) = result {
            assert_eq!(msg, "Task description required for 'add' action");
        }
    }

    #[tokio::test]
    async fn test_error_task_not_found() {
        let tracker = TaskTracker::new();

        let result = tracker
            .execute(json!({
                "action": "wip",
                "task": "Non-existent task"
            }))
            .await;

        assert!(result.is_err());
        if let Err(ToolError::InvalidParameters(msg)) = result {
            assert_eq!(msg, "Task not found: \"Non-existent task\"");
        }
    }
}
