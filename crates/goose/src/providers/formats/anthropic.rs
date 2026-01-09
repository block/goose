use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{convert_image, ImageFormat};
use anyhow::{anyhow, Result};
use rmcp::model::{object, CallToolRequestParam, ErrorCode, ErrorData, JsonObject, Role, Tool};
use rmcp::object as json_object;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::Arc;

// Constants for frequently used strings in Anthropic API format
const TYPE_FIELD: &str = "type";
const CONTENT_FIELD: &str = "content";
const TEXT_TYPE: &str = "text";
const ROLE_FIELD: &str = "role";
const USER_ROLE: &str = "user";
const ASSISTANT_ROLE: &str = "assistant";
const TOOL_USE_TYPE: &str = "tool_use";
const TOOL_RESULT_TYPE: &str = "tool_result";
const THINKING_TYPE: &str = "thinking";
const REDACTED_THINKING_TYPE: &str = "redacted_thinking";
const CACHE_CONTROL_FIELD: &str = "cache_control";
const ID_FIELD: &str = "id";
const NAME_FIELD: &str = "name";
const INPUT_FIELD: &str = "input";
const TOOL_USE_ID_FIELD: &str = "tool_use_id";
const IS_ERROR_FIELD: &str = "is_error";
const SIGNATURE_FIELD: &str = "signature";
const DATA_FIELD: &str = "data";

/// Convert internal Message format to Anthropic's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    let mut anthropic_messages = Vec::new();

    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        let role = match message.role {
            Role::User => USER_ROLE,
            Role::Assistant => ASSISTANT_ROLE,
        };

        let mut content = Vec::new();
        for msg_content in &message.content {
            match msg_content {
                MessageContent::Text(text) => {
                    content.push(json!({
                        TYPE_FIELD: TEXT_TYPE,
                        TEXT_TYPE: text.text
                    }));
                }
                MessageContent::ToolRequest(tool_request) => {
                    match &tool_request.tool_call {
                        Ok(tool_call) => {
                            content.push(json!({
                                TYPE_FIELD: TOOL_USE_TYPE,
                                ID_FIELD: tool_request.id,
                                NAME_FIELD: tool_call.name,
                                INPUT_FIELD: tool_call.arguments
                            }));
                        }
                        Err(_tool_error) => {
                            // Skip malformed tool requests - they shouldn't be sent to Anthropic
                            // This maintains the existing behavior for ToolRequest errors
                        }
                    }
                }
                MessageContent::ToolResponse(tool_response) => match &tool_response.tool_result {
                    Ok(result) => {
                        let text = result
                            .content
                            .iter()
                            .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                            .collect::<Vec<_>>()
                            .join("\n");

                        content.push(json!({
                            TYPE_FIELD: TOOL_RESULT_TYPE,
                            TOOL_USE_ID_FIELD: tool_response.id,
                            CONTENT_FIELD: text
                        }));
                    }
                    Err(tool_error) => {
                        content.push(json!({
                            TYPE_FIELD: TOOL_RESULT_TYPE,
                            TOOL_USE_ID_FIELD: tool_response.id,
                            CONTENT_FIELD: format!("Error: {}", tool_error),
                            IS_ERROR_FIELD: true
                        }));
                    }
                },
                MessageContent::ToolConfirmationRequest(_tool_confirmation_request) => {
                    // Skip tool confirmation requests
                }
                MessageContent::ActionRequired(_action_required) => {
                    // Skip action required messages - they're for UI only
                }
                MessageContent::SystemNotification(_) => {
                    // Skip
                }
                MessageContent::Thinking(thinking) => {
                    content.push(json!({
                        TYPE_FIELD: THINKING_TYPE,
                        THINKING_TYPE: thinking.thinking,
                        SIGNATURE_FIELD: thinking.signature
                    }));
                }
                MessageContent::RedactedThinking(redacted) => {
                    content.push(json!({
                        TYPE_FIELD: REDACTED_THINKING_TYPE,
                        DATA_FIELD: redacted.data
                    }));
                }
                MessageContent::Image(image) => {
                    content.push(convert_image(image, &ImageFormat::Anthropic));
                }
                MessageContent::FrontendToolRequest(tool_request) => {
                    if let Ok(tool_call) = &tool_request.tool_call {
                        content.push(json!({
                            TYPE_FIELD: TOOL_USE_TYPE,
                            ID_FIELD: tool_request.id,
                            NAME_FIELD: tool_call.name,
                            INPUT_FIELD: tool_call.arguments
                        }));
                    }
                }
            }
        }

        // Skip messages with empty content
        if !content.is_empty() {
            anthropic_messages.push(json!({
                ROLE_FIELD: role,
                CONTENT_FIELD: content
            }));
        }
    }

    // If no messages, add a default one
    if anthropic_messages.is_empty() {
        anthropic_messages.push(json!({
            ROLE_FIELD: USER_ROLE,
            CONTENT_FIELD: [{
                TYPE_FIELD: TEXT_TYPE,
                TEXT_TYPE: "Ignore"
            }]
        }));
    }

    // Add "cache_control" to the last and second-to-last "user" messages.
    // During each turn, we mark the final message with cache_control so the conversation can be
    // incrementally cached. The second-to-last user message is also marked for caching with the
    // cache_control parameter, so that this checkpoint can read from the previous cache.
    let mut user_count = 0;
    for message in anthropic_messages.iter_mut().rev() {
        if message.get(ROLE_FIELD) == Some(&json!(USER_ROLE)) {
            if let Some(content) = message.get_mut(CONTENT_FIELD) {
                if let Some(content_array) = content.as_array_mut() {
                    if let Some(last_content) = content_array.last_mut() {
                        last_content.as_object_mut().unwrap().insert(
                            CACHE_CONTROL_FIELD.to_string(),
                            json!({ TYPE_FIELD: "ephemeral" }),
                        );
                    }
                }
            }
            user_count += 1;
            if user_count >= 2 {
                break;
            }
        }
    }

    anthropic_messages
}

