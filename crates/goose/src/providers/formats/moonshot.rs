use crate::conversation::message::{Message, MessageContent, ProviderMetadata};
use crate::providers::formats::openai;
use rmcp::model::Role;
use serde_json::{json, Value};

pub const REASONING_CONTENT_KEY: &str = "reasoning_content";

fn has_assistant_content(message: &Message) -> bool {
    message.content.iter().any(|c| match c {
        MessageContent::Text(t) => !t.text.is_empty(),
        MessageContent::Image(_) => true,
        MessageContent::ToolRequest(req) => req.tool_call.is_ok(),
        MessageContent::FrontendToolRequest(req) => req.tool_call.is_ok(),
        _ => false,
    })
}

pub fn extract_reasoning_content(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|m| m.get("message"))
        .and_then(|msg| msg.get("reasoning_content"))
        .and_then(|r| r.as_str())
        .map(|s| s.to_string())
}

pub fn get_reasoning_content(metadata: &Option<ProviderMetadata>) -> Option<String> {
    metadata
        .as_ref()
        .and_then(|m| m.get(REASONING_CONTENT_KEY))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let mut message = openai::response_to_message(response)?;

    if let Some(reasoning) = extract_reasoning_content(response) {
        for content in &mut message.content {
            if let MessageContent::ToolRequest(req) = content {
                let mut meta = req.metadata.clone().unwrap_or_default();
                meta.insert(REASONING_CONTENT_KEY.to_string(), json!(reasoning));
                req.metadata = Some(meta);
            }
        }
    }

    Ok(message)
}

pub fn add_reasoning_content_to_request(payload: &mut Value, messages: &[Message]) {
    let mut assistant_reasoning: Vec<Option<String>> = messages
        .iter()
        .filter(|m| m.is_agent_visible())
        .filter(|m| m.role == Role::Assistant)
        .filter(|m| has_assistant_content(m))
        .map(|message| {
            message.content.iter().find_map(|c| match c {
                MessageContent::ToolRequest(req) => get_reasoning_content(&req.metadata),
                _ => None,
            })
        })
        .collect();

    if let Some(payload_messages) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|m| m.as_array_mut())
    {
        let mut assistant_idx = 0;
        for payload_msg in payload_messages.iter_mut() {
            if payload_msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                if assistant_idx < assistant_reasoning.len() {
                    if let Some(reasoning) = assistant_reasoning
                        .get_mut(assistant_idx)
                        .and_then(|r| r.take())
                    {
                        if let Some(obj) = payload_msg.as_object_mut() {
                            obj.insert("reasoning_content".to_string(), json!(reasoning));
                        }
                    }
                }
                assistant_idx += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_reasoning_content() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Hello",
                    "reasoning_content": "Let me think about this..."
                }
            }]
        });

        let reasoning = extract_reasoning_content(&response).unwrap();
        assert_eq!(reasoning, "Let me think about this...");
    }

    #[test]
    fn test_response_to_message_with_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"NYC\"}"
                        }
                    }],
                    "reasoning_content": "I need to check the weather"
                }
            }]
        });

        let message = response_to_message(&response).unwrap();
        assert!(!message.content.is_empty());

        let tool_request = message
            .content
            .iter()
            .find_map(|c| {
                if let MessageContent::ToolRequest(req) = c {
                    Some(req)
                } else {
                    None
                }
            })
            .unwrap();

        assert!(tool_request.metadata.is_some());
        let reasoning = get_reasoning_content(&tool_request.metadata).unwrap();
        assert_eq!(reasoning, "I need to check the weather");
    }
}
