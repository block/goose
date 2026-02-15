//! Maps goosed MessageEvent → ACP SSE events.
//!
//! A single goosed MessageEvent may expand to multiple ACP events.
//! For example, a Message event becomes message.created + N×message.part + message.completed.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::message::{goose_message_to_acp, AcpMessage, AcpMessagePart};
use super::types::{AcpError, AcpRun, AcpRunStatus};

/// ACP event types per v0.2.0 spec.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum AcpEventType {
    #[serde(rename = "message.created")]
    MessageCreated,
    #[serde(rename = "message.part")]
    MessagePart,
    #[serde(rename = "message.completed")]
    MessageCompleted,
    #[serde(rename = "run.created")]
    RunCreated,
    #[serde(rename = "run.in-progress")]
    RunInProgress,
    #[serde(rename = "run.awaiting")]
    RunAwaiting,
    #[serde(rename = "run.completed")]
    RunCompleted,
    #[serde(rename = "run.cancelled")]
    RunCancelled,
    #[serde(rename = "run.failed")]
    RunFailed,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "generic")]
    Generic,
}

impl AcpEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MessageCreated => "message.created",
            Self::MessagePart => "message.part",
            Self::MessageCompleted => "message.completed",
            Self::RunCreated => "run.created",
            Self::RunInProgress => "run.in-progress",
            Self::RunAwaiting => "run.awaiting",
            Self::RunCompleted => "run.completed",
            Self::RunCancelled => "run.cancelled",
            Self::RunFailed => "run.failed",
            Self::Error => "error",
            Self::Generic => "generic",
        }
    }
}

