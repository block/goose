//! Request types mapped from a2a.proto *Request messages.

use serde::{Deserialize, Serialize};

use super::config::{PushNotificationConfig, SendMessageConfiguration};
use super::core::Message;

/// Send message request (proto `SendMessageRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<SendMessageConfiguration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Get task request (proto `GetTaskRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskRequest {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
}

/// List tasks request (proto `ListTasksRequest`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<super::core::TaskState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_timestamp_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_artifacts: Option<bool>,
}

/// Cancel task request (proto `CancelTaskRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskRequest {
    pub id: String,
}

/// Subscribe to task request (proto `SubscribeToTaskRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeToTaskRequest {
    pub id: String,
}

/// Create push notification config request (proto `CreateTaskPushNotificationConfigRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskPushNotificationConfigRequest {
    pub task_id: String,
    pub config_id: String,
    pub config: PushNotificationConfig,
}

/// Get push notification config request (proto `GetTaskPushNotificationConfigRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskPushNotificationConfigRequest {
    pub task_id: String,
    pub id: String,
}

/// Delete push notification config request (proto `DeleteTaskPushNotificationConfigRequest`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskPushNotificationConfigRequest {
    pub task_id: String,
    pub id: String,
}

/// List push notification configs request (proto `ListTaskPushNotificationConfigRequest`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPushNotificationConfigRequest {
    pub task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// Get extended agent card request (proto `GetExtendedAgentCardRequest`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetExtendedAgentCardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::{Part, Role};

    #[test]
    fn test_send_message_request_serde() {
        let request = SendMessageRequest {
            message: Message {
                message_id: "msg-1".to_string(),
                context_id: None,
                task_id: None,
                role: Role::User,
                parts: vec![Part::text("Hello")],
                metadata: None,
                extensions: vec![],
                reference_task_ids: vec![],
            },
            configuration: Some(SendMessageConfiguration {
                blocking: true,
                ..Default::default()
            }),
            metadata: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["message"]["role"], "ROLE_USER");
        assert_eq!(json["configuration"]["blocking"], true);
    }

    #[test]
    fn test_list_tasks_request_defaults() {
        let request = ListTasksRequest::default();
        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("contextId").is_none());
        assert!(json.get("pageSize").is_none());
    }
}
