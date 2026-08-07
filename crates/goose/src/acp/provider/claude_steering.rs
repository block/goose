use agent_client_protocol::schema::v1::{ContentBlock, InitializeResponse, SessionId};
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use semver::Version;
use serde::{Deserialize, Serialize};

const MINIMUM_CLAUDE_ACP_VERSION: Version = Version::new(0, 65, 0);

pub(super) fn is_supported(response: &InitializeResponse) -> bool {
    let steering_supported = response
        .meta
        .as_ref()
        .and_then(|meta| meta.get("steering"))
        .and_then(|steering| steering.get("supported"))
        .and_then(serde_json::Value::as_bool)
        == Some(true);

    steering_supported
        && response
            .agent_info
            .as_ref()
            .and_then(|info| Version::parse(&info.version).ok())
            .is_some_and(|version| version >= MINIMUM_CLAUDE_ACP_VERSION)
}

pub(super) fn delivery_confirmed(response: ClaudeSteeringResponse) -> bool {
    matches!(response, ClaudeSteeringResponse::Injected)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonRpcRequest)]
#[request(
    method = "_session/steering",
    response = ClaudeSteeringResponse
)]
#[serde(rename_all = "camelCase")]
pub(super) struct ClaudeSteeringRequest {
    session_id: SessionId,
    prompt: Vec<ContentBlock>,
    #[serde(rename = "_meta")]
    meta: serde_json::Value,
}

impl ClaudeSteeringRequest {
    pub(super) fn new(session_id: SessionId, prompt: Vec<ContentBlock>) -> Self {
        Self {
            session_id,
            prompt,
            meta: serde_json::json!({
                "steering": {
                    "idleBehavior": "promptRequired"
                }
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonRpcResponse)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub(super) enum ClaudeSteeringResponse {
    Injected,
    PromptRequired,
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{Implementation, TextContent};
    use agent_client_protocol::schema::ProtocolVersion;

    fn initialize_response(
        version: Option<&str>,
        steering_supported: Option<serde_json::Value>,
    ) -> InitializeResponse {
        let mut response = InitializeResponse::new(ProtocolVersion::LATEST);
        if let Some(version) = version {
            response = response.agent_info(Implementation::new("claude-agent-acp", version));
        }
        if let Some(supported) = steering_supported {
            let mut meta = serde_json::Map::new();
            meta.insert(
                "steering".to_string(),
                serde_json::json!({ "supported": supported }),
            );
            response = response.meta(meta);
        }
        response
    }

    #[test]
    fn detects_supported_capability_and_version() {
        for (version, capability, expected) in [
            (Some("0.65.0"), Some(serde_json::json!(true)), true),
            (Some("0.65.1"), Some(serde_json::json!(true)), true),
            (Some("1.0.0"), Some(serde_json::json!(true)), true),
            (Some("0.64.2"), Some(serde_json::json!(true)), false),
            (Some("0.65.0-beta.1"), Some(serde_json::json!(true)), false),
            (Some("invalid"), Some(serde_json::json!(true)), false),
            (None, Some(serde_json::json!(true)), false),
            (Some("0.65.0"), Some(serde_json::json!(false)), false),
            (Some("0.65.0"), Some(serde_json::json!("true")), false),
            (Some("0.65.0"), None, false),
        ] {
            assert_eq!(
                is_supported(&initialize_response(version, capability)),
                expected,
                "unexpected support result for version {version:?}"
            );
        }
    }

    #[test]
    fn serializes_prompt_required_request_shape() {
        let request = ClaudeSteeringRequest::new(
            SessionId::new("claude-session"),
            vec![ContentBlock::Text(TextContent::new("focus on tests"))],
        );

        assert_eq!(
            agent_client_protocol::JsonRpcMessage::method(&request),
            "_session/steering"
        );
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "sessionId": "claude-session",
                "prompt": [{ "type": "text", "text": "focus on tests" }],
                "_meta": {
                    "steering": {
                        "idleBehavior": "promptRequired"
                    }
                }
            })
        );
    }

    #[test]
    fn accepts_prompt_required_with_unknown_or_missing_reason() {
        for response in [
            serde_json::json!({
                "outcome": "promptRequired",
                "reason": "futureReason"
            }),
            serde_json::json!({ "outcome": "promptRequired" }),
        ] {
            assert!(matches!(
                serde_json::from_value(response).unwrap(),
                ClaudeSteeringResponse::PromptRequired
            ));
        }
    }

    #[test]
    fn rejects_unknown_delivery_outcome() {
        let response = serde_json::json!({ "outcome": "futureOutcome" });

        assert!(serde_json::from_value::<ClaudeSteeringResponse>(response).is_err());
    }

    #[test]
    fn confirms_only_injected_delivery() {
        assert!(delivery_confirmed(ClaudeSteeringResponse::Injected));
        assert!(!delivery_confirmed(ClaudeSteeringResponse::PromptRequired));
    }
}
