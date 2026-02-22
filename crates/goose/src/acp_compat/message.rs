//! Bidirectional converters between goose Message ↔ ACP Message.
//!
//! ACP v0.2.0 uses a multi-part message format where each part has a content_type
//! and inline content. goose uses MCP-style MessageContent variants (Text, Image,
//! ToolRequest, ToolResponse, etc.).

use rmcp::model::{CallToolRequestParams, CallToolResult, Content, RawContent, RawTextContent};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::conversation::message::{
    ActionRequired, ActionRequiredData, Message, MessageContent, ThinkingContent, ToolRequest,
    ToolResponse,
};

/// ACP message: role + ordered parts.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcpMessage {
    pub role: AcpRole,
    pub parts: Vec<AcpMessagePart>,
}

/// ACP role — "user" or "agent" (with optional sub-agent path like "agent/coding").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AcpRole {
    User,
    Agent,
}

/// A single part of an ACP message.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcpMessagePart {
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AcpMessagePart {
    pub fn text(text: impl Into<String>) -> Self {
        AcpMessagePart {
            content_type: "text/plain".to_string(),
            content: Some(text.into()),
            content_url: None,
            content_encoding: None,
            metadata: None,
        }
    }

    pub fn json(value: &serde_json::Value, metadata: Option<serde_json::Value>) -> Self {
        AcpMessagePart {
            content_type: "application/json".to_string(),
            content: Some(value.to_string()),
            content_url: None,
            content_encoding: None,
            metadata,
        }
    }

    pub fn image(data: &str, mime_type: &str) -> Self {
        AcpMessagePart {
            content_type: mime_type.to_string(),
            content: Some(data.to_string()),
            content_url: None,
            content_encoding: Some("base64".to_string()),
            metadata: None,
        }
    }
}

/// Convert a goose Message to an ACP Message.
pub fn goose_message_to_acp(msg: &Message) -> AcpMessage {
    let role = match msg.role {
        rmcp::model::Role::User => AcpRole::User,
        rmcp::model::Role::Assistant => AcpRole::Agent,
    };

    let parts = msg
        .content
        .iter()
        .filter_map(goose_content_to_acp_part)
        .collect();

    AcpMessage { role, parts }
}

