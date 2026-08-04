use agent_client_protocol::schema::v1::{ContentBlock, SessionId};
use agent_client_protocol::{JsonRpcRequest, JsonRpcResponse};
use serde::{Deserialize, Serialize};

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
