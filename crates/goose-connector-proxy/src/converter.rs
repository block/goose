//! Core conversion logic between OpenAI Chat Completions format and the custom LLM format.

use serde_json::Value;

use crate::models::{CustomLlmRequest, CustomLlmResponse, LlmConfig, OpenAiChatRequest, ProxyConfig};
use crate::structured_output::{build_structured_output_prompt, parse_structured_output};
use crate::tool_injection::{
    build_tool_use_prompt, format_tool_result, parse_tool_calls, serialize_tool_calls_to_text,
};

/// Build the complete system prompt augmentation combining tools and structured output.
fn build_system_augmentation(
    tools: Option<&[Value]>,
    tool_choice: Option<&Value>,
    response_format: Option<&Value>,
) -> Option<String> {
    let mut parts = Vec::new();

    if let Some(tools) = tools {
        if let Some(tool_prompt) = build_tool_use_prompt(tools, tool_choice) {
            parts.push(tool_prompt);
        }
    }

    if let Some(rf) = response_format {
        if let Some(structured_prompt) = build_structured_output_prompt(rf) {
            parts.push(structured_prompt);
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Convert OpenAI-format messages to custom LLM contents (list of JSON strings).
///
/// Handles:
/// - System message augmentation with tool/structured output instructions
/// - Assistant messages with tool_calls -> reconstructed with `<tool_call>` tags
/// - Tool role messages -> converted to user messages with `<tool_response>` tags
pub fn openai_messages_to_contents(
    request: &OpenAiChatRequest,
) -> Vec<String> {
    let tools_slice = request
        .tools
        .as_ref()
        .map(|t| t.as_slice());
    let augmentation = build_system_augmentation(
        tools_slice,
        request.tool_choice.as_ref(),
        request.response_format.as_ref(),
    );
    let mut augmentation_injected = false;
    let mut result = Vec::new();

    for msg in &request.messages {
        let role = &msg.role;
        let content = extract_text_content(&msg.content);
        let tool_calls = &msg.tool_calls;
        let tool_call_id = &msg.tool_call_id;

        match role.as_str() {
            "system" => {
                let mut sys_content = content;
                if let Some(ref aug) = augmentation {
                    if !augmentation_injected {
                        sys_content = format!("{}\n\n{}", sys_content, aug);
                        augmentation_injected = true;
                    }
                }
                result.push(
                    serde_json::json!({"role": "system", "content": sys_content}).to_string(),
                );
            }
            "user" => {
                // If no system message has been seen yet, inject augmentation as a system message
                if let Some(ref aug) = augmentation {
                    if !augmentation_injected {
                        result.push(
                            serde_json::json!({"role": "system", "content": aug}).to_string(),
                        );
                        augmentation_injected = true;
                    }
                }
                result.push(
                    serde_json::json!({"role": "user", "content": content}).to_string(),
                );
            }
            "assistant" => {
                let mut assistant_content = content;
                if let Some(tc) = tool_calls {
                    if !tc.is_empty() {
                        let tool_text = serialize_tool_calls_to_text(tc);
                        assistant_content = if assistant_content.is_empty() {
                            tool_text
                        } else {
                            format!("{}\n{}", assistant_content, tool_text).trim().to_string()
                        };
                    }
                }
                result.push(
                    serde_json::json!({"role": "assistant", "content": assistant_content})
                        .to_string(),
                );
            }
            "tool" => {
                let tcid = tool_call_id
                    .as_deref()
                    .unwrap_or("unknown");
                let formatted = format_tool_result(tcid, &content);
                result.push(
                    serde_json::json!({"role": "user", "content": formatted}).to_string(),
                );
            }
            _ => {
                // Unknown role - pass through as user
                result.push(
                    serde_json::json!({"role": "user", "content": content}).to_string(),
                );
            }
        }
    }

    result
}

/// Extract text content from an OpenAI message content field.
///
/// Content can be a string, a list of content parts, or null.
fn extract_text_content(content: &Option<Value>) -> String {
    match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => {
            let mut text_parts = Vec::new();
            for part in parts {
                if let Some(obj) = part.as_object() {
                    if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                            text_parts.push(text.to_string());
                        }
                    }
                } else if let Some(s) = part.as_str() {
                    text_parts.push(s.to_string());
                }
            }
            text_parts.join("\n")
        }
        _ => String::new(),
    }
}

