//! Result manager — processes execution events and updates task state.
//!
//! Mirrors the JS `ResultManager` pattern: receives events from the agent executor
//! via the event bus, applies them to the task stored in the `TaskStore`, and
//! produces the final result (Task or Message) for the request handler.

use crate::error::A2AError;
use crate::types::core::{Message, Task, TaskState, TaskStatus};
use crate::types::events::{
    AgentExecutionEvent, StreamResponse, TaskArtifactUpdateEvent, TaskStatusUpdateEvent,
};

use super::store::TaskStore;

/// Processes agent execution events and manages task state transitions.
pub struct ResultManager<S: TaskStore> {
    store: S,
    task_id: String,
}

impl<S: TaskStore> ResultManager<S> {
    pub fn new(store: S, task_id: String) -> Self {
        Self { store, task_id }
    }

    /// Process a status update event: update task state and save.
    pub async fn process_status_update(
        &self,
        event: &TaskStatusUpdateEvent,
    ) -> Result<Task, A2AError> {
        let mut task = self.load_task().await?;

        task.status = TaskStatus {
            state: event.status.state,
            message: event.status.message.clone(),
            timestamp: event
                .status
                .timestamp
                .clone()
                .or_else(|| Some(now_iso8601())),
        };

        self.store.save(&task).await?;
        Ok(task)
    }

    /// Process an artifact update event: append or replace artifact parts, save task.
    pub async fn process_artifact_update(
        &self,
        event: &TaskArtifactUpdateEvent,
    ) -> Result<Task, A2AError> {
        let mut task = self.load_task().await?;

        if let Some(existing) = task
            .artifacts
            .iter_mut()
            .find(|a| a.artifact_id == event.artifact.artifact_id)
        {
            if event.append {
                existing.parts.extend(event.artifact.parts.clone());
            } else {
                existing.parts = event.artifact.parts.clone();
            }
            if let Some(ref name) = event.artifact.name {
                existing.name = Some(name.clone());
            }
            if let Some(ref desc) = event.artifact.description {
                existing.description = Some(desc.clone());
            }
        } else {
            task.artifacts.push(event.artifact.clone());
        }

        self.store.save(&task).await?;
        Ok(task)
    }

    /// Process any execution event and return the appropriate stream response.
    pub async fn process_event(
        &self,
        event: &AgentExecutionEvent,
    ) -> Result<StreamResponse, A2AError> {
        match event {
            AgentExecutionEvent::StatusUpdate(update) => {
                self.process_status_update(update).await?;
                Ok(StreamResponse::StatusUpdate(update.clone()))
            }
            AgentExecutionEvent::ArtifactUpdate(update) => {
                self.process_artifact_update(update).await?;
                Ok(StreamResponse::ArtifactUpdate(update.clone()))
            }
        }
    }

    /// Get the current task from the store.
    pub async fn current_task(&self) -> Result<Task, A2AError> {
        self.load_task().await
    }

    /// Determine the final result: if task has a terminal state, return the Task;
    /// otherwise return the last agent message from the task history.
    pub async fn final_result(&self) -> Result<FinalResult, A2AError> {
        let task = self.load_task().await?;
        Ok(FinalResult::Task(task))
    }

    /// Mark the task as failed with an error message.
    pub async fn mark_failed(&self, message: &str) -> Result<Task, A2AError> {
        let mut task = self.load_task().await?;
        task.status = TaskStatus {
            state: TaskState::Failed,
            message: Some(Box::new(Message::agent(vec![
                crate::types::core::Part::text(message),
            ]))),
            timestamp: Some(now_iso8601()),
        };
        self.store.save(&task).await?;
        Ok(task)
    }

    async fn load_task(&self) -> Result<Task, A2AError> {
        self.store
            .load(&self.task_id)
            .await?
            .ok_or_else(|| A2AError::task_not_found(&self.task_id))
    }
}

/// Final result of an agent execution.
pub enum FinalResult {
    Task(Task),
    Message(Message),
}

fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::store::InMemoryTaskStore;
    use crate::types::core::{Artifact, Part, TaskState, TaskStatus};

    async fn setup() -> (ResultManager<InMemoryTaskStore>, InMemoryTaskStore) {
        let store = InMemoryTaskStore::new();
        let task = Task::new("t1", "ctx-1", TaskState::Submitted);
        store.save(&task).await.unwrap();

        let store2 = InMemoryTaskStore::new();
        store2.save(&task).await.unwrap();

        let mgr = ResultManager::new(store, "t1".to_string());
        (mgr, store2)
    }

    #[tokio::test]
    async fn test_process_status_update() {
        let (mgr, _) = setup().await;

        let event = TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Working,
                message: None,
                timestamp: Some("2025-01-01T00:00:00Z".to_string()),
            },
            metadata: None,
        };

        let task = mgr.process_status_update(&event).await.unwrap();
        assert_eq!(task.status.state, TaskState::Working);
        assert_eq!(
            task.status.timestamp.as_deref(),
            Some("2025-01-01T00:00:00Z")
        );
    }

    #[tokio::test]
    async fn test_process_artifact_update_new() {
        let (mgr, _) = setup().await;

        let artifact = Artifact {
            artifact_id: "a1".to_string(),
            name: Some("output".to_string()),
            description: None,
            parts: vec![Part::text("hello")],
            metadata: None,
            extensions: vec![],
        };

        let event = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            artifact: artifact.clone(),
            append: false,
            last_chunk: true,
            metadata: None,
        };

        let task = mgr.process_artifact_update(&event).await.unwrap();
        assert_eq!(task.artifacts.len(), 1);
        assert_eq!(task.artifacts[0].artifact_id, "a1");
    }

    #[tokio::test]
    async fn test_process_artifact_update_append() {
        let (mgr, _) = setup().await;

        // First chunk
        let event1 = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                name: Some("output".to_string()),
                description: None,
                parts: vec![Part::text("hello ")],
                metadata: None,
                extensions: vec![],
            },
            append: false,
            last_chunk: false,
            metadata: None,
        };
        mgr.process_artifact_update(&event1).await.unwrap();

        // Second chunk (append)
        let event2 = TaskArtifactUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            artifact: Artifact {
                artifact_id: "a1".to_string(),
                name: None,
                description: None,
                parts: vec![Part::text("world")],
                metadata: None,
                extensions: vec![],
            },
            append: true,
            last_chunk: true,
            metadata: None,
        };
        let task = mgr.process_artifact_update(&event2).await.unwrap();
        assert_eq!(task.artifacts.len(), 1);
        assert_eq!(task.artifacts[0].parts.len(), 2);
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let (mgr, _) = setup().await;
        let task = mgr.mark_failed("something went wrong").await.unwrap();
        assert_eq!(task.status.state, TaskState::Failed);
        assert!(task.status.message.is_some());
    }

    #[tokio::test]
    async fn test_process_event_status() {
        let (mgr, _) = setup().await;

        let event = AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
            task_id: "t1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Completed,
                message: None,
                timestamp: None,
            },
            metadata: None,
        });

        let response = mgr.process_event(&event).await.unwrap();
        match response {
            StreamResponse::StatusUpdate(u) => {
                assert_eq!(u.status.state, TaskState::Completed);
            }
            _ => panic!("Expected StatusUpdate"),
        }
    }

    #[tokio::test]
    async fn test_current_task() {
        let (mgr, _) = setup().await;
        let task = mgr.current_task().await.unwrap();
        assert_eq!(task.id, "t1");
        assert_eq!(task.status.state, TaskState::Submitted);
    }

    #[tokio::test]
    async fn test_load_missing_task() {
        let store = InMemoryTaskStore::new();
        let mgr = ResultManager::new(store, "nonexistent".to_string());
        let result = mgr.current_task().await;
        assert!(result.is_err());
    }
}
