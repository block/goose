use agent_client_protocol::schema::v1::{ContentBlock, InitializeResponse, SessionId};
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use semver::Version;
use serde::{Deserialize, Serialize};

const MINIMUM_VERSION: Version = Version::new(0, 64, 0);

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
            .is_some_and(|version| version >= MINIMUM_VERSION)
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
    meta: ClaudeSteeringMeta,
}

impl ClaudeSteeringRequest {
    pub(super) fn new(session_id: SessionId, prompt: Vec<ContentBlock>) -> Self {
        Self {
            session_id,
            prompt,
            meta: ClaudeSteeringMeta {
                steering: ClaudeSteeringOptions {
                    idle_behavior: ClaudeSteeringIdleBehavior::PromptRequired,
                },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClaudeSteeringMeta {
    steering: ClaudeSteeringOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeSteeringOptions {
    idle_behavior: ClaudeSteeringIdleBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ClaudeSteeringIdleBehavior {
    PromptRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonRpcResponse)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub(super) enum ClaudeSteeringResponse {
    Injected,
    PromptRequired { reason: ClaudePromptRequiredReason },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum ClaudePromptRequiredReason {
    NoRunningTurn,
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::Implementation;
    use agent_client_protocol::schema::ProtocolVersion;

    fn initialize_response(version: &str, steering_supported: Option<bool>) -> InitializeResponse {
        let response = InitializeResponse::new(ProtocolVersion::LATEST)
            .agent_info(Implementation::new("claude-agent-acp", version));

        if let Some(supported) = steering_supported {
            let mut meta = serde_json::Map::new();
            meta.insert(
                "steering".to_string(),
                serde_json::json!({ "supported": supported }),
            );
            response.meta(meta)
        } else {
            response
        }
    }

    #[test]
    fn detects_prompt_required_steering_support() {
        for (version, steering_supported, expected) in [
            ("0.64.0", Some(true), true),
            ("0.63.9", Some(true), false),
            ("invalid", Some(true), false),
            ("0.64.0", None, false),
        ] {
            let response = initialize_response(version, steering_supported);
            assert_eq!(is_supported(&response), expected);
        }
    }
}