/// Build an OpenAI request with tool injection (for OpenAI proxy mode).
/// Keeps OpenAI format but injects tools into the system message.
pub fn build_openai_request_with_tools(
    request: &OpenAiChatRequest,
    config: &ProxyConfig,
) -> Value {
    let tools_slice = request.tools.as_ref().map(|t| t.as_slice());

    let augmentation = build_system_augmentation(
        tools_slice,
        request.tool_choice.as_ref(),
        request.response_format.as_ref(),
    );

    // Build messages with tool injection
    let mut messages = Vec::new();
    let mut augmentation_injected = false;

    for msg in &request.messages {
        let role = msg.role.as_str();
        let content = extract_text_content(&msg.content);

        match role {
            "system" => {
                let mut sys_content = content;
                if let Some(ref aug) = augmentation {
                    if !augmentation_injected {
                        sys_content = format!("{}\n\n{}", sys_content, aug);
                        augmentation_injected = true;
                    }
                }
                messages.push(serde_json::json!({"role": "system", "content": sys_content}));
            }
            "user" => {
                // If no system message has been seen yet, inject augmentation as a system message
                if let Some(ref aug) = augmentation {
                    if !augmentation_injected {
                        messages.push(serde_json::json!({"role": "system", "content": aug}));
                        augmentation_injected = true;
                    }
                }
                messages.push(serde_json::json!({"role": "user", "content": content}));
            }
            "assistant" => {
                // Handle tool_calls in assistant message
                if let Some(tool_calls) = &msg.tool_calls {
                    if !tool_calls.is_empty() {
                        let tool_text = serialize_tool_calls_to_text(tool_calls);
                        let combined = if content.is_empty() {
                            tool_text
                        } else {
                            format!("{}\n{}", content, tool_text)
                        };
                        messages.push(serde_json::json!({"role": "assistant", "content": combined}));
                    } else {
                        messages.push(serde_json::json!({"role": "assistant", "content": content}));
                    }
                } else {
                    messages.push(serde_json::json!({"role": "assistant", "content": content}));
                }
            }
            "tool" => {
                // Convert tool response to user message format
                let tool_id = msg.tool_call_id.as_deref().unwrap_or("unknown");
                let formatted = format_tool_result(tool_id, &content);
                messages.push(serde_json::json!({"role": "user", "content": formatted}));
            }
            _ => {
                messages.push(serde_json::json!({"role": role, "content": content}));
            }
        }
    }

    // Build OpenAI format request (without tools - they're injected into messages)
    // Respect force_non_stream config option
    let stream = if config.force_non_stream {
        false
    } else {
        request.stream.unwrap_or(false)
    };
    let mut openai_body = serde_json::json!({
        "model": config.llm_id,
        "messages": messages,
        "stream": stream,
    });

    // Add optional parameters
    if let Some(temp) = request.temperature {
        openai_body["temperature"] = serde_json::json!(temp);
    }
    if let Some(top_p) = request.top_p {
        openai_body["top_p"] = serde_json::json!(top_p);
    }
    if let Some(max_tokens) = request.max_tokens.or(request.max_completion_tokens) {
        openai_body["max_tokens"] = serde_json::json!(max_tokens);
    }

    openai_body
}

/// Convert an OpenAI ChatCompletion request body to the custom LLM request body.
pub fn build_custom_request(
    request: &OpenAiChatRequest,
    config: &ProxyConfig,
) -> CustomLlmRequest {
    let contents = openai_messages_to_contents(request);

    let temperature = request.temperature.unwrap_or(config.temperature);
    let top_p = request.top_p.unwrap_or(config.top_p);
    let max_tokens = request
        .max_completion_tokens
        .or(request.max_tokens)
        .unwrap_or(config.max_tokens);

    // Respect force_non_stream config option
    let is_stream = if config.force_non_stream {
        false
    } else {
        request.stream.unwrap_or(false)
    };

    CustomLlmRequest {
        contents,
        llm_id: config.llm_id.clone(),
        is_stream,
        llm_config: LlmConfig {
            temperature,
            top_p,
            top_k: config.top_k,
            repitition_penalty: config.repetition_penalty,
            max_new_token: max_tokens,
        },
    }
}

