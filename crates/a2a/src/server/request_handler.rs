//! Default implementation of the A2A request handler.
//!
//! Orchestrates request processing: creates tasks, dispatches to the agent executor,
//! processes events via the result manager, and returns responses.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use crate::error::A2AError;
use crate::types::agent_card::AgentCard;
use crate::types::config::TaskPushNotificationConfig;
use crate::types::core::{Message, Task, TaskState, TaskStatus};
use crate::types::events::{AgentExecutionEvent, StreamResponse};
use crate::types::requests::{
    CancelTaskRequest, CreateTaskPushNotificationConfigRequest,
    DeleteTaskPushNotificationConfigRequest, GetTaskPushNotificationConfigRequest, GetTaskRequest,
    ListTaskPushNotificationConfigRequest, ListTasksRequest, SendMessageRequest,
};
use crate::types::responses::{
    ListTaskPushNotificationConfigResponse, ListTasksResponse, SendMessageResponse,
};

use super::context::RequestContext;
use super::event_bus::EventBusManager;
use super::executor::AgentExecutor;
use super::push_notification::{
    InMemoryPushNotificationStore, PushNotificationSender, PushNotificationStore,
};
use super::result_manager::ResultManager;
use super::store::TaskStore;

/// Default request handler that orchestrates A2A protocol operations.
pub struct DefaultRequestHandler<
    S: TaskStore,
    E: AgentExecutor,
    P: PushNotificationStore = InMemoryPushNotificationStore,
> {
    agent_card: AgentCard,
    store: S,
    executor: Arc<E>,
    event_bus_manager: EventBusManager,
    push_store: Option<P>,
    #[allow(dead_code)]
    push_sender: Option<PushNotificationSender>,
}

impl<S: TaskStore + Clone + 'static, E: AgentExecutor + 'static> DefaultRequestHandler<S, E> {
    pub fn new(agent_card: AgentCard, store: S, executor: E) -> Self {
        Self {
            agent_card,
            store,
            executor: Arc::new(executor),
            event_bus_manager: EventBusManager::new(),
            push_store: None,
            push_sender: None,
        }
    }
}