fn anthropic_flavored_input_schema(input_schema: Arc<JsonObject>) -> Arc<JsonObject> {
    if input_schema.is_empty() {
        return Arc::new(json_object!({
            "type": "object",
        }));
    }
    input_schema
}

/// Convert internal Tool format to Anthropic's API tool specification
pub fn format_tools(tools: &[Tool]) -> Vec<Value> {
    let mut unique_tools = HashSet::new();
    let mut tool_specs = Vec::new();

    for tool in tools {
        if unique_tools.insert(tool.name.clone()) {
            tool_specs.push(json!({
                NAME_FIELD: tool.name,
                "description": tool.description,
                "input_schema": anthropic_flavored_input_schema(tool.input_schema.clone())
            }));
        }
    }

    // Add "cache_control" to the last tool spec, if any. This means that all tool definitions,
    // will be cached as a single prefix.
    if let Some(last_tool) = tool_specs.last_mut() {
        last_tool.as_object_mut().unwrap().insert(
            CACHE_CONTROL_FIELD.to_string(),
            json!({ TYPE_FIELD: "ephemeral" }),
        );
    }

    tool_specs
}

/// Convert system message to Anthropic's API system specification
pub fn format_system(system: &str) -> Value {
    json!([{
        TYPE_FIELD: TEXT_TYPE,
        TEXT_TYPE: system,
        CACHE_CONTROL_FIELD: { TYPE_FIELD: "ephemeral" }
    }])
}

/// Convert Anthropic's API response to internal Message format
pub fn response_to_message(response: &Value) -> Result<Message> {
    let content_blocks = response
        .get(CONTENT_FIELD)
        .and_then(|c| c.as_array())
        .ok_or_else(|| anyhow!("Invalid response format: missing content array"))?;

    let mut message = Message::assistant();

    for block in content_blocks {
        match block.get(TYPE_FIELD).and_then(|t| t.as_str()) {
            Some(TEXT_TYPE) => {
                if let Some(text) = block.get(TEXT_TYPE).and_then(|t| t.as_str()) {
                    message = message.with_text(text.to_string());
                }
            }
            Some(TOOL_USE_TYPE) => {
                let id = block
                    .get(ID_FIELD)
                    .and_then(|i| i.as_str())
                    .ok_or_else(|| anyhow!("Missing tool_use id"))?;
                let name = block
                    .get(NAME_FIELD)
                    .and_then(|n| n.as_str())
                    .ok_or_else(|| anyhow!("Missing tool_use name"))?
                    .to_string();
                let input = block
                    .get(INPUT_FIELD)
                    .ok_or_else(|| anyhow!("Missing tool_use input"))?;

                let tool_call = CallToolRequestParam {
                    name: name.into(),
                    arguments: Some(object(input.clone())),
                };
                message = message.with_tool_request(id, Ok(tool_call));
            }
            Some(THINKING_TYPE) => {
                let thinking = block
                    .get(THINKING_TYPE)
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| anyhow!("Missing thinking content"))?
                    .to_string();
                let signature = block
                    .get(SIGNATURE_FIELD)
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| anyhow!("Missing thinking signature"))?;
                message = message.with_thinking(thinking, signature);
            }
            Some(REDACTED_THINKING_TYPE) => {
                let data = block
                    .get(DATA_FIELD)
                    .and_then(|d| d.as_str())
                    .ok_or_else(|| anyhow!("Missing redacted_thinking data"))?;
                message = message.with_redacted_thinking(data);
            }
            _ => continue,
        }
    }

    Ok(message)
}