/// Convert a single goose MessageContent to an ACP MessagePart.
fn goose_content_to_acp_part(content: &MessageContent) -> Option<AcpMessagePart> {
    match content {
        MessageContent::Text(text) => Some(AcpMessagePart::text(&text.text)),

        MessageContent::JsonRenderSpec(spec) => {
            let metadata = serde_json::json!({
                "goose": { "type": "json_render_spec" }
            });
            Some(AcpMessagePart {
                content_type: "application/json".to_string(),
                content: Some(spec.spec.clone()),
                content_url: None,
                content_encoding: None,
                metadata: Some(metadata),
            })
        }

        MessageContent::Image(image) => Some(AcpMessagePart::image(&image.data, &image.mime_type)),

        MessageContent::ToolRequest(req) => {
            let (name, arguments) = match &req.tool_call {
                Ok(call) => (
                    call.name.to_string(),
                    call.arguments
                        .as_ref()
                        .map(|a| serde_json::to_value(a).unwrap_or_default())
                        .unwrap_or(serde_json::Value::Object(Default::default())),
                ),
                Err(e) => (
                    "error".to_string(),
                    serde_json::json!({ "error": e.message.to_string() }),
                ),
            };

            let payload = serde_json::json!({
                "tool_name": name,
                "arguments": arguments,
            });

            let metadata = serde_json::json!({
                "trajectory": {
                    "type": "tool_call",
                    "tool_call_id": req.id,
                }
            });

            Some(AcpMessagePart::json(&payload, Some(metadata)))
        }

        MessageContent::ToolResponse(res) => {
            let result_content = match &res.tool_result {
                Ok(result) => {
                    let texts: Vec<String> = result
                        .content
                        .iter()
                        .filter_map(|c| c.as_text().map(|t| t.text.to_string()))
                        .collect();
                    serde_json::json!({
                        "output": texts.join("
                    "),
                        "is_error": result.is_error.unwrap_or(false),
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "output": e.message.to_string(),
                        "is_error": true,
                    })
                }
            };

            let metadata = serde_json::json!({
                "trajectory": {
                    "type": "tool_result",
                    "tool_call_id": res.id,
                }
            });

            Some(AcpMessagePart::json(&result_content, Some(metadata)))
        }

        MessageContent::Thinking(thinking) => {
            let metadata = serde_json::json!({
                "trajectory": { "type": "thinking" }
            });
            Some(AcpMessagePart {
                content_type: "text/plain".to_string(),
                content: Some(thinking.thinking.clone()),
                content_url: None,
                content_encoding: None,
                metadata: Some(metadata),
            })
        }

        MessageContent::ActionRequired(action) => {
            let payload = serde_json::to_value(&action.data).unwrap_or_default();
            let metadata = serde_json::json!({
                "goose": { "type": "action_required" }
            });
            Some(AcpMessagePart::json(&payload, Some(metadata)))
        }

        // These are goose-internal UI concerns, not meaningful for ACP wire format
        MessageContent::ToolConfirmationRequest(_)
        | MessageContent::FrontendToolRequest(_)
        | MessageContent::RedactedThinking(_)
        | MessageContent::SystemNotification(_) => None,
    }
}

/// Convert an ACP Message to a goose Message.
pub fn acp_message_to_goose(acp: &AcpMessage) -> Message {
    let mut msg = match acp.role {
        AcpRole::User => Message::user(),
        AcpRole::Agent => Message::assistant(),
    };

    for part in &acp.parts {
        if let Some(content) = acp_part_to_goose_content(part) {
            msg = msg.with_content(content);
        }
    }

    msg
}

/// Convert a single ACP MessagePart to a goose MessageContent.
fn acp_part_to_goose_content(part: &AcpMessagePart) -> Option<MessageContent> {
    let content_str = part.content.as_deref().unwrap_or("");
    let trajectory_type = part
        .metadata
        .as_ref()
        .and_then(|m| m.get("trajectory"))
        .and_then(|t| t.get("type"))
        .and_then(|t| t.as_str());

    match trajectory_type {
        Some("tool_call") => parse_tool_call_part(part, content_str),
        Some("tool_result") => parse_tool_result_part(part, content_str),
        Some("thinking") => Some(MessageContent::Thinking(ThinkingContent {
            thinking: content_str.to_string(),
            signature: String::new(),
        })),
        _ => parse_content_part(part, content_str),
    }
}

fn parse_tool_call_part(part: &AcpMessagePart, content_str: &str) -> Option<MessageContent> {
    let tool_call_id = part
        .metadata
        .as_ref()
        .and_then(|m| m.get("trajectory"))
        .and_then(|t| t.get("tool_call_id"))
        .and_then(|id| id.as_str())
        .unwrap_or("unknown")
        .to_string();

    let parsed: serde_json::Value = serde_json::from_str(content_str).ok()?;
    let tool_name = parsed
        .get("tool_name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();
    let arguments = parsed.get("arguments").cloned();

    let arguments_obj = arguments.and_then(|a| {
        if let serde_json::Value::Object(map) = a {
            Some(map)
        } else {
            None
        }
    });

    Some(MessageContent::ToolRequest(ToolRequest {
        id: tool_call_id,
        tool_call: Ok(CallToolRequestParams {
            meta: None,
            task: None,
            name: tool_name.into(),
            arguments: arguments_obj,
        }),
        metadata: None,
        tool_meta: None,
    }))
}

fn parse_tool_result_part(part: &AcpMessagePart, content_str: &str) -> Option<MessageContent> {
    let tool_call_id = part
        .metadata
        .as_ref()
        .and_then(|m| m.get("trajectory"))
        .and_then(|t| t.get("tool_call_id"))
        .and_then(|id| id.as_str())
        .unwrap_or("unknown")
        .to_string();

    let parsed: serde_json::Value = serde_json::from_str(content_str).ok()?;
    let output = parsed.get("output").and_then(|o| o.as_str()).unwrap_or("");
    let is_error = parsed
        .get("is_error")
        .and_then(|e| e.as_bool())
        .unwrap_or(false);

    let text_content = Content {
        raw: RawContent::Text(RawTextContent {
            text: output.to_string(),
            meta: None,
        }),
        annotations: None,
    };

    Some(MessageContent::ToolResponse(ToolResponse {
        id: tool_call_id,
        tool_result: Ok(CallToolResult {
            content: vec![text_content],
            is_error: Some(is_error),
            structured_content: None,
            meta: None,
        }),
        metadata: None,
    }))
}

fn parse_content_part(part: &AcpMessagePart, content_str: &str) -> Option<MessageContent> {
    let ct = &part.content_type;

    if ct.starts_with("text/") {
        Some(MessageContent::text(content_str))
    } else if ct.starts_with("image/") {
        Some(MessageContent::image(content_str, ct))
    } else if ct == "application/json" {
        // Generic JSON part with goose-specific metadata
        let goose_type = part
            .metadata
            .as_ref()
            .and_then(|m| m.get("goose"))
            .and_then(|g| g.get("type"))
            .and_then(|t| t.as_str());

        match goose_type {
            Some("action_required") => {
                let data: ActionRequiredData = serde_json::from_str(content_str).ok()?;
                Some(MessageContent::ActionRequired(ActionRequired { data }))
            }
            Some("json_render_spec") => Some(MessageContent::json_render_spec(content_str)),
            _ => Some(MessageContent::text(content_str)),
        }
    } else {
        Some(MessageContent::text(format!(
            "[Unsupported content_type: {}]",
            ct
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Role;
    use rmcp::object;

    #[test]
    fn test_text_roundtrip() {
        let goose_msg = Message::user().with_text("Hello, world!");
        let acp = goose_message_to_acp(&goose_msg);

        assert_eq!(acp.role, AcpRole::User);
        assert_eq!(acp.parts.len(), 1);
        assert_eq!(acp.parts[0].content_type, "text/plain");
        assert_eq!(acp.parts[0].content.as_deref(), Some("Hello, world!"));

        let roundtrip = acp_message_to_goose(&acp);
        assert_eq!(roundtrip.role, Role::User);
        assert_eq!(roundtrip.as_concat_text(), "Hello, world!");
    }

    #[test]
    fn test_image_roundtrip() {
        let goose_msg = Message::user().with_image("base64data", "image/png");
        let acp = goose_message_to_acp(&goose_msg);

        assert_eq!(acp.parts.len(), 1);
        assert_eq!(acp.parts[0].content_type, "image/png");
        assert_eq!(acp.parts[0].content.as_deref(), Some("base64data"));
        assert_eq!(acp.parts[0].content_encoding.as_deref(), Some("base64"));

        let roundtrip = acp_message_to_goose(&acp);
        if let MessageContent::Image(img) = &roundtrip.content[0] {
            assert_eq!(img.data, "base64data");
            assert_eq!(img.mime_type, "image/png");
        } else {
            panic!("Expected Image content");
        }
    }

    #[test]
    fn test_tool_request_roundtrip() {
        let goose_msg = Message::assistant().with_tool_request(
            "call_1",
            Ok(CallToolRequestParams {
                meta: None,
                task: None,
                name: "shell".into(),
                arguments: Some(object!({"command": "ls"})),
            }),
        );

        let acp = goose_message_to_acp(&goose_msg);
        assert_eq!(acp.role, AcpRole::Agent);
        assert_eq!(acp.parts.len(), 1);
        assert_eq!(acp.parts[0].content_type, "application/json");

        let trajectory = acp.parts[0]
            .metadata
            .as_ref()
            .unwrap()
            .get("trajectory")
            .unwrap();
        assert_eq!(trajectory["type"], "tool_call");
        assert_eq!(trajectory["tool_call_id"], "call_1");

        let roundtrip = acp_message_to_goose(&acp);
        assert_eq!(roundtrip.role, Role::Assistant);
        if let MessageContent::ToolRequest(req) = &roundtrip.content[0] {
            assert_eq!(req.id, "call_1");
            let call = req.tool_call.as_ref().unwrap();
            assert_eq!(call.name.as_ref(), "shell");
            assert_eq!(call.arguments.as_ref().unwrap()["command"], "ls");
        } else {
            panic!("Expected ToolRequest content");
        }
    }

    #[test]
    fn test_tool_response_roundtrip() {
        let text_content = Content {
            raw: RawContent::Text(RawTextContent {
                text: "file1.txt
file2.txt"
                    .to_string(),
                meta: None,
            }),
            annotations: None,
        };

        let goose_msg = Message::user().with_tool_response(
            "call_1",
            Ok(CallToolResult {
                content: vec![text_content],
                is_error: Some(false),
                structured_content: None,
                meta: None,
            }),
        );

        let acp = goose_message_to_acp(&goose_msg);
        assert_eq!(acp.parts.len(), 1);

        let trajectory = acp.parts[0]
            .metadata
            .as_ref()
            .unwrap()
            .get("trajectory")
            .unwrap();
        assert_eq!(trajectory["type"], "tool_result");
        assert_eq!(trajectory["tool_call_id"], "call_1");

        let roundtrip = acp_message_to_goose(&acp);
        if let MessageContent::ToolResponse(res) = &roundtrip.content[0] {
            assert_eq!(res.id, "call_1");
            let result = res.tool_result.as_ref().unwrap();
            assert_eq!(
                result.content[0].as_text().unwrap().text,
                "file1.txt
file2.txt"
            );
            assert_eq!(result.is_error, Some(false));
        } else {
            panic!("Expected ToolResponse content");
        }
    }

    #[test]
    fn test_multi_content_message() {
        let goose_msg = Message::assistant()
            .with_text("I'll run that command for you.")
            .with_tool_request(
                "call_2",
                Ok(CallToolRequestParams {
                    meta: None,
                    task: None,
                    name: "shell".into(),
                    arguments: Some(object!({"command": "echo hi"})),
                }),
            );

        let acp = goose_message_to_acp(&goose_msg);
        assert_eq!(acp.parts.len(), 2);
        assert_eq!(acp.parts[0].content_type, "text/plain");
        assert_eq!(acp.parts[1].content_type, "application/json");

        let roundtrip = acp_message_to_goose(&acp);
        assert_eq!(roundtrip.content.len(), 2);
        assert!(matches!(&roundtrip.content[0], MessageContent::Text(_)));
        assert!(matches!(
            &roundtrip.content[1],
            MessageContent::ToolRequest(_)
        ));
    }

    #[test]
    fn test_acp_text_to_goose() {
        let acp = AcpMessage {
            role: AcpRole::User,
            parts: vec![AcpMessagePart::text("Hello from ACP")],
        };

        let goose = acp_message_to_goose(&acp);
        assert_eq!(goose.role, Role::User);
        assert_eq!(goose.as_concat_text(), "Hello from ACP");
    }

    #[test]
    fn test_system_notification_filtered_out() {
        use crate::conversation::message::SystemNotificationType;
        let goose_msg = Message::assistant()
            .with_text("visible")
            .with_system_notification(SystemNotificationType::InlineMessage, "internal");

        let acp = goose_message_to_acp(&goose_msg);
        assert_eq!(acp.parts.len(), 1);
        assert_eq!(acp.parts[0].content.as_deref(), Some("visible"));
    }

    #[test]
    fn test_thinking_roundtrip() {
        let goose_msg = Message::assistant().with_thinking("Let me think about this...", "sig123");

        let acp = goose_message_to_acp(&goose_msg);
        assert_eq!(acp.parts.len(), 1);
        assert_eq!(acp.parts[0].content_type, "text/plain");
        assert_eq!(
            acp.parts[0].content.as_deref(),
            Some("Let me think about this...")
        );
        let trajectory = acp.parts[0]
            .metadata
            .as_ref()
            .unwrap()
            .get("trajectory")
            .unwrap();
        assert_eq!(trajectory["type"], "thinking");

        let roundtrip = acp_message_to_goose(&acp);
        if let MessageContent::Thinking(t) = &roundtrip.content[0] {
            assert_eq!(t.thinking, "Let me think about this...");
        } else {
            panic!("Expected Thinking content");
        }
    }
}