impl<
        S: TaskStore + Clone + 'static,
        E: AgentExecutor + 'static,
        P: PushNotificationStore + Clone + 'static,
    > DefaultRequestHandler<S, E, P>
{
    pub fn with_push_notifications(
        agent_card: AgentCard,
        store: S,
        executor: E,
        push_store: P,
    ) -> Self {
        Self {
            agent_card,
            store,
            executor: Arc::new(executor),
            event_bus_manager: EventBusManager::new(),
            push_store: Some(push_store),
            push_sender: Some(PushNotificationSender::new()),
        }
    }

    pub fn agent_card(&self) -> &AgentCard {
        &self.agent_card
    }

    /// Handle a send message request (blocking).
    pub async fn send_message(
        &self,
        request: &SendMessageRequest,
    ) -> Result<SendMessageResponse, A2AError> {
        let message = &request.message;
        let task = self.get_or_create_task(message).await?;
        let task_id = task.id.clone();

        // Set task to working
        let mut working_task = task.clone();
        working_task.status = TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        self.store.save(&working_task).await?;

        let context = RequestContext {
            user_message: message.clone(),
            task_id: task_id.clone(),
            context_id: task.context_id.clone(),
            task: Some(working_task),
            reference_tasks: vec![],
            requested_extensions: vec![],
        };

        // Create event channel and execute
        let (tx, mut rx) = mpsc::channel::<AgentExecutionEvent>(256);
        let executor = Arc::clone(&self.executor);

        let exec_handle = tokio::spawn(async move { executor.execute(context, tx).await });

        // Process all events
        let result_manager = ResultManager::new(self.store.clone(), task_id);

        while let Some(event) = rx.recv().await {
            result_manager.process_event(&event).await?;
        }

        // Wait for executor to finish
        match exec_handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                result_manager.mark_failed(&e.to_string()).await?;
            }
            Err(e) => {
                result_manager
                    .mark_failed(&format!("executor panicked: {e}"))
                    .await?;
            }
        }

        let final_task = result_manager.current_task().await?;
        Ok(SendMessageResponse::Task(final_task))
    }

    /// Handle a streaming send message request.
    pub async fn send_message_stream(
        &self,
        request: &SendMessageRequest,
    ) -> Result<impl Stream<Item = Result<StreamResponse, A2AError>>, A2AError> {
        let message = &request.message;
        let task = self.get_or_create_task(message).await?;
        let task_id = task.id.clone();

        let mut working_task = task.clone();
        working_task.status = TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        self.store.save(&working_task).await?;

        let context = RequestContext {
            user_message: message.clone(),
            task_id: task_id.clone(),
            context_id: task.context_id.clone(),
            task: Some(working_task),
            reference_tasks: vec![],
            requested_extensions: vec![],
        };

        let (exec_tx, mut exec_rx) = mpsc::channel::<AgentExecutionEvent>(256);
        let (stream_tx, stream_rx) = mpsc::channel::<Result<StreamResponse, A2AError>>(256);

        let executor = Arc::clone(&self.executor);
        let store = self.store.clone();

        tokio::spawn(async move {
            let result_manager = ResultManager::new(store, task_id);

            let exec_handle = tokio::spawn(async move { executor.execute(context, exec_tx).await });

            while let Some(event) = exec_rx.recv().await {
                match result_manager.process_event(&event).await {
                    Ok(response) => {
                        if stream_tx.send(Ok(response)).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = stream_tx.send(Err(e)).await;
                        break;
                    }
                }
            }

            match exec_handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    if let Ok(task) = result_manager.mark_failed(&e.to_string()).await {
                        let _ = stream_tx.send(Ok(StreamResponse::Task(task))).await;
                    }
                }
                Err(e) => {
                    if let Ok(task) = result_manager
                        .mark_failed(&format!("executor panicked: {e}"))
                        .await
                    {
                        let _ = stream_tx.send(Ok(StreamResponse::Task(task))).await;
                    }
                }
            }
        });

        Ok(ReceiverStream::new(stream_rx))
    }

    /// Get a task by ID.
    pub async fn get_task(&self, request: &GetTaskRequest) -> Result<Task, A2AError> {
        self.store
            .load(&request.id)
            .await?
            .ok_or_else(|| A2AError::task_not_found(&request.id))
    }

    /// List tasks with optional filtering.
    pub async fn list_tasks(
        &self,
        request: &ListTasksRequest,
    ) -> Result<ListTasksResponse, A2AError> {
        self.store.list(request).await
    }

    /// Cancel a task.
    pub async fn cancel_task(&self, request: &CancelTaskRequest) -> Result<Task, A2AError> {
        let task = self
            .store
            .load(&request.id)
            .await?
            .ok_or_else(|| A2AError::task_not_found(&request.id))?;

        if task.status.state.is_terminal() {
            return Err(A2AError::task_not_cancelable(&request.id));
        }

        let executor = Arc::clone(&self.executor);
        let (tx, _rx) = mpsc::channel(1);
        executor.cancel(&task.id, tx).await?;

        let mut cancelled_task = task;
        cancelled_task.status = TaskStatus {
            state: TaskState::Canceled,
            message: None,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        };
        self.store.save(&cancelled_task).await?;

        self.event_bus_manager.remove(&cancelled_task.id).await;

        Ok(cancelled_task)
    }

    /// Get the agent card.
    pub fn get_agent_card(&self) -> AgentCard {
        self.agent_card.clone()
    }

    /// Push notification config management.
    pub async fn set_push_notification_config(
        &self,
        request: &CreateTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        let push_store = self
            .push_store
            .as_ref()
            .ok_or(A2AError::PushNotificationNotSupported)?;
        push_store
            .save(&request.task_id, request.config.clone())
            .await
    }

    pub async fn get_push_notification_config(
        &self,
        request: &GetTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        let push_store = self
            .push_store
            .as_ref()
            .ok_or(A2AError::PushNotificationNotSupported)?;
        push_store
            .load(&request.task_id, &request.id)
            .await?
            .ok_or_else(|| {
                A2AError::task_not_found(format!("push config {} not found", request.id))
            })
    }

    pub async fn list_push_notification_configs(
        &self,
        request: &ListTaskPushNotificationConfigRequest,
    ) -> Result<ListTaskPushNotificationConfigResponse, A2AError> {
        let push_store = self
            .push_store
            .as_ref()
            .ok_or(A2AError::PushNotificationNotSupported)?;
        let configs = push_store.list(&request.task_id).await?;
        Ok(ListTaskPushNotificationConfigResponse {
            configs,
            next_page_token: None,
        })
    }

    pub async fn delete_push_notification_config(
        &self,
        request: &DeleteTaskPushNotificationConfigRequest,
    ) -> Result<(), A2AError> {
        let push_store = self
            .push_store
            .as_ref()
            .ok_or(A2AError::PushNotificationNotSupported)?;
        push_store.delete(&request.task_id, &request.id).await?;
        Ok(())
    }

    async fn get_or_create_task(&self, message: &Message) -> Result<Task, A2AError> {
        if let Some(ref task_id) = message.task_id {
            if let Some(task) = self.store.load(task_id).await? {
                return Ok(task);
            }
        }

        let context_id = message
            .context_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let task = Task::new(
            uuid::Uuid::new_v4().to_string(),
            &context_id,
            TaskState::Submitted,
        );
        self.store.save(&task).await?;
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::store::InMemoryTaskStore;
    use crate::types::core::{Artifact, Part, Role};
    use crate::types::events::{TaskArtifactUpdateEvent, TaskStatusUpdateEvent};

    struct EchoExecutor;

    #[async_trait::async_trait]
    impl AgentExecutor for EchoExecutor {
        async fn execute(
            &self,
            context: RequestContext,
            tx: mpsc::Sender<AgentExecutionEvent>,
        ) -> Result<(), A2AError> {
            let artifact = Artifact {
                artifact_id: uuid::Uuid::new_v4().to_string(),
                name: Some("response".to_string()),
                description: None,
                parts: context.user_message.parts.clone(),
                metadata: None,
                extensions: vec![],
            };

            let _ = tx
                .send(AgentExecutionEvent::ArtifactUpdate(
                    TaskArtifactUpdateEvent {
                        task_id: context.task_id.clone(),
                        context_id: context.context_id.clone(),
                        artifact,
                        append: false,
                        last_chunk: true,
                        metadata: None,
                    },
                ))
                .await;

            let _ = tx
                .send(AgentExecutionEvent::StatusUpdate(TaskStatusUpdateEvent {
                    task_id: context.task_id.clone(),
                    context_id: context.context_id.clone(),
                    status: TaskStatus {
                        state: TaskState::Completed,
                        message: None,
                        timestamp: Some(chrono::Utc::now().to_rfc3339()),
                    },
                    metadata: None,
                }))
                .await;

            Ok(())
        }

        async fn cancel(
            &self,
            _task_id: &str,
            _tx: mpsc::Sender<AgentExecutionEvent>,
        ) -> Result<(), A2AError> {
            Ok(())
        }
    }

    fn test_agent_card() -> AgentCard {
        AgentCard {
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            ..Default::default()
        }
    }

    fn send_request(text: &str) -> SendMessageRequest {
        SendMessageRequest {
            message: Message {
                message_id: uuid::Uuid::new_v4().to_string(),
                role: Role::User,
                parts: vec![Part::text(text)],
                context_id: Some("ctx-test".to_string()),
                task_id: None,
                metadata: None,
                extensions: vec![],
                reference_task_ids: vec![],
            },
            configuration: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn test_send_message_creates_task() {
        let handler =
            DefaultRequestHandler::new(test_agent_card(), InMemoryTaskStore::new(), EchoExecutor);

        let request = send_request("Hello!");
        let response = handler.send_message(&request).await.unwrap();

        match response {
            SendMessageResponse::Task(task) => {
                assert_eq!(task.status.state, TaskState::Completed);
                assert_eq!(task.artifacts.len(), 1);
                assert_eq!(task.context_id, "ctx-test");
            }
            SendMessageResponse::Message(_) => panic!("Expected Task response"),
        }
    }

    #[tokio::test]
    async fn test_get_task_not_found() {
        let handler =
            DefaultRequestHandler::new(test_agent_card(), InMemoryTaskStore::new(), EchoExecutor);

        let request = GetTaskRequest {
            id: "nonexistent".to_string(),
            history_length: None,
        };

        let result = handler.get_task(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_agent_card() {
        let handler =
            DefaultRequestHandler::new(test_agent_card(), InMemoryTaskStore::new(), EchoExecutor);

        let card = handler.get_agent_card();
        assert_eq!(card.name, "Test Agent");
    }

    #[tokio::test]
    async fn test_send_and_get_task() {
        let handler =
            DefaultRequestHandler::new(test_agent_card(), InMemoryTaskStore::new(), EchoExecutor);

        let request = send_request("Test");
        let response = handler.send_message(&request).await.unwrap();

        let task_id = match &response {
            SendMessageResponse::Task(t) => t.id.clone(),
            _ => panic!("Expected Task"),
        };

        let get_request = GetTaskRequest {
            id: task_id,
            history_length: None,
        };

        let task = handler.get_task(&get_request).await.unwrap();
        assert_eq!(task.status.state, TaskState::Completed);
    }

    #[tokio::test]
    async fn test_cancel_completed_task_fails() {
        let handler =
            DefaultRequestHandler::new(test_agent_card(), InMemoryTaskStore::new(), EchoExecutor);

        let request = send_request("Test");
        let response = handler.send_message(&request).await.unwrap();

        let task_id = match &response {
            SendMessageResponse::Task(t) => t.id.clone(),
            _ => panic!("Expected Task"),
        };

        let cancel_request = CancelTaskRequest { id: task_id };
        let result = handler.cancel_task(&cancel_request).await;
        assert!(result.is_err());
    }

    // Push notification tests

    #[tokio::test]
    async fn test_push_notification_not_supported_without_store() {
        let handler =
            DefaultRequestHandler::new(test_agent_card(), InMemoryTaskStore::new(), EchoExecutor);

        let request = CreateTaskPushNotificationConfigRequest {
            task_id: "t1".to_string(),
            config_id: "pn-1".to_string(),
            config: crate::types::config::PushNotificationConfig {
                id: Some("pn-1".to_string()),
                url: "https://example.com/hook".to_string(),
                token: None,
                authentication: None,
            },
        };
        let result = handler.set_push_notification_config(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_push_notification_crud_with_store() {
        use crate::server::push_notification::InMemoryPushNotificationStore;

        let handler: DefaultRequestHandler<
            InMemoryTaskStore,
            EchoExecutor,
            InMemoryPushNotificationStore,
        > = DefaultRequestHandler::with_push_notifications(
            test_agent_card(),
            InMemoryTaskStore::new(),
            EchoExecutor,
            InMemoryPushNotificationStore::default(),
        );

        // Create
        let create_req = CreateTaskPushNotificationConfigRequest {
            task_id: "t1".to_string(),
            config_id: "pn-1".to_string(),
            config: crate::types::config::PushNotificationConfig {
                id: Some("pn-1".to_string()),
                url: "https://example.com/hook".to_string(),
                token: Some("my-token".to_string()),
                authentication: None,
            },
        };
        let created = handler
            .set_push_notification_config(&create_req)
            .await
            .unwrap();
        assert_eq!(created.task_id, "t1");
        assert_eq!(created.config.url, "https://example.com/hook");

        // Get
        let get_req = GetTaskPushNotificationConfigRequest {
            task_id: "t1".to_string(),
            id: "pn-1".to_string(),
        };
        let got = handler
            .get_push_notification_config(&get_req)
            .await
            .unwrap();
        assert_eq!(got.config.token, Some("my-token".to_string()));

        // List
        let list_req = ListTaskPushNotificationConfigRequest {
            task_id: "t1".to_string(),
            page_size: None,
            page_token: None,
        };
        let listed = handler
            .list_push_notification_configs(&list_req)
            .await
            .unwrap();
        assert_eq!(listed.configs.len(), 1);

        // Delete
        let delete_req = DeleteTaskPushNotificationConfigRequest {
            task_id: "t1".to_string(),
            id: "pn-1".to_string(),
        };
        handler
            .delete_push_notification_config(&delete_req)
            .await
            .unwrap();

        // Verify deleted
        let listed_after = handler
            .list_push_notification_configs(&list_req)
            .await
            .unwrap();
        assert!(listed_after.configs.is_empty());
    }
}
