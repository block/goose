use super::*;

// https://github.com/anthropics/claude-agent-sdk-python/blob/0e9397e/src/claude_agent_sdk/types.py#L857-L859
#[derive(Serialize)]
pub(super) struct ControlResponse<T: Serialize> {
    #[serde(rename = "type")]
    pub msg_type: &'static str,
    pub response: ControlResponseBody<T>,
}

#[derive(Serialize)]
pub(super) struct ControlResponseBody<T: Serialize> {
    pub subtype: &'static str,
    pub request_id: String,
    pub response: T,
}

// https://github.com/anthropics/claude-agent-sdk-python/blob/0e9397e/src/claude_agent_sdk/types.py#L135-L153
#[derive(Serialize)]
#[serde(tag = "behavior")]
pub(super) enum PermissionResponse {
    #[serde(rename = "allow")]
    Allow {
        #[serde(rename = "updatedInput")]
        updated_input: serde_json::Map<String, Value>,
        #[serde(rename = "toolUseID")]
        tool_use_id: String,
    },
    #[serde(rename = "deny")]
    Deny { message: String },
}

#[derive(Serialize)]
pub(super) struct ControlRequest {
    #[serde(rename = "type")]
    pub msg_type: &'static str,
    pub request_id: String,
    pub request: ControlRequestBody,
}

#[derive(Serialize)]
#[serde(tag = "subtype")]
pub(super) enum ControlRequestBody {
    #[serde(rename = "initialize")]
    Initialize,
    #[serde(rename = "set_model")]
    SetModel { model: String },
}

impl ControlRequestBody {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::SetModel { .. } => "set_model",
        }
    }
}

#[derive(Deserialize)]
pub(super) struct IncomingControlResponse {
    pub response: IncomingControlResponseBody,
}

#[derive(Deserialize)]
#[serde(tag = "subtype")]
pub(super) enum IncomingControlResponseBody {
    #[serde(rename = "success")]
    Success {
        request_id: String,
        #[serde(default)]
        response: Option<Value>,
    },
    #[serde(rename = "error")]
    Error {
        request_id: String,
        #[serde(default)]
        error: String,
    },
}

#[derive(Deserialize)]
pub(super) struct IncomingControlRequest {
    pub request_id: String,
    pub request: IncomingRequestBody,
}

#[derive(Deserialize)]
#[serde(tag = "subtype")]
pub(super) enum IncomingRequestBody {
    #[serde(rename = "can_use_tool")]
    CanUseTool {
        tool_name: String,
        #[serde(default)]
        input: serde_json::Map<String, Value>,
        #[serde(default)]
        tool_use_id: String,
    },
}

impl<T: Serialize> ControlResponse<T> {
    pub fn success(request_id: String, response: T) -> Self {
        Self {
            msg_type: "control_response",
            response: ControlResponseBody {
                subtype: "success",
                request_id,
                response,
            },
        }
    }
}

pub(super) async fn exchange_control(
    stdin: &mut (impl AsyncWrite + Unpin),
    reader: &mut (impl AsyncBufRead + Unpin),
    request_id: &str,
    body: ControlRequestBody,
) -> Result<Option<Value>, ProviderError> {
    let label = body.label();
    let req = ControlRequest {
        msg_type: "control_request",
        request_id: request_id.to_string(),
        request: body,
    };
    let mut req_str = serde_json::to_string(&req).map_err(|e| {
        ProviderError::RequestFailed(format!("Failed to serialize {label} request: {e}"))
    })?;
    req_str.push('\n');
    stdin.write_all(req_str.as_bytes()).await.map_err(|e| {
        ProviderError::RequestFailed(format!("Failed to write {label} request: {e}"))
    })?;

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                return Err(ProviderError::RequestFailed(format!(
                    "CLI process terminated while waiting for {label} response"
                )));
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if let Ok(msg) = serde_json::from_str::<IncomingControlResponse>(trimmed) {
                    match msg.response {
                        IncomingControlResponseBody::Success {
                            request_id: ref rid,
                            response,
                        } if rid == request_id => return Ok(response),
                        IncomingControlResponseBody::Error {
                            request_id: ref rid,
                            error,
                        } if rid == request_id => {
                            return Err(ProviderError::RequestFailed(format!(
                                "{label} failed: {error}"
                            )));
                        }
                        _ => continue,
                    }
                }
            }
            Err(e) => {
                return Err(ProviderError::RequestFailed(format!(
                    "Failed to read {label} response: {e}"
                )));
            }
        }
    }
}