/// An ACP SSE event.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcpEvent {
    #[serde(rename = "type")]
    pub event_type: AcpEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<AcpRun>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AcpMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<AcpMessagePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AcpError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl AcpEvent {
    pub fn run_created(run: &AcpRun) -> Self {
        AcpEvent {
            event_type: AcpEventType::RunCreated,
            run: Some(run.clone()),
            message: None,
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn run_in_progress(run: &AcpRun) -> Self {
        AcpEvent {
            event_type: AcpEventType::RunInProgress,
            run: Some(run.clone()),
            message: None,
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn run_completed(run: &AcpRun) -> Self {
        AcpEvent {
            event_type: AcpEventType::RunCompleted,
            run: Some(run.clone()),
            message: None,
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn run_failed(run: &AcpRun) -> Self {
        AcpEvent {
            event_type: AcpEventType::RunFailed,
            run: Some(run.clone()),
            message: None,
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn run_cancelled(run: &AcpRun) -> Self {
        AcpEvent {
            event_type: AcpEventType::RunCancelled,
            run: Some(run.clone()),
            message: None,
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn run_awaiting(run: &AcpRun) -> Self {
        AcpEvent {
            event_type: AcpEventType::RunAwaiting,
            run: Some(run.clone()),
            message: None,
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn message_created(message: &AcpMessage) -> Self {
        AcpEvent {
            event_type: AcpEventType::MessageCreated,
            run: None,
            message: Some(message.clone()),
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn message_part(part: &AcpMessagePart) -> Self {
        AcpEvent {
            event_type: AcpEventType::MessagePart,
            run: None,
            message: None,
            part: Some(part.clone()),
            error: None,
            data: None,
        }
    }

    pub fn message_completed(message: &AcpMessage) -> Self {
        AcpEvent {
            event_type: AcpEventType::MessageCompleted,
            run: None,
            message: Some(message.clone()),
            part: None,
            error: None,
            data: None,
        }
    }

    pub fn error(error: AcpError) -> Self {
        AcpEvent {
            event_type: AcpEventType::Error,
            run: None,
            message: None,
            part: None,
            error: Some(error),
            data: None,
        }
    }

    pub fn generic(data: serde_json::Value) -> Self {
        AcpEvent {
            event_type: AcpEventType::Generic,
            run: None,
            message: None,
            part: None,
            error: None,
            data: Some(data),
        }
    }
}

/// Context needed to generate ACP events from goosed events.
pub struct AcpEventContext {
    pub run_id: String,
    pub agent_name: String,
    pub session_id: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

impl AcpEventContext {
    fn snapshot(&self, status: AcpRunStatus) -> AcpRun {
        AcpRun {
            run_id: self.run_id.clone(),
            agent_name: self.agent_name.clone(),
            status,
            session_id: self.session_id.clone(),
            output: Vec::new(),
            await_request: None,
            error: None,
            created_at: self.created_at,
            finished_at: None,
            metadata: None,
        }
    }
}

/// Convert a goosed MessageEvent into zero or more ACP events.
///
/// This is the central adapter: it takes the goosed-internal event model
/// and produces the ACP-standard event stream.
pub fn goosed_events_to_acp(
    event_type: &str,
    event_data: &serde_json::Value,
    ctx: &AcpEventContext,
) -> Vec<AcpEvent> {
    match event_type {
        "Message" => convert_message_event(event_data, ctx),
        "Error" => convert_error_event(event_data, ctx),
        "Finish" => convert_finish_event(event_data, ctx),
        "ModelChange"
        | "RoutingDecision"
        | "PlanProposal"
        | "Notification"
        | "UpdateConversation"
        | "ToolAvailabilityChange" => {
            vec![AcpEvent::generic(serde_json::json!({
                "goose_event_type": event_type,
                "data": event_data,
            }))]
        }
        _ => Vec::new(),
    }
}

fn convert_message_event(data: &serde_json::Value, _ctx: &AcpEventContext) -> Vec<AcpEvent> {
    let Some(msg_value) = data.get("message") else {
        return Vec::new();
    };

    let Ok(goose_msg) =
        serde_json::from_value::<crate::conversation::message::Message>(msg_value.clone())
    else {
        return Vec::new();
    };

    let acp_msg = goose_message_to_acp(&goose_msg);
    let mut events = Vec::with_capacity(acp_msg.parts.len() + 2);

    events.push(AcpEvent::message_created(&acp_msg));
    for part in &acp_msg.parts {
        events.push(AcpEvent::message_part(part));
    }
    events.push(AcpEvent::message_completed(&acp_msg));

    events
}

fn convert_error_event(data: &serde_json::Value, ctx: &AcpEventContext) -> Vec<AcpEvent> {
    let error_msg = data
        .get("error")
        .and_then(|e| e.as_str())
        .unwrap_or("Unknown error")
        .to_string();

    let mut run = ctx.snapshot(AcpRunStatus::Failed);
    run.error = Some(AcpError {
        code: "agent_error".to_string(),
        message: error_msg.clone(),
        data: None,
    });
    run.finished_at = Some(Utc::now());

    vec![
        AcpEvent::error(AcpError {
            code: "agent_error".to_string(),
            message: error_msg,
            data: None,
        }),
        AcpEvent::run_failed(&run),
    ]
}

fn convert_finish_event(data: &serde_json::Value, ctx: &AcpEventContext) -> Vec<AcpEvent> {
    let reason = data
        .get("reason")
        .and_then(|r| r.as_str())
        .unwrap_or("end_turn");

    let mut run = ctx.snapshot(AcpRunStatus::Completed);
    run.finished_at = Some(Utc::now());

    match reason {
        "cancelled" => {
            run.status = AcpRunStatus::Cancelled;
            vec![AcpEvent::run_cancelled(&run)]
        }
        _ => vec![AcpEvent::run_completed(&run)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> AcpEventContext {
        AcpEventContext {
            run_id: "run_123".to_string(),
            agent_name: "goose".to_string(),
            session_id: Some("sess_456".to_string()),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_message_event_produces_created_parts_completed() {
        let event_data = serde_json::json!({
            "message": {
                "role": "assistant",
                "created": 1234567890,
                "content": [
                    { "type": "text", "text": "Hello!" },
                    { "type": "text", "text": "How can I help?" }
                ],
                "metadata": { "userVisible": true, "agentVisible": true }
            },
            "tokenState": {
                "inputTokens": 10, "outputTokens": 5, "totalTokens": 15,
                "accumulatedInputTokens": 10, "accumulatedOutputTokens": 5,
                "accumulatedTotalTokens": 15
            }
        });

        let ctx = test_ctx();
        let events = goosed_events_to_acp("Message", &event_data, &ctx);

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].event_type, AcpEventType::MessageCreated);
        assert_eq!(events[1].event_type, AcpEventType::MessagePart);
        assert_eq!(events[2].event_type, AcpEventType::MessagePart);
        assert_eq!(events[3].event_type, AcpEventType::MessageCompleted);

        assert_eq!(
            events[1].part.as_ref().unwrap().content.as_deref(),
            Some("Hello!")
        );
        assert_eq!(
            events[2].part.as_ref().unwrap().content.as_deref(),
            Some("How can I help?")
        );
    }

    #[test]
    fn test_error_event() {
        let event_data = serde_json::json!({ "error": "Provider timeout" });
        let ctx = test_ctx();
        let events = goosed_events_to_acp("Error", &event_data, &ctx);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, AcpEventType::Error);
        assert_eq!(
            events[0].error.as_ref().unwrap().message,
            "Provider timeout"
        );
        assert_eq!(events[1].event_type, AcpEventType::RunFailed);
        assert_eq!(events[1].run.as_ref().unwrap().status, AcpRunStatus::Failed);
    }

    #[test]
    fn test_finish_event_completed() {
        let event_data = serde_json::json!({ "reason": "end_turn", "tokenState": {} });
        let ctx = test_ctx();
        let events = goosed_events_to_acp("Finish", &event_data, &ctx);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AcpEventType::RunCompleted);
    }

    #[test]
    fn test_finish_event_cancelled() {
        let event_data = serde_json::json!({ "reason": "cancelled", "tokenState": {} });
        let ctx = test_ctx();
        let events = goosed_events_to_acp("Finish", &event_data, &ctx);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, AcpEventType::RunCancelled);
    }

    #[test]
    fn test_goose_extension_events_become_generic() {
        let ctx = test_ctx();

        for event_type in &[
            "ModelChange",
            "RoutingDecision",
            "PlanProposal",
            "Notification",
        ] {
            let events =
                goosed_events_to_acp(event_type, &serde_json::json!({"some": "data"}), &ctx);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].event_type, AcpEventType::Generic);
        }
    }

    #[test]
    fn test_unknown_event_produces_nothing() {
        let ctx = test_ctx();
        let events = goosed_events_to_acp("SomeUnknownEvent", &serde_json::json!({}), &ctx);
        assert!(events.is_empty());
    }
}
