use agent_client_protocol::schema::{
    Meta, SessionUpdate, ToolCallId, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use rmcp::model::{LoggingMessageNotificationParam, ProgressNotificationParam, ServerNotification};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ToolNotification {
    Message {
        params: LoggingMessageNotificationParam,
    },
    Progress {
        params: ProgressNotificationParam,
    },
}

pub(super) fn tool_notification_update(
    tool_call_id: impl Into<ToolCallId>,
    notification: ServerNotification,
) -> Option<SessionUpdate> {
    let tool_notification = match notification {
        ServerNotification::LoggingMessageNotification(notification) => ToolNotification::Message {
            params: notification.params,
        },
        ServerNotification::ProgressNotification(notification) => ToolNotification::Progress {
            params: notification.params,
        },
        _ => return None,
    };

    let mut meta = Meta::new();
    meta.insert(
        "toolNotification".to_string(),
        serde_json::to_value(tool_notification).ok()?,
    );

    Some(SessionUpdate::ToolCallUpdate(
        ToolCallUpdate::new(
            tool_call_id,
            ToolCallUpdateFields::new().status(ToolCallStatus::InProgress),
        )
        .meta(meta),
    ))
}

#[cfg(test)]
mod tests {
    use super::tool_notification_update;
    use rmcp::model::{
        CancelledNotificationParam, LoggingLevel, LoggingMessageNotificationParam, Notification,
        NumberOrString, ProgressNotificationParam, ProgressToken, ServerNotification,
    };
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn maps_logging_message_notification_to_tool_update_meta() {
        let notification = ServerNotification::LoggingMessageNotification(Notification::new(
            LoggingMessageNotificationParam::new(
                LoggingLevel::Info,
                json!({
                    "type": "subagent_tool_request",
                    "subagent_id": "session_1",
                    "tool_call": {
                        "name": "developer__shell"
                    }
                }),
            )
            .with_logger("subagent:session_1"),
        ));

        let update = tool_notification_update("tool_1", notification).expect("expected update");
        let value = serde_json::to_value(update).expect("update should serialize");

        assert_eq!(value["sessionUpdate"], "tool_call_update");
        assert_eq!(value["toolCallId"], "tool_1");
        assert_eq!(value["status"], "in_progress");
        assert_eq!(value["_meta"]["toolNotification"]["type"], "message");
        assert_eq!(
            value["_meta"]["toolNotification"]["params"]["level"],
            "info"
        );
        assert_eq!(
            value["_meta"]["toolNotification"]["params"]["logger"],
            "subagent:session_1"
        );
        assert_eq!(
            value["_meta"]["toolNotification"]["params"]["data"]["tool_call"]["name"],
            "developer__shell"
        );
    }

    #[test]
    fn maps_progress_notification_to_tool_update_meta() {
        let notification = ServerNotification::ProgressNotification(Notification::new(
            ProgressNotificationParam::new(
                ProgressToken(NumberOrString::String(Arc::from("scan-repo"))),
                3.0,
            )
            .with_total(10.0)
            .with_message("Scanned 3 of 10 directories"),
        ));

        let update = tool_notification_update("tool_1", notification).expect("expected update");
        let value = serde_json::to_value(update).expect("update should serialize");

        assert_eq!(value["sessionUpdate"], "tool_call_update");
        assert_eq!(value["toolCallId"], "tool_1");
        assert_eq!(value["status"], "in_progress");
        assert_eq!(value["_meta"]["toolNotification"]["type"], "progress");
        assert_eq!(
            value["_meta"]["toolNotification"]["params"]["progressToken"],
            "scan-repo"
        );
        assert_eq!(
            value["_meta"]["toolNotification"]["params"]["progress"],
            3.0
        );
        assert_eq!(value["_meta"]["toolNotification"]["params"]["total"], 10.0);
        assert_eq!(
            value["_meta"]["toolNotification"]["params"]["message"],
            "Scanned 3 of 10 directories"
        );
    }

    #[test]
    fn ignores_non_tool_live_notification_variants() {
        let notification = ServerNotification::CancelledNotification(Notification::new(
            CancelledNotificationParam {
                request_id: NumberOrString::String(Arc::from("request_1")),
                reason: None,
            },
        ));

        assert!(tool_notification_update("tool_1", notification).is_none());
    }
}