/// Extract usage information from Anthropic's API response
pub fn get_usage(data: &Value) -> Result<Usage> {
    // Extract usage data if available
    if let Some(usage) = data.get("usage") {
        // Get all token fields for analysis
        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cache_creation_tokens = usage
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cache_read_tokens = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // IMPORTANT: For display purposes, we want to show the ACTUAL total tokens consumed
        // The cache pricing should only affect cost calculation, not token count display
        let total_input_tokens = input_tokens + cache_creation_tokens + cache_read_tokens;

        // Convert to i32 with bounds checking
        let total_input_i32 = total_input_tokens.min(i32::MAX as u64) as i32;
        let output_tokens_i32 = output_tokens.min(i32::MAX as u64) as i32;
        let total_tokens_i32 =
            (total_input_i32 as i64 + output_tokens_i32 as i64).min(i32::MAX as i64) as i32;

        Ok(Usage::new(
            Some(total_input_i32),
            Some(output_tokens_i32),
            Some(total_tokens_i32),
        ))
    } else if data.as_object().is_some() {
        // Check if the data itself is the usage object (for message_delta events that might have usage at top level)
        let input_tokens = data
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cache_creation_tokens = data
            .get("cache_creation_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let cache_read_tokens = data
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let output_tokens = data
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // If we found any token data, process it
        if input_tokens > 0
            || cache_creation_tokens > 0
            || cache_read_tokens > 0
            || output_tokens > 0
        {
            let total_input_tokens = input_tokens + cache_creation_tokens + cache_read_tokens;

            let total_input_i32 = total_input_tokens.min(i32::MAX as u64) as i32;
            let output_tokens_i32 = output_tokens.min(i32::MAX as u64) as i32;
            let total_tokens_i32 =
                (total_input_i32 as i64 + output_tokens_i32 as i64).min(i32::MAX as i64) as i32;

            tracing::debug!("🔍 Anthropic ACTUAL token counts from direct object: input={}, output={}, total={}", 
                    total_input_i32, output_tokens_i32, total_tokens_i32);

            Ok(Usage::new(
                Some(total_input_i32),
                Some(output_tokens_i32),
                Some(total_tokens_i32),
            ))
        } else {
            tracing::debug!("🔍 Anthropic no token data found in object");
            Ok(Usage::new(None, None, None))
        }
    } else {
        tracing::debug!(
            "Failed to get usage data: {}",
            ProviderError::UsageError("No usage data found in response".to_string())
        );
        // If no usage data, return None for all values
        Ok(Usage::new(None, None, None))
    }
}

/// Create a complete request payload for Anthropic's API
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let anthropic_messages = format_messages(messages);
    let tool_specs = format_tools(tools);
    let system_spec = format_system(system);

    // Check if we have any messages to send
    if anthropic_messages.is_empty() {
        return Err(anyhow!("No valid messages to send to Anthropic API"));
    }

    // https://platform.claude.com/docs/en/about-claude/models/overview
    // 64k output tokens works for most claude models, but not old opus:
    let max_tokens = model_config.max_tokens.unwrap_or_else(|| {
        let name = &model_config.model_name;
        if name.contains("claude-3-haiku") {
            4096
        } else if name.contains("claude-opus-4-0") || name.contains("claude-opus-4-1") {
            32000
        } else {
            64000
        }
    });
    let mut payload = json!({
        "model": model_config.model_name,
        "messages": anthropic_messages,
        "max_tokens": max_tokens,
    });

    // Add system message if present
    if !system.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("system".to_string(), json!(system_spec));
    }

    // Add tools if present
    if !tool_specs.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tool_specs));
    }

    // Add temperature if specified and not using extended thinking model
    if let Some(temp) = model_config.temperature {
        payload
            .as_object_mut()
            .unwrap()
            .insert("temperature".to_string(), json!(temp));
    }

    // Add thinking parameters for claude-3-7-sonnet model
    let is_thinking_enabled = std::env::var("CLAUDE_THINKING_ENABLED").is_ok();
    if is_thinking_enabled {
        // Minimum budget_tokens is 1024
        let budget_tokens = std::env::var("CLAUDE_THINKING_BUDGET")
            .unwrap_or_else(|_| "16000".to_string())
            .parse()
            .unwrap_or(16000);

        payload
            .as_object_mut()
            .unwrap()
            .insert("max_tokens".to_string(), json!(max_tokens + budget_tokens));

        payload.as_object_mut().unwrap().insert(
            "thinking".to_string(),
            json!({
                "type": "enabled",
                "budget_tokens": budget_tokens
            }),
        );
    }
    Ok(payload)
}

