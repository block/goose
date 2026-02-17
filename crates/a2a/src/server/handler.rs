//! A2A request handler trait defining the full server interface.

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::error::A2AError;
use crate::types::agent_card::AgentCard;
use crate::types::config::TaskPushNotificationConfig;
use crate::types::core::Task;
use crate::types::events::StreamResponse;
use crate::types::requests::*;
use crate::types::responses::*;

/// Full A2A server handler interface mapped from the A2AService proto RPCs.
#[async_trait]
pub trait A2ARequestHandler: Send + Sync {
    async fn get_agent_card(&self) -> Result<AgentCard, A2AError>;

    async fn get_authenticated_extended_card(
        &self,
        request: GetExtendedAgentCardRequest,
    ) -> Result<AgentCard, A2AError>;

    async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<SendMessageResponse, A2AError>;

    fn send_message_stream(
        &self,
        request: SendMessageRequest,
    ) -> BoxStream<'_, Result<StreamResponse, A2AError>>;

    async fn get_task(&self, request: GetTaskRequest) -> Result<Task, A2AError>;

    async fn list_tasks(&self, request: ListTasksRequest) -> Result<ListTasksResponse, A2AError>;

    async fn cancel_task(&self, request: CancelTaskRequest) -> Result<Task, A2AError>;

    fn subscribe_to_task(
        &self,
        request: SubscribeToTaskRequest,
    ) -> BoxStream<'_, Result<StreamResponse, A2AError>>;

    async fn set_push_notification_config(
        &self,
        request: CreateTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError>;

    async fn get_push_notification_config(
        &self,
        request: GetTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError>;

    async fn list_push_notification_configs(
        &self,
        request: ListTaskPushNotificationConfigRequest,
    ) -> Result<ListTaskPushNotificationConfigResponse, A2AError>;

    async fn delete_push_notification_config(
        &self,
        request: DeleteTaskPushNotificationConfigRequest,
    ) -> Result<(), A2AError>;
}
