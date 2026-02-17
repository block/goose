//! Response types mapped from a2a.proto *Response messages.

use serde::{Deserialize, Serialize};

use super::config::TaskPushNotificationConfig;
use super::core::{Message, Task};

/// Send message response (proto `SendMessageResponse` oneof).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SendMessageResponse {
    Task(Task),
    Message(Message),
}

/// List tasks response (proto `ListTasksResponse`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksResponse {
    pub tasks: Vec<Task>,
    pub next_page_token: String,
    pub page_size: i32,
    pub total_size: i32,
}

/// List push notification configs response (proto `ListTaskPushNotificationConfigResponse`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPushNotificationConfigResponse {
    pub configs: Vec<TaskPushNotificationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::core::TaskState;

    #[test]
    fn test_send_message_response_task() {
        let task = Task::new("task-1", "ctx-1", TaskState::Completed);
        let response = SendMessageResponse::Task(task);
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["id"], "task-1");
        assert_eq!(json["status"]["state"], "completed");
    }

    #[test]
    fn test_send_message_response_message() {
        let msg = Message::agent(vec![crate::types::core::Part::text("Hi")]);
        let response = SendMessageResponse::Message(msg);
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["role"], "agent");
    }

    #[test]
    fn test_list_tasks_response_serde() {
        let response = ListTasksResponse {
            tasks: vec![Task::new("t1", "c1", TaskState::Submitted)],
            next_page_token: "".to_string(),
            page_size: 50,
            total_size: 1,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["tasks"][0]["id"], "t1");
        assert_eq!(json["pageSize"], 50);
        assert_eq!(json["totalSize"], 1);
    }
}
