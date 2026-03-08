//! Configuration types mapped from a2a.proto.

use serde::{Deserialize, Serialize};

/// Send message request configuration (proto `SendMessageConfiguration`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageConfiguration {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_output_modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notification_config: Option<PushNotificationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    #[serde(default)]
    pub blocking: bool,
}

/// Push notification configuration (proto `PushNotificationConfig`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<AuthenticationInfo>,
}

/// Authentication details for push notifications (proto `AuthenticationInfo`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationInfo {
    pub scheme: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

/// Task push notification config wrapper (proto `TaskPushNotificationConfig`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPushNotificationConfig {
    pub task_id: String,
    pub config: PushNotificationConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_config_defaults() {
        let config = SendMessageConfiguration::default();
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["blocking"], false);
        assert!(json.get("historyLength").is_none());
    }

    #[test]
    fn test_push_notification_config_serde() {
        let config = PushNotificationConfig {
            id: Some("pn-1".to_string()),
            url: "https://example.com/webhook".to_string(),
            token: Some("tok123".to_string()),
            authentication: Some(AuthenticationInfo {
                scheme: "Bearer".to_string(),
                credentials: Some("secret-token".to_string()),
            }),
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["url"], "https://example.com/webhook");
        assert_eq!(json["authentication"]["scheme"], "Bearer");
    }
}