/// Process streaming response from Anthropic's API
pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl futures::Stream<
    Item = anyhow::Result<(
        Option<Message>,
        Option<crate::providers::base::ProviderUsage>,
    )>,
> + 'static
where
    S: futures::Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
{
    use async_stream::try_stream;
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    struct StreamingEvent {
        #[serde(rename = "type")]
        event_type: String,
        #[serde(flatten)]
        data: Value,
    }

    try_stream! {
        let mut accumulated_text = String::new();
        let mut accumulated_tool_calls: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
        let mut current_tool_id: Option<String> = None;
        let mut final_usage: Option<crate::providers::base::ProviderUsage> = None;
        let mut message_id: Option<String> = None;

        while let Some(line_result) = stream.next().await {
            let line = line_result?;

            // Skip empty lines and non-data lines
            if line.trim().is_empty() || !line.starts_with("data: ") {
                continue;
            }

            let data_part = line.strip_prefix("data: ").unwrap_or(&line);

            // Handle end of stream
            if data_part.trim() == "[DONE]" {
                break;
            }

            // Parse the JSON event
            let event: StreamingEvent = match serde_json::from_str(data_part) {
                Ok(event) => event,
                Err(e) => {
                    tracing::debug!("Failed to parse streaming event: {} - Line: {}", e, data_part);
                    continue;
                }
            };

            match event.event_type.as_str() {
                "message_start" => {
                    // Message started, we can extract initial metadata and usage if needed
                    if let Some(message_data) = event.data.get("message") {
                        // Extract message ID
                        if let Some(id) = message_data.get("id").and_then(|v| v.as_str()) {
                            message_id = Some(id.to_string());
                        }

                        if let Some(usage_data) = message_data.get("usage") {
                            let usage = get_usage(usage_data).unwrap_or_default();
                            tracing::debug!("🔍 Anthropic message_start parsed usage: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}",
                                    usage.input_tokens, usage.output_tokens, usage.total_tokens);
                            let model = message_data.get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            final_usage = Some(crate::providers::base::ProviderUsage::new(model, usage));
                        } else {
                            tracing::debug!("🔍 Anthropic message_start has no usage data");
                        }
                    }
                    continue;
                }
                "content_block_start" => {
                    // A new content block started
                    if let Some(content_block) = event.data.get("content_block") {
                        if content_block.get("type") == Some(&json!("tool_use")) {
                            if let Some(id) = content_block.get("id").and_then(|v| v.as_str()) {
                                current_tool_id = Some(id.to_string());
                                if let Some(name) = content_block.get("name").and_then(|v| v.as_str()) {
                                    accumulated_tool_calls.insert(id.to_string(), (name.to_string(), String::new()));
                                }
                            }
                        }
                    }
                    continue;
                }
                "content_block_delta" => {
                    if let Some(delta) = event.data.get("delta") {
                        if delta.get("type") == Some(&json!("text_delta")) {
                            // Text content delta
                            if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                                accumulated_text.push_str(text);

                                // Yield partial text message with the same ID from message_start
                                let mut message = Message::new(
                                    Role::Assistant,
                                    chrono::Utc::now().timestamp(),
                                    vec![MessageContent::text(text)],
                                );
                                message.id = message_id.clone();
                                yield (Some(message), None);
                            }
                        } else if delta.get("type") == Some(&json!("input_json_delta")) {
                            // Tool input delta
                            if let Some(tool_id) = &current_tool_id {
                                if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                                    if let Some((_name, args)) = accumulated_tool_calls.get_mut(tool_id) {
                                        args.push_str(partial_json);
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }
                "content_block_stop" => {
                    // Content block finished
                    if let Some(tool_id) = current_tool_id.take() {
                        // Tool call finished, yield complete tool call
                        if let Some((name, args)) = accumulated_tool_calls.remove(&tool_id) {
                            let parsed_args = if args.is_empty() {
                                json!({})
                            } else {
                                match serde_json::from_str::<Value>(&args) {
                                    Ok(parsed) => parsed,
                                    Err(_) => {
                                        // If parsing fails, create an error tool request
                                        let error = ErrorData::new(
                                            ErrorCode::INVALID_PARAMS,
                                            format!("Could not parse tool arguments: {}", args),
                                            None,
                                        );
                                        let mut message = Message::new(
                                            Role::Assistant,
                                            chrono::Utc::now().timestamp(),
                                            vec![MessageContent::tool_request(tool_id, Err(error))],
                                        );
                                        message.id = message_id.clone();
                                        yield (Some(message), None);
                                        continue;
                                    }
                                }
                            };

                            let tool_call = CallToolRequestParam{ name: name.into(), arguments: Some(object(parsed_args)) };

                            let mut message = Message::new(
                                rmcp::model::Role::Assistant,
                                chrono::Utc::now().timestamp(),
                                vec![MessageContent::tool_request(tool_id, Ok(tool_call))],
                            );
                            message.id = message_id.clone();
                            yield (Some(message), None);
                        }
                    }
                    continue;
                }
                "message_delta" => {
                    // Message metadata delta (like stop_reason) and cumulative usage
                    tracing::debug!("🔍 Anthropic message_delta event data: {}", serde_json::to_string_pretty(&event.data).unwrap_or_else(|_| format!("{:?}", event.data)));
                    if let Some(usage_data) = event.data.get("usage") {
                        tracing::debug!("🔍 Anthropic message_delta usage data (cumulative): {}", serde_json::to_string_pretty(usage_data).unwrap_or_else(|_| format!("{:?}", usage_data)));
                        let delta_usage = get_usage(usage_data).unwrap_or_default();
                        tracing::debug!("🔍 Anthropic message_delta parsed usage: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}",
                                delta_usage.input_tokens, delta_usage.output_tokens, delta_usage.total_tokens);

                        // IMPORTANT: message_delta usage should be MERGED with existing usage, not replace it
                        // message_start has input tokens, message_delta has output tokens
                        if let Some(existing_usage) = &final_usage {
                            let merged_input = existing_usage.usage.input_tokens.or(delta_usage.input_tokens);
                            let merged_output = delta_usage.output_tokens.or(existing_usage.usage.output_tokens);
                            let merged_total = match (merged_input, merged_output) {
                                (Some(input), Some(output)) => Some(input + output),
                                (Some(input), None) => Some(input),
                                (None, Some(output)) => Some(output),
                                (None, None) => None,
                            };

                            let merged_usage = crate::providers::base::Usage::new(merged_input, merged_output, merged_total);
                            final_usage = Some(crate::providers::base::ProviderUsage::new(existing_usage.model.clone(), merged_usage));
                            tracing::debug!("🔍 Anthropic MERGED usage: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}",
                                    merged_input, merged_output, merged_total);
                        } else {
                            // No existing usage, just use delta usage
                            let model = event.data.get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            final_usage = Some(crate::providers::base::ProviderUsage::new(model, delta_usage));
                            tracing::debug!("🔍 Anthropic no existing usage, using delta usage");
                        }
                    } else {
                        tracing::debug!("🔍 Anthropic message_delta event has no usage field");
                    }
                    continue;
                }
                "message_stop" => {
                    // Message finished, extract final usage if available
                    if let Some(usage_data) = event.data.get("usage") {
                        tracing::debug!("🔍 Anthropic streaming usage data: {}", serde_json::to_string_pretty(usage_data).unwrap_or_else(|_| format!("{:?}", usage_data)));
                        let usage = get_usage(usage_data).unwrap_or_default();
                        tracing::debug!("🔍 Anthropic parsed usage: input_tokens={:?}, output_tokens={:?}, total_tokens={:?}",
                                usage.input_tokens, usage.output_tokens, usage.total_tokens);
                        let model = event.data.get("model")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        tracing::debug!("🔍 Anthropic final_usage created with model: {}", model);
                        final_usage = Some(crate::providers::base::ProviderUsage::new(model, usage));
                    } else {
                        tracing::debug!("🔍 Anthropic message_stop event has no usage data");
                    }
                    break;
                }
                _ => {
                    // Unknown event type, log and continue
                    tracing::debug!("Unknown streaming event type: {}", event.event_type);
                    continue;
                }
            }
        }

        // Yield final usage information if available
        if let Some(usage) = final_usage {
            yield (None, Some(usage));
        } else {
            tracing::debug!("🔍 Anthropic no final usage to yield");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::object;
    use serde_json::json;

    #[test]
    fn test_parse_text_response() -> Result<()> {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Hello! How can I assist you today?"
            }],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15,
                "cache_creation_input_tokens": 12,
                "cache_read_input_tokens": 0
            }
        });

        let message = response_to_message(&response)?;
        let usage = get_usage(&response)?;

        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello! How can I assist you today?");
        } else {
            panic!("Expected Text content");
        }

        assert_eq!(usage.input_tokens, Some(24)); // 12 + 12 = 24 actual tokens
        assert_eq!(usage.output_tokens, Some(15));
        assert_eq!(usage.total_tokens, Some(39)); // 24 + 15

        Ok(())
    }

    #[test]
    fn test_parse_tool_response() -> Result<()> {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "tool_use",
                "id": "tool_1",
                "name": "calculator",
                "input": {
                    "expression": "2 + 2"
                }
            }],
            "model": "claude-3-sonnet-20240229",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 15,
                "output_tokens": 20,
                "cache_creation_input_tokens": 15,
                "cache_read_input_tokens": 0,
            }
        });

        let message = response_to_message(&response)?;
        let usage = get_usage(&response)?;

        if let MessageContent::ToolRequest(tool_request) = &message.content[0] {
            let tool_call = tool_request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "calculator");
            assert_eq!(tool_call.arguments, Some(object!({"expression": "2 + 2"})));
        } else {
            panic!("Expected ToolRequest content");
        }

        assert_eq!(usage.input_tokens, Some(30)); // 15 + 15 = 30 actual tokens
        assert_eq!(usage.output_tokens, Some(20));
        assert_eq!(usage.total_tokens, Some(50)); // 30 + 20

        Ok(())
    }

    #[test]
    fn test_parse_thinking_response() -> Result<()> {
        let response = json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "thinking",
                    "thinking": "This is a step-by-step thought process...",
                    "signature": "EuYBCkQYAiJAVbJNBoH7HQiDcMwwAMhWqNyoe4G2xHRprK8ICM8gZzu16i7Se4EiEbmlKqNH1GtwcX1BMK6iLu8bxWn5wPVIFBIMnptdlVal7ZX5iNPFGgwWjX+BntcEOHky4HciMFVef7FpQeqnuiL1Xt7J4OLHZSyu4tcr809AxAbclcJ5dm1xE5gZrUO+/v60cnJM2ipQp4B8/3eHI03KSV6bZR/vMrBSYCV+aa/f5KHX2cRtLGp/Ba+3Tk/efbsg01WSduwAIbR4coVrZLnGJXNyVTFW/Be2kLy/ECZnx8cqvU3oQOg="
                },
                {
                    "type": "redacted_thinking",
                    "data": "EmwKAhgBEgy3va3pzix/LafPsn4aDFIT2Xlxh0L5L8rLVyIwxtE3rAFBa8cr3qpP"
                },
                {
                    "type": "text",
                    "text": "I've analyzed the problem and here's the solution."
                }
            ],
            "model": "claude-3-7-sonnet-20250219",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 45,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
            }
        });

        let message = response_to_message(&response)?;
        let usage = get_usage(&response)?;

        assert_eq!(message.content.len(), 3);

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(
                thinking.thinking,
                "This is a step-by-step thought process..."
            );
            assert!(thinking
                .signature
                .starts_with("EuYBCkQYAiJAVbJNBoH7HQiDcMwwAMhWqNyoe4G2xHRprK8ICM8g"));
        } else {
            panic!("Expected Thinking content at index 0");
        }

        if let MessageContent::RedactedThinking(redacted) = &message.content[1] {
            assert_eq!(
                redacted.data,
                "EmwKAhgBEgy3va3pzix/LafPsn4aDFIT2Xlxh0L5L8rLVyIwxtE3rAFBa8cr3qpP"
            );
        } else {
            panic!("Expected RedactedThinking content at index 1");
        }

        if let MessageContent::Text(text) = &message.content[2] {
            assert_eq!(
                text.text,
                "I've analyzed the problem and here's the solution."
            );
        } else {
            panic!("Expected Text content at index 2");
        }

        assert_eq!(usage.input_tokens, Some(10));
        assert_eq!(usage.output_tokens, Some(45));
        assert_eq!(usage.total_tokens, Some(55));

        Ok(())
    }

    #[test]
    fn test_message_to_anthropic_spec() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there"),
            Message::user().with_text("How are you?"),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 3);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"][0]["type"], "text");
        assert_eq!(spec[0]["content"][0]["text"], "Hello");
        assert_eq!(spec[1]["role"], "assistant");
        assert_eq!(spec[1]["content"][0]["text"], "Hi there");
        assert_eq!(spec[2]["role"], "user");
        assert_eq!(spec[2]["content"][0]["text"], "How are you?");
    }

    #[test]
    fn test_tools_to_anthropic_spec() {
        let tools = vec![
            Tool::new(
                "calculator",
                "Calculate mathematical expressions",
                object!({
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "The mathematical expression to evaluate"
                        }
                    }
                }),
            ),
            Tool::new(
                "weather",
                "Get weather information",
                object!({
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "The location to get weather for"
                        }
                    }
                }),
            ),
        ];

        let spec = format_tools(&tools);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["name"], "calculator");
        assert_eq!(spec[0]["description"], "Calculate mathematical expressions");
        assert_eq!(spec[1]["name"], "weather");
        assert_eq!(spec[1]["description"], "Get weather information");

        // Verify cache control is added to last tool
        assert!(spec[1].get("cache_control").is_some());
    }

    #[test]
    fn test_system_to_anthropic_spec() {
        let system = "You are a helpful assistant.";
        let spec = format_system(system);

        assert!(spec.is_array());
        let spec_array = spec.as_array().unwrap();
        assert_eq!(spec_array.len(), 1);
        assert_eq!(spec_array[0]["type"], "text");
        assert_eq!(spec_array[0]["text"], system);
        assert!(spec_array[0].get("cache_control").is_some());
    }

    #[test]
    fn test_cache_pricing_calculation() -> Result<()> {
        // Test realistic cache scenario: small fresh input, large cached content
        let response = json!({
            "id": "msg_cache_test",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "text",
                "text": "Based on the cached context, here's my response."
            }],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 7,        // Small fresh input
                "output_tokens": 50,      // Output tokens
                "cache_creation_input_tokens": 10000, // Large cache creation
                "cache_read_input_tokens": 5000       // Large cache read
            }
        });

        let usage = get_usage(&response)?;

        // ACTUAL input tokens should be:
        // 7 + 10000 + 5000 = 15007 total actual tokens
        assert_eq!(usage.input_tokens, Some(15007));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(15057)); // 15007 + 50

        Ok(())
    }

    #[test]
    fn test_tool_error_handling_maintains_pairing() {
        use crate::conversation::message::Message;
        use rmcp::model::{ErrorCode, ErrorData};

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParam {
                    name: "calculator".into(),
                    arguments: Some(object!({"expression": "2 + 2"})),
                }),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Tool failed".to_string(),
                    None,
                )),
            ),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 2);

        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"][0]["type"], "tool_use");
        assert_eq!(spec[0]["content"][0]["id"], "tool_1");
        assert_eq!(spec[0]["content"][0]["name"], "calculator");

        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"][0]["type"], "tool_result");
        assert_eq!(spec[1]["content"][0]["tool_use_id"], "tool_1");
        assert_eq!(
            spec[1]["content"][0]["content"],
            "Error: -32603: Tool failed"
        );
        assert_eq!(spec[1]["content"][0]["is_error"], true);
    }
}