/// Convert a custom LLM response to an OpenAI ChatCompletion response.
///
/// Handles:
/// - Parsing tool calls from `<tool_call>` tags in content
/// - Cleaning structured output JSON
/// - Mapping token counts
/// - Error status handling
pub fn custom_response_to_openai(
    custom_response: &CustomLlmResponse,
    openai_request: &OpenAiChatRequest,
) -> Value {
    let status = custom_response
        .status
        .as_deref()
        .unwrap_or("SUCCESS");

    if status == "FAIL" {
        return serde_json::json!({
            "error": {
                "message": format!(
                    "Custom LLM returned FAIL: {}",
                    custom_response.response_code.as_deref().unwrap_or("unknown")
                ),
                "type": "server_error",
                "code": "llm_error",
            }
        });
    }

    let raw_content = custom_response
        .content
        .clone()
        .unwrap_or_default();
    let raw_reasoning = custom_response
        .reasoning
        .clone()
        .unwrap_or_default();
    let mut content: Option<String> = Some(raw_content.clone());
    let mut tool_calls_parsed: Option<Vec<Value>> = None;
    let mut finish_reason = "stop".to_string();

    // Always try to parse tool calls (no has_tools hint from LLM)
    // Try content first, then reasoning field
    if !raw_content.is_empty() {
        let (parsed, remaining) = parse_tool_calls(&raw_content);
        if let Some(calls) = parsed {
            content = if remaining.trim().is_empty() {
                None
            } else {
                Some(remaining)
            };
            tool_calls_parsed = Some(calls);
            finish_reason = "tool_calls".to_string();
        }
    }

    // If no tool_calls found in content, try reasoning field
    if tool_calls_parsed.is_none() && !raw_reasoning.is_empty() {
        let (parsed, _) = parse_tool_calls(&raw_reasoning);
        if let Some(calls) = parsed {
            // Keep original content, just extract tool_calls from reasoning
            tool_calls_parsed = Some(calls);
            finish_reason = "tool_calls".to_string();
        }
    }

    // If response_format was json_schema or json_object, clean the output
    if let Some(ref rf) = openai_request.response_format {
        let fmt_type = rf.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if (fmt_type == "json_schema" || fmt_type == "json_object")
            && tool_calls_parsed.is_none()
        {
            if let Some(ref c) = content {
                if !c.is_empty() {
                    content = Some(parse_structured_output(c));
                }
            }
        }
    }

    let mut message = serde_json::json!({
        "role": "assistant",
        "content": content,
    });
    if let Some(ref calls) = tool_calls_parsed {
        message["tool_calls"] = serde_json::json!(calls);
    }

    let prompt_tokens = custom_response.prompt_token.unwrap_or(0);
    let completion_tokens = custom_response.completion_token.unwrap_or(0);

    serde_json::json!({
        "id": format!("chatcmpl-{}", custom_response.id.as_deref().unwrap_or("unknown")),
        "object": "chat.completion",
        "created": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "model": openai_request.model.as_deref().unwrap_or("custom-llm"),
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason,
        }],
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": prompt_tokens + completion_tokens,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OpenAiMessage;

    fn make_request(messages: Vec<OpenAiMessage>) -> OpenAiChatRequest {
        OpenAiChatRequest {
            model: Some("test-model".to_string()),
            messages,
            tools: None,
            tool_choice: None,
            response_format: None,
            temperature: None,
            top_p: None,
            max_tokens: None,
            max_completion_tokens: None,
            stream: None,
        }
    }

    #[test]
    fn test_messages_to_contents_basic() {
        let req = make_request(vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: Some(Value::String("You are helpful.".to_string())),
                tool_calls: None,
                tool_call_id: None,
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: Some(Value::String("hello".to_string())),
                tool_calls: None,
                tool_call_id: None,
            },
        ]);
        let contents = openai_messages_to_contents(&req);
        assert_eq!(contents.len(), 2);
        let sys: Value = serde_json::from_str(&contents[0]).unwrap();
        assert_eq!(sys["role"], "system");
        assert_eq!(sys["content"], "You are helpful.");
        let usr: Value = serde_json::from_str(&contents[1]).unwrap();
        assert_eq!(usr["role"], "user");
        assert_eq!(usr["content"], "hello");
    }

    #[test]
    fn test_messages_to_contents_tool_role() {
        let req = make_request(vec![OpenAiMessage {
            role: "tool".to_string(),
            content: Some(Value::String("\"file contents\"".to_string())),
            tool_calls: None,
            tool_call_id: Some("call_123".to_string()),
        }]);
        let contents = openai_messages_to_contents(&req);
        assert_eq!(contents.len(), 1);
        let parsed: Value = serde_json::from_str(&contents[0]).unwrap();
        assert_eq!(parsed["role"], "user");
        assert!(parsed["content"].as_str().unwrap().contains("<tool_response>"));
        assert!(parsed["content"].as_str().unwrap().contains("call_123"));
    }

    #[test]
    fn test_build_custom_request_defaults() {
        let req = make_request(vec![OpenAiMessage {
            role: "user".to_string(),
            content: Some(Value::String("hello".to_string())),
            tool_calls: None,
            tool_call_id: None,
        }]);
        let config = ProxyConfig {
            llm_id: "test-llm".to_string(),
            ..Default::default()
        };
        let custom = build_custom_request(&req, &config);
        assert_eq!(custom.llm_id, "test-llm");
        assert!(!custom.is_stream);
        assert_eq!(custom.llm_config.temperature, 0.7);
        assert_eq!(custom.llm_config.top_p, 0.9);
        assert_eq!(custom.llm_config.max_new_token, 1024);
    }

    #[test]
    fn test_custom_response_to_openai_success() {
        let custom_resp = CustomLlmResponse {
            id: Some("resp-123".to_string()),
            status: Some("SUCCESS".to_string()),
            content: Some("Hello!".to_string()),
            response_code: None,
            prompt_token: Some(10),
            completion_token: Some(5),
        };
        let req = make_request(vec![]);
        let openai_resp = custom_response_to_openai(&custom_resp, &req);
        assert_eq!(openai_resp["choices"][0]["message"]["content"], "Hello!");
        assert_eq!(openai_resp["choices"][0]["finish_reason"], "stop");
        assert_eq!(openai_resp["usage"]["prompt_tokens"], 10);
        assert_eq!(openai_resp["usage"]["completion_tokens"], 5);
    }

    #[test]
    fn test_custom_response_to_openai_fail() {
        let custom_resp = CustomLlmResponse {
            id: None,
            status: Some("FAIL".to_string()),
            content: None,
            response_code: Some("RATE_LIMIT".to_string()),
            prompt_token: None,
            completion_token: None,
        };
        let req = make_request(vec![]);
        let openai_resp = custom_response_to_openai(&custom_resp, &req);
        assert!(openai_resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("RATE_LIMIT"));
    }

    #[test]
    fn test_custom_response_with_tool_calls() {
        let custom_resp = CustomLlmResponse {
            id: Some("resp-456".to_string()),
            status: Some("SUCCESS".to_string()),
            content: Some(
                r#"Let me check. <tool_call>{"name":"get_weather","arguments":{"city":"Seoul"}}</tool_call>"#
                    .to_string(),
            ),
            response_code: None,
            prompt_token: Some(50),
            completion_token: Some(20),
        };
        let mut req = make_request(vec![]);
        req.tools = Some(vec![serde_json::json!({
            "type": "function",
            "function": {"name": "get_weather"}
        })]);
        let openai_resp = custom_response_to_openai(&custom_resp, &req);
        assert_eq!(openai_resp["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(
            openai_resp["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "get_weather"
        );
    }

    #[test]
    fn test_extract_text_content_multimodal() {
        let content = Some(Value::Array(vec![
            serde_json::json!({"type": "text", "text": "Hello"}),
            serde_json::json!({"type": "image_url", "image_url": {"url": "..."}}),
            serde_json::json!({"type": "text", "text": "World"}),
        ]));
        let text = extract_text_content(&content);
        assert_eq!(text, "Hello\nWorld");
    }
}
