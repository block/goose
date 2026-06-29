use crate::canonical::maybe_get_canonical_model;
use crate::canonical::ThinkingMode;
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::token_usage::{ProviderUsage, Usage};
use crate::errors::ProviderError;
use crate::images::{convert_image, ImageFormat};
use crate::mcp_utils::extract_text_from_resource;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use anyhow::{anyhow, Result};
use rmcp::model::{object, CallToolRequestParams, ErrorCode, ErrorData, JsonObject, Role, Tool};
use rmcp::object as json_object;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

pub const ANTHROPIC_PROVIDER_NAME: &str = "anthropic";

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $str:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    $($str => Ok(Self::$variant),)+
                    other => Err(format!("unknown {}: '{other}'", stringify!($name))),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self { $(Self::$variant => write!(f, $str),)+ }
            }
        }
    }
}

string_enum!(ThinkingType { Adaptive => "adaptive", Enabled => "enabled", Disabled => "disabled" });

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnthropicFormatOptions {
    pub preserve_unsigned_thinking: bool,
    pub preserve_thinking_context: bool,
    pub thinking_disabled: bool,
}

impl AnthropicFormatOptions {
    fn for_model(self, model_config: &ModelConfig) -> Self {
        let preserve_thinking_context = model_config
            .request_param::<bool>("preserve_thinking_context")
            .unwrap_or(self.preserve_thinking_context);
        let preserve_unsigned_thinking = model_config
            .request_param::<bool>("preserve_unsigned_thinking")
            .unwrap_or(self.preserve_unsigned_thinking)
            || preserve_thinking_context;
        let thinking_disabled = model_config.reasoning == Some(false)
            || model_config.thinking_effort() == Some(ThinkingEffort::Off);

        Self {
            preserve_unsigned_thinking,
            preserve_thinking_context,
            thinking_disabled,
        }
    }
}

fn canonical_thinking_mode(provider_name: &str, model_name: &str) -> Option<ThinkingMode> {
    maybe_get_canonical_model(provider_name, model_name).and_then(|model| model.thinking_mode)
}

fn canonical_reasoning(provider_name: &str, model_config: &ModelConfig) -> Option<bool> {
    maybe_get_canonical_model(provider_name, &model_config.model_name)
        .and_then(|model| model.reasoning)
}

pub fn model_supports_temperature(provider_name: &str, model_config: &ModelConfig) -> bool {
    maybe_get_canonical_model(provider_name, &model_config.model_name)
        .and_then(|model| model.temperature)
        .unwrap_or(true)
}

pub fn thinking_type(model_config: &ModelConfig) -> ThinkingType {
    thinking_type_for_provider(ANTHROPIC_PROVIDER_NAME, model_config)
}

pub fn thinking_type_for_provider(provider_name: &str, model_config: &ModelConfig) -> ThinkingType {
    let mode = canonical_thinking_mode(provider_name, &model_config.model_name);
    let reasoning = model_config
        .reasoning
        .or_else(|| canonical_reasoning(provider_name, model_config));

    if reasoning != Some(true) {
        return ThinkingType::Disabled;
    }

    if mode == Some(ThinkingMode::AlwaysOnAdaptive) {
        return ThinkingType::Adaptive;
    }

    let effort = model_config.thinking_effort();

    if effort.is_none() && model_config.request_param::<i32>("budget_tokens").is_some() {
        return match mode {
            Some(ThinkingMode::Adaptive) => ThinkingType::Adaptive,
            _ => ThinkingType::Enabled,
        };
    }

    match effort.unwrap_or(ThinkingEffort::Off) {
        ThinkingEffort::Off => ThinkingType::Disabled,
        _ if mode == Some(ThinkingMode::Adaptive) => ThinkingType::Adaptive,
        _ => ThinkingType::Enabled,
    }
}

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
const EVENT_MESSAGE_START: &str = "message_start";
const EVENT_MESSAGE_DELTA: &str = "message_delta";
const EVENT_MESSAGE_STOP: &str = "message_stop";
const EVENT_CONTENT_BLOCK_START: &str = "content_block_start";
const EVENT_CONTENT_BLOCK_DELTA: &str = "content_block_delta";
const EVENT_CONTENT_BLOCK_STOP: &str = "content_block_stop";
const STOP_REASON_REFUSAL: &str = "refusal";
const REFUSAL_FALLBACK_DETAILS: &str = "No additional details were provided.";

/// Coerce a tool call's optional arguments into the JSON value Anthropic
/// expects for the `input` field of a `tool_use` content block.
///
/// Anthropic's Messages API requires `input` to be an object. When the
/// internal `CallToolRequestParams::arguments` is `None` (which happens for
/// parameterless tools, tool calls round-tripped from disk, or calls created
/// via `CallToolRequestParams::new` without `.with_arguments(...)`) the
/// `json!` macro would otherwise serialize it as JSON `null` and the API
/// rejects the next replay of the tool_use block with a 400 error:
/// `messages.<N>.content.<M>.tool_use.input: Input should be an object.`
/// See issue #9287.
fn args_to_input_value(arguments: Option<JsonObject>) -> Value {
    Value::Object(arguments.unwrap_or_default())
}

/// Convert internal Message format to Anthropic's API message specification
pub fn format_messages(messages: &[Message]) -> Vec<Value> {
    format_messages_with_options(messages, AnthropicFormatOptions::default())
}

fn format_messages_with_options(
    messages: &[Message],
    options: AnthropicFormatOptions,
) -> Vec<Value> {
    let mut anthropic_messages = Vec::new();

    for message in messages {
        let role = match message.role {
            Role::User => USER_ROLE,
            Role::Assistant => ASSISTANT_ROLE,
        };

        let mut content = Vec::new();
        for msg_content in &message.content {
            match msg_content {
                MessageContent::Text(text) => {
                    if !text.text.trim().is_empty() {
                        content.push(json!({
                            TYPE_FIELD: TEXT_TYPE,
                            TEXT_TYPE: text.text
                        }));
                    }
                }
                MessageContent::ToolRequest(tool_request) => {
                    match &tool_request.tool_call {
                        Ok(tool_call) => {
                            content.push(json!({
                                TYPE_FIELD: TOOL_USE_TYPE,
                                ID_FIELD: tool_request.id,
                                NAME_FIELD: tool_call.name,
                                INPUT_FIELD: args_to_input_value(tool_call.arguments.clone())
                            }));
                        }
                        Err(_tool_error) => {
                            // The paired tool response carries the parse error and
                            // serializes to a tool_result below; Anthropic rejects a
                            // tool_result without a preceding tool_use, so emit a
                            // placeholder tool_use with the same id to keep history valid.
                            content.push(json!({
                                TYPE_FIELD: TOOL_USE_TYPE,
                                ID_FIELD: tool_request.id,
                                NAME_FIELD: "unparseable_tool_call",
                                INPUT_FIELD: json!({})
                            }));
                        }
                    }
                }
                MessageContent::ToolResponse(tool_response) => match &tool_response.tool_result {
                    Ok(result) => {
                        let text = result
                            .content
                            .iter()
                            .filter_map(|c| {
                                if let Some(t) = c.as_text() {
                                    return Some(t.text.clone());
                                }
                                if let Some(r) = c.as_resource() {
                                    let text = extract_text_from_resource(&r.resource);
                                    if !text.is_empty() {
                                        return Some(text);
                                    }
                                }
                                None
                            })
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
                    // Anthropic rejects thinking blocks sent without a matching thinking config.
                    if !options.thinking_disabled {
                        if !thinking.signature.is_empty() {
                            content.push(json!({
                                TYPE_FIELD: THINKING_TYPE,
                                THINKING_TYPE: thinking.thinking,
                                SIGNATURE_FIELD: thinking.signature
                            }));
                        } else if options.preserve_unsigned_thinking
                            && !thinking.thinking.is_empty()
                        {
                            content.push(json!({
                                TYPE_FIELD: THINKING_TYPE,
                                THINKING_TYPE: thinking.thinking
                            }));
                        }
                    }
                }
                MessageContent::RedactedThinking(redacted) => {
                    if !options.thinking_disabled {
                        content.push(json!({
                            TYPE_FIELD: REDACTED_THINKING_TYPE,
                            DATA_FIELD: redacted.data
                        }));
                    }
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
                            INPUT_FIELD: args_to_input_value(tool_call.arguments.clone())
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

                let tool_call =
                    CallToolRequestParams::new(name).with_arguments(object(input.clone()));
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
                    .unwrap_or_default();
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

fn usage_from_anthropic_fields(usage: &Value) -> Usage {
    let field = |key: &str| {
        usage
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v.min(i32::MAX as u64) as i32)
    };

    Usage::from_cache_exclusive_input(
        Some(field("input_tokens").unwrap_or(0)),
        Some(field("output_tokens").unwrap_or(0)),
        None,
        field("cache_read_input_tokens"),
        field("cache_creation_input_tokens"),
    )
}

/// Merge a `message_delta` usage into the usage captured at `message_start`.
/// Delta usage is cumulative (input grows during server tool use), so fields
/// present in the raw delta payload win over the start values.
fn merge_delta_usage(existing: &Usage, delta: &Usage, delta_data: &Value) -> Usage {
    let reports = |key: &str| delta_data.get(key).is_some();

    let output = if reports("output_tokens") {
        delta.output_tokens
    } else {
        existing.output_tokens
    };

    if !reports("input_tokens") {
        Usage::new(existing.input_tokens, output, None).with_cache_tokens(
            existing.cache_read_input_tokens,
            existing.cache_write_input_tokens,
        )
    } else if reports("cache_read_input_tokens") || reports("cache_creation_input_tokens") {
        Usage::new(delta.input_tokens, output, None).with_cache_tokens(
            delta.cache_read_input_tokens,
            delta.cache_write_input_tokens,
        )
    } else {
        Usage::from_cache_exclusive_input(
            delta.input_tokens,
            output,
            None,
            existing.cache_read_input_tokens,
            existing.cache_write_input_tokens,
        )
    }
}

pub fn get_usage(data: &Value) -> Result<Usage> {
    if let Some(usage) = data.get("usage") {
        Ok(usage_from_anthropic_fields(usage))
    } else if data.as_object().is_some() {
        // Check if the data itself is the usage object (for message_delta events that might have usage at top level)
        let usage = usage_from_anthropic_fields(data);
        if usage.total_tokens.unwrap_or(0) > 0 {
            Ok(usage)
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

pub fn thinking_effort(model_config: &ModelConfig) -> ThinkingEffort {
    model_config
        .thinking_effort()
        .unwrap_or(ThinkingEffort::High)
}

pub fn adaptive_output_effort(model_config: &ModelConfig) -> ThinkingEffort {
    match thinking_effort(model_config) {
        ThinkingEffort::Off => ThinkingEffort::High,
        effort => effort,
    }
}

pub fn thinking_budget_tokens(model_config: &ModelConfig) -> i32 {
    if let Some(request_param) = model_config
        .request_params
        .as_ref()
        .and_then(|params| params.get("budget_tokens"))
        .and_then(|v| serde_json::from_value::<i32>(v.clone()).ok())
    {
        return request_param.max(1024);
    }

    let effort = model_config
        .thinking_effort()
        .unwrap_or(ThinkingEffort::High);
    match effort {
        ThinkingEffort::Off => 1024,
        ThinkingEffort::Low => 4000,
        ThinkingEffort::Medium => 10000,
        ThinkingEffort::High => 16000,
        ThinkingEffort::Max => 32000,
    }
}

// Anthropic counts thinking tokens against max_tokens, so the budget must leave
// room for a response. Clamp it to preserve at least this many answer tokens, and
// drop thinking only when even a minimal budget wouldn't fit under the cap.
const MIN_ANSWER_TOKENS: i32 = 1024;

fn apply_thinking_config(
    payload: &mut Value,
    provider_name: &str,
    model_config: &ModelConfig,
    max_tokens: i32,
    options: AnthropicFormatOptions,
) {
    let obj = payload.as_object_mut().unwrap();
    match thinking_type_for_provider(provider_name, model_config) {
        ThinkingType::Adaptive => {
            obj.insert("thinking".to_string(), json!({"type": "adaptive"}));
            let effort = adaptive_output_effort(model_config).to_string();
            obj.insert("output_config".to_string(), json!({"effort": effort}));
        }
        ThinkingType::Enabled => {
            let budget_tokens = thinking_budget_tokens(model_config)
                .min(max_tokens.saturating_sub(MIN_ANSWER_TOKENS));
            if budget_tokens >= MIN_ANSWER_TOKENS {
                obj.insert(
                    "thinking".to_string(),
                    json!({
                        "type": "enabled",
                        "budget_tokens": budget_tokens
                    }),
                );
            }
        }
        ThinkingType::Disabled => {}
    }

    if options.preserve_thinking_context && !options.thinking_disabled {
        if !obj.contains_key("thinking") {
            let budget_tokens = thinking_budget_tokens(model_config)
                .min(max_tokens.saturating_sub(MIN_ANSWER_TOKENS));
            if budget_tokens >= MIN_ANSWER_TOKENS {
                obj.insert(
                    "thinking".to_string(),
                    json!({
                        "type": "enabled",
                        "budget_tokens": budget_tokens
                    }),
                );
            }
        }

        if let Some(thinking) = obj.get_mut("thinking").and_then(|t| t.as_object_mut()) {
            thinking.insert("clear_thinking".to_string(), json!(false));
        }
    }
}

pub fn create_request(
    provider_name: &str,
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    options: AnthropicFormatOptions,
) -> Result<Value> {
    let options = options.for_model(model_config);
    let anthropic_messages = format_messages_with_options(messages, options);
    let tool_specs = format_tools(tools);
    let system_spec = format_system(system);

    if anthropic_messages.is_empty() {
        return Err(anyhow!("No valid messages to send to Anthropic API"));
    }

    let max_tokens = model_config.max_output_tokens();
    let mut payload = json!({
        "model": model_config.model_name,
        "messages": anthropic_messages,
        "max_tokens": max_tokens,
    });

    if !system.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("system".to_string(), json!(system_spec));
    }

    if !tool_specs.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tool_specs));
    }

    if model_supports_temperature(provider_name, model_config) {
        if let Some(temp) = model_config.temperature {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
        }
    }

    apply_thinking_config(
        &mut payload,
        provider_name,
        model_config,
        max_tokens,
        options,
    );

    Ok(payload)
}

/// Process streaming response from Anthropic's API
pub fn response_to_streaming_message<S>(
    mut stream: S,
) -> impl futures::Stream<Item = anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> + 'static
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

    #[derive(Deserialize, Debug)]
    #[serde(tag = "type", rename_all = "snake_case")]
    #[allow(clippy::enum_variant_names)]
    enum ContentDelta {
        TextDelta { text: String },
        InputJsonDelta { partial_json: String },
        ThinkingDelta { thinking: String },
        SignatureDelta { signature: String },
    }

    struct ThinkingState {
        text: String,
        signature: String,
    }

    try_stream! {
        let mut accumulated_tool_calls: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
        let mut current_tool_id: Option<String> = None;
        let mut final_usage: Option<ProviderUsage> = None;
        let mut message_id: Option<String> = None;
        let mut thinking: Option<ThinkingState> = None;
        let mut stop_reason: Option<String> = None;

        while let Some(line_result) = stream.next().await {
            let line = line_result?;

            // Skip empty lines and non-data lines
            // Note: SSE spec allows both "data: value" and "data:value" (space is optional)
            if line.trim().is_empty() || !line.starts_with("data:") {
                continue;
            }

            let data_part = line.strip_prefix("data: ").or_else(|| line.strip_prefix("data:")).unwrap_or(&line);

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
                EVENT_MESSAGE_START => {
                    if let Some(message_data) = event.data.get("message") {
                        if let Some(id) = message_data.get("id").and_then(|v| v.as_str()) {
                            message_id = Some(id.to_string());
                        }

                        if let Some(usage_data) = message_data.get("usage") {
                            let usage = get_usage(usage_data).unwrap_or_default();
                            let model = message_data.get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            final_usage = Some(ProviderUsage::new(model, usage));
                        }
                    }
                    continue;
                }
                EVENT_CONTENT_BLOCK_START => {
                    if let Some(content_block) = event.data.get("content_block") {
                        match content_block.get(TYPE_FIELD).and_then(|v| v.as_str()) {
                            Some(TOOL_USE_TYPE) => {
                                if let Some(id) = content_block.get("id").and_then(|v| v.as_str()) {
                                    current_tool_id = Some(id.to_string());
                                    if let Some(name) = content_block.get("name").and_then(|v| v.as_str()) {
                                        accumulated_tool_calls.insert(id.to_string(), (name.to_string(), String::new()));
                                    }
                                }
                            }
                            Some(THINKING_TYPE) => {
                                thinking = Some(ThinkingState {
                                    text: content_block
                                        .get(THINKING_TYPE)
                                        .and_then(|t| t.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    signature: content_block
                                        .get(SIGNATURE_FIELD)
                                        .and_then(|s| s.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                });
                            }
                            Some(REDACTED_THINKING_TYPE) => {
                                if let Some(data) = content_block.get(DATA_FIELD).and_then(|d| d.as_str()) {
                                    let mut message = Message::assistant()
                                        .with_redacted_thinking(data);
                                    message.id = message_id.clone();
                                    yield (Some(message), None);
                                } else {
                                    tracing::warn!("redacted_thinking block missing '{}' field", DATA_FIELD);
                                }
                            }
                            _ => {}
                        }
                    }
                    continue;
                }
                EVENT_CONTENT_BLOCK_DELTA => {
                    if let Some(delta) = event.data.get("delta") {
                        match serde_json::from_value::<ContentDelta>(delta.clone()) {
                            Ok(ContentDelta::TextDelta { text }) => {
                                let mut message = Message::assistant().with_text(&text);
                                message.id = message_id.clone();
                                yield (Some(message), None);
                            }
                            Ok(ContentDelta::InputJsonDelta { partial_json }) => {
                                if let Some(tool_id) = &current_tool_id {
                                    if let Some((_name, args)) = accumulated_tool_calls.get_mut(tool_id) {
                                        args.push_str(&partial_json);
                                    }
                                }
                            }
                            Ok(ContentDelta::ThinkingDelta { thinking: t }) => {
                                if let Some(ref mut state) = thinking {
                                    state.text.push_str(&t);
                                }
                            }
                            Ok(ContentDelta::SignatureDelta { signature: s }) => {
                                if let Some(ref mut state) = thinking {
                                    state.signature.push_str(&s);
                                }
                            }
                            Err(e) => {
                                tracing::debug!("Unknown content_block_delta type: {}", e);
                            }
                        }
                    }
                    continue;
                }
                EVENT_CONTENT_BLOCK_STOP => {
                    if let Some(state) = thinking.take() {
                        if !state.text.is_empty() {
                            let mut message = Message::assistant()
                                .with_thinking(state.text, state.signature);
                            message.id = message_id.clone();
                            yield (Some(message), None);
                        }
                    }
                    if let Some(tool_id) = current_tool_id.take() {
                        if let Some((name, args)) = accumulated_tool_calls.remove(&tool_id) {
                            let parsed_args = if args.is_empty() {
                                json!({})
                            } else {
                                match crate::json::parse_tool_arguments(&args) {
                                    Some(parsed) => parsed,
                                    None => {
                                        let message_text = crate::json::truncation_error_message(&args)
                                            .unwrap_or_else(|| {
                                                format!("Could not parse tool arguments: {args}")
                                            });
                                        let error = ErrorData::new(
                                            ErrorCode::INVALID_PARAMS,
                                            message_text,
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

                            let tool_call = CallToolRequestParams::new(name).with_arguments(object(parsed_args));

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
                EVENT_MESSAGE_DELTA => {
                    if let Some(usage_data) = event.data.get("usage") {
                        let delta_usage = get_usage(usage_data).unwrap_or_default();

                        if let Some(existing_usage) = &final_usage {
                            let merged_usage = merge_delta_usage(&existing_usage.usage, &delta_usage, usage_data);
                            final_usage = Some(ProviderUsage::new(existing_usage.model.clone(), merged_usage));
                        } else {
                            let model = event.data.get("model")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            final_usage = Some(ProviderUsage::new(model, delta_usage));
                        }
                    }
                    if let Some(delta) = event.data.get("delta") {
                        let stop_details = delta.get("stop_details").filter(|d| !d.is_null());
                        if stop_reason.is_none() {
                            if let Some(sr) = delta.get("stop_reason").and_then(|v| v.as_str()) {
                                stop_reason = Some(sr.to_string());
                            }
                        }
                        if delta.get("stop_reason").and_then(|v| v.as_str()) == Some(STOP_REASON_REFUSAL) {
                            let str_field = |key: &str| stop_details
                                .and_then(|d| d.get(key))
                                .and_then(|v| v.as_str())
                                .map(str::to_string);
                            let details = str_field("explanation")
                                .or_else(|| stop_details.map(|d| d.to_string()))
                                .unwrap_or_else(|| REFUSAL_FALLBACK_DETAILS.to_string());
                            let category = str_field("category");
                            // The refusal delta carries the request's usage;
                            // flush it so refused turns are still accounted.
                            if let Some(usage) = final_usage.take() {
                                yield (None, Some(usage));
                            }
                            Err(ProviderError::Refusal { details, category })?;
                        } else if let Some(details) = stop_details {
                            // No specific handling for these stop details yet —
                            // forward them rather than silently dropping the turn.
                            let mut message = Message::assistant().with_text(format!(
                                "The provider ended the response with: {details}"
                            ));
                            message.id = message_id.clone();
                            yield (Some(message), None);
                        }
                    }
                    continue;
                }
                EVENT_MESSAGE_STOP => {
                    if let Some(usage_data) = event.data.get("usage") {
                        let usage = get_usage(usage_data).unwrap_or_default();
                        let model = event.data.get("model")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        final_usage = Some(ProviderUsage::new(model, usage));
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

        // A tool_use block left open at stream end never received its
        // content_block_stop, so its args are truncated rather than complete.
        if !accumulated_tool_calls.is_empty() {
            let truncated_by_limit = stop_reason.as_deref() == Some("max_tokens");
            let mut ids: Vec<String> = accumulated_tool_calls.keys().cloned().collect();
            ids.sort();
            for id in ids {
                if let Some((_name, args)) = accumulated_tool_calls.remove(&id) {
                    let guidance = if truncated_by_limit {
                        "The model's response was truncated — it hit the output token limit while generating this tool call. \
                         Try increasing max_tokens for this provider or breaking the task into smaller steps."
                    } else {
                        "A tool call was not completed before the stream ended. \
                         Try resending your message or breaking the task into smaller steps."
                    };
                    let snippet_len = args.chars().count();
                    let tail: String = args.chars().rev().take(80).collect::<Vec<_>>().into_iter().rev().collect();
                    let message_text = format!(
                        "{guidance}\nReceived {snippet_len} characters of arguments; cut off at: …{tail}"
                    );
                    let error = ErrorData::new(ErrorCode::INVALID_PARAMS, message_text, None);
                    let mut message = Message::new(
                        Role::Assistant,
                        chrono::Utc::now().timestamp(),
                        vec![MessageContent::tool_request(id, Err(error))],
                    );
                    message.id = message_id.clone();
                    yield (Some(message), None);
                }
            }
        }

        if let Some(usage) = final_usage {
            yield (None, Some(usage));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use crate::model::ModelConfig;
    use rmcp::object;
    use serde_json::json;

    /// Create a complete request payload for Anthropic's API
    fn create_request_with_default_options(
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<Value> {
        create_request_with_options_provider(
            model_config,
            system,
            messages,
            tools,
            AnthropicFormatOptions::default(),
        )
    }

    fn create_request_with_options_provider(
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        options: AnthropicFormatOptions,
    ) -> Result<Value> {
        create_request(
            ANTHROPIC_PROVIDER_NAME,
            model_config,
            system,
            messages,
            tools,
            options,
        )
    }

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
        assert_eq!(usage.cache_read_input_tokens, Some(0));
        assert_eq!(usage.cache_write_input_tokens, Some(12));

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
        assert_eq!(usage.cache_read_input_tokens, Some(0));
        assert_eq!(usage.cache_write_input_tokens, Some(15));

        Ok(())
    }

    #[test]
    fn test_parse_unsigned_thinking_response() -> Result<()> {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{
                "type": "thinking",
                "thinking": "internal reasoning"
            }],
            "model": "glm-4.7",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 12,
                "output_tokens": 15
            }
        });

        let message = response_to_message(&response)?;

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "internal reasoning");
            assert_eq!(thinking.signature, "");
        } else {
            panic!("Expected Thinking content");
        }

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
    fn test_message_to_anthropic_spec_skips_unsigned_thinking() {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::assistant().with_text("Hi there"),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"][0]["type"], "text");
        assert_eq!(spec[0]["content"][0]["text"], "Hi there");
    }

    #[test]
    fn test_message_to_anthropic_spec_preserves_unsigned_thinking_when_enabled() {
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::assistant().with_text("Hi there"),
        ];

        let spec = format_messages_with_options(
            &messages,
            AnthropicFormatOptions {
                preserve_unsigned_thinking: true,
                preserve_thinking_context: false,
                thinking_disabled: false,
            },
        );

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"][0]["type"], "thinking");
        assert_eq!(spec[0]["content"][0]["thinking"], "internal");
        assert!(spec[0]["content"][0].get("signature").is_none());
        assert_eq!(spec[1]["content"][0]["text"], "Hi there");
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
        assert_eq!(usage.cache_read_input_tokens, Some(5000));
        assert_eq!(usage.cache_write_input_tokens, Some(10000));

        Ok(())
    }

    #[test]
    fn test_create_request_adaptive_thinking_for_46_models() -> Result<()> {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!("high"));

        let mut config = cfg("claude-opus-4-6");
        config.max_tokens = Some(4096);
        config.request_params = Some(params);
        let messages = vec![Message::user().with_text("Hello")];
        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["type"], "adaptive");
        assert_eq!(payload["output_config"]["effort"], "high");
        assert!(payload.get("budget_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_enabled_thinking_with_budget() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("GOOSE_THINKING_EFFORT", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ]);

        let mut config = cfg_with_effort("claude-3-7-sonnet-20250219", "high");
        config.max_tokens = Some(64000);

        let messages = vec![Message::user().with_text("Hello")];
        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["type"], "enabled");
        let budget = payload["thinking"]["budget_tokens"].as_i64().unwrap();
        assert!(budget > 0);
        assert_eq!(payload["max_tokens"], 64000);
        assert!(budget < 64000);

        Ok(())
    }

    #[test]
    fn test_create_request_clamps_thinking_budget_to_fit_max_tokens() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("GOOSE_THINKING_EFFORT", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ]);

        let mut config = cfg_with_effort("claude-3-7-sonnet-20250219", "high");
        let messages = vec![Message::user().with_text("Hello")];

        // Budget larger than max_tokens is clamped to leave room for a response.
        config.max_tokens = Some(4096);
        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;
        let budget = payload["thinking"]["budget_tokens"].as_i64().unwrap();
        assert!(budget >= 1024);
        assert!(budget <= 4096 - 1024);
        assert_eq!(payload["max_tokens"], 4096);

        // Too small to fit any thinking alongside a response — drop it.
        config.max_tokens = Some(1500);
        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;
        assert!(payload.get("thinking").is_none());
        assert_eq!(payload["max_tokens"], 1500);

        Ok(())
    }

    #[test]
    fn test_create_request_disabled_thinking_no_thinking_field() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("GOOSE_THINKING_EFFORT", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
        ]);

        let config = cfg_with_effort("claude-sonnet-4-20250514", "off");
        let messages = vec![Message::user().with_text("Hello")];
        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

        assert!(payload.get("thinking").is_none());
        assert!(payload.get("output_config").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_preserves_thinking_context_for_compatible_models() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("CLAUDE_THINKING_ENABLED", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
            ("ANTHROPIC_PRESERVE_UNSIGNED_THINKING", None::<&str>),
        ]);

        let mut config = cfg("glm-4.7");
        config.max_tokens = Some(64000);
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::user().with_text("Continue"),
        ];

        let payload = create_request_with_options_provider(
            &config,
            "system",
            &messages,
            &[],
            AnthropicFormatOptions {
                preserve_unsigned_thinking: true,
                preserve_thinking_context: true,
                thinking_disabled: false,
            },
        )?;

        assert_eq!(payload["thinking"]["type"], "enabled");
        assert!(payload["thinking"]["budget_tokens"].as_i64().unwrap() >= 1024);
        assert_eq!(payload["thinking"]["clear_thinking"], false);
        assert_eq!(payload["max_tokens"], 64000);
        assert_eq!(payload["messages"][0]["content"][0]["type"], "thinking");
        assert_eq!(payload["messages"][0]["content"][0]["thinking"], "internal");
        assert!(payload["messages"][0]["content"][0]
            .get("signature")
            .is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_model_params_enable_preserved_thinking_context() -> Result<()> {
        let _guard = env_lock::lock_env([
            ("CLAUDE_THINKING_ENABLED", None::<&str>),
            ("ANTHROPIC_PRESERVE_THINKING_CONTEXT", None::<&str>),
            ("ANTHROPIC_PRESERVE_UNSIGNED_THINKING", None::<&str>),
        ]);

        let mut params = std::collections::HashMap::new();
        params.insert("preserve_thinking_context".to_string(), json!(true));

        let mut config = cfg("glm-4.7");
        config.request_params = Some(params);
        config.max_tokens = Some(64000);
        let messages = vec![
            Message::assistant().with_content(MessageContent::thinking("internal", "")),
            Message::user().with_text("Continue"),
        ];

        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["clear_thinking"], false);
        assert_eq!(payload["messages"][0]["content"][0]["type"], "thinking");
        assert_eq!(payload["messages"][0]["content"][0]["thinking"], "internal");

        Ok(())
    }

    #[test]
    fn test_tool_error_handling_maintains_pairing() {
        use crate::conversation::message::Message;
        use rmcp::model::{ErrorCode, ErrorData};

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("calculator")
                    .with_arguments(object!({"expression": "2 + 2"}))),
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

    #[test]
    fn test_whitespace_only_text_blocks_are_skipped() {
        let messages = vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("").with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("search").with_arguments(object!({"query": "test"}))),
            ),
            Message::user()
                .with_tool_response("tool_1", Ok(rmcp::model::CallToolResult::success(vec![]))),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 3);

        let assistant_content = spec[1]["content"].as_array().unwrap();
        assert_eq!(assistant_content.len(), 1);
        assert_eq!(assistant_content[0]["type"], "tool_use");
    }

    #[test]
    fn test_tool_response_with_resource_content() {
        use rmcp::model::{CallToolResult, Content};

        let resource_content = Content::embedded_text(
            "file:///test/file.txt",
            "This is the file content from a resource",
        );

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("view_file")
                    .with_arguments(object!({"path": "/test/file.txt"}))),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Ok(CallToolResult::success(vec![resource_content])),
            ),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"][0]["type"], "tool_result");
        assert_eq!(spec[1]["content"][0]["tool_use_id"], "tool_1");
        assert_eq!(
            spec[1]["content"][0]["content"],
            "This is the file content from a resource"
        );
    }

    #[test]
    fn test_tool_response_with_mixed_content() {
        use rmcp::model::{CallToolResult, Content};

        let text_content = Content::text("Summary: file loaded");
        let resource_content = Content::embedded_text("file:///test/file.txt", "File content here");

        let messages = vec![
            Message::assistant().with_tool_request(
                "tool_1",
                Ok(CallToolRequestParams::new("view_file")
                    .with_arguments(object!({"path": "/test/file.txt"}))),
            ),
            Message::user().with_tool_response(
                "tool_1",
                Ok(CallToolResult::success(vec![
                    text_content,
                    resource_content,
                ])),
            ),
        ];

        let spec = format_messages(&messages);

        assert_eq!(spec[1]["content"][0]["type"], "tool_result");
        assert_eq!(
            spec[1]["content"][0]["content"],
            "Summary: file loaded\nFile content here"
        );
    }

    #[test]
    fn test_args_to_input_value_returns_empty_object_for_none() {
        let value = args_to_input_value(None);
        assert!(value.is_object(), "expected JSON object, got {value:?}");
        assert_eq!(value, json!({}));
        assert!(!value.is_null());
    }

    #[test]
    fn test_unparseable_tool_request_emits_placeholder_tool_use() {
        use rmcp::model::{ErrorCode, ErrorData};

        let err = ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "Tool arguments for id call_bad must be a JSON object".to_string(),
            None,
        );
        let mut response = Message::user();
        response.add_tool_response_with_metadata("call_bad", Err(err.clone()), None);
        let messages = vec![
            Message::assistant().with_tool_request("call_bad", Err(err)),
            response,
        ];

        let spec = format_messages(&messages);

        let mut open = std::collections::HashSet::new();
        for m in &spec {
            for block in m["content"].as_array().into_iter().flatten() {
                match block["type"].as_str() {
                    Some("tool_use") => {
                        open.insert(block["id"].as_str().unwrap().to_string());
                    }
                    Some("tool_result") => {
                        let id = block["tool_use_id"].as_str().unwrap();
                        assert!(open.contains(id), "orphan tool_result for id {id:?}");
                    }
                    _ => {}
                }
            }
        }
        assert!(open.contains("call_bad"));
    }

    #[test]
    fn test_args_to_input_value_preserves_existing_args() {
        let args = object!({"query": "rust"});
        let value = args_to_input_value(Some(args));
        assert_eq!(value, json!({"query": "rust"}));
    }

    #[test]
    fn test_parameterless_tool_request_serializes_input_as_empty_object() {
        // Regression test for #9287: when arguments is None (parameterless
        // MCP tool, session reload, or provider switching) the `input` field
        // must serialize as `{}` so the Anthropic API does not reject the
        // replayed tool_use block with a 400 error.
        let messages = vec![
            Message::assistant()
                .with_tool_request("tool_1", Ok(CallToolRequestParams::new("list_things"))),
            Message::user()
                .with_tool_response("tool_1", Ok(rmcp::model::CallToolResult::success(vec![]))),
        ];

        let spec = format_messages(&messages);

        let input = &spec[0]["content"][0]["input"];
        assert!(input.is_object(), "expected object, got {input:?}");
        assert!(!input.is_null());
        assert_eq!(input, &json!({}));
    }

    #[test]
    fn test_parameterless_frontend_tool_request_serializes_input_as_empty_object() {
        // Same regression as above, but exercises the FrontendToolRequest
        // branch which is reached for UI-originated tool calls.
        let messages = vec![Message::assistant().with_frontend_tool_request(
            "frontend_tool_1",
            Ok(CallToolRequestParams::new("list_things")),
        )];

        let spec = format_messages(&messages);

        let input = &spec[0]["content"][0]["input"];
        assert!(input.is_object(), "expected object, got {input:?}");
        assert!(!input.is_null());
        assert_eq!(input, &json!({}));
    }

    fn cfg(name: &str) -> ModelConfig {
        ModelConfig::new(name)
    }

    fn cfg_with_effort(name: &str, effort: &str) -> ModelConfig {
        let mut params = std::collections::HashMap::new();
        params.insert("thinking_effort".to_string(), json!(effort));
        ModelConfig::new(name).with_merged_request_params(params)
    }

    #[test]
    fn test_thinking_type_from_effort() {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);
        // Adaptive model with effort → adaptive
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-6", "high")),
            ThinkingType::Adaptive
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-7", "high")),
            ThinkingType::Adaptive
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-8", "high")),
            ThinkingType::Adaptive
        );
        // Adaptive model with off → disabled
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-6", "off")),
            ThinkingType::Disabled
        );
        // Non-adaptive Claude with effort → enabled
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-3-7-sonnet-20250219", "high")),
            ThinkingType::Enabled
        );
        // Non-adaptive Claude with off → disabled
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-3-7-sonnet-20250219", "off")),
            ThinkingType::Disabled
        );
    }

    #[test]
    fn test_thinking_type_always_on_adaptive() {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);

        assert_eq!(
            thinking_type(&cfg("claude-fable-5")),
            ThinkingType::Adaptive
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-fable-5", "off")),
            ThinkingType::Adaptive
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-fable-5", "high")),
            ThinkingType::Adaptive
        );
    }

    #[test]
    fn test_create_request_fable_5_omits_temperature() -> Result<()> {
        let _guard = env_lock::lock_env([("GOOSE_THINKING_EFFORT", None::<&str>)]);
        let mut config = cfg("claude-fable-5");
        config.max_tokens = Some(4096);
        config.temperature = Some(0.7);
        let messages = vec![Message::user().with_text("Hello")];

        let payload = create_request_with_default_options(&config, "system", &messages, &[])?;

        assert_eq!(payload["thinking"]["type"], "adaptive");
        assert!(payload.get("temperature").is_none());
        assert_eq!(payload["output_config"]["effort"], "high");

        Ok(())
    }

    #[test]
    fn test_thinking_type_non_claude_always_disabled() {
        assert_eq!(
            thinking_type(&cfg_with_effort("gpt-4o", "off")),
            ThinkingType::Disabled
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("gpt-4o", "high")),
            ThinkingType::Disabled
        );
    }

    #[test]
    fn test_thinking_type_off_means_disabled() {
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-opus-4-6", "off")),
            ThinkingType::Disabled
        );
        assert_eq!(
            thinking_type(&cfg_with_effort("claude-3-7-sonnet-20250219", "off")),
            ThinkingType::Disabled
        );
    }

    #[derive(Default)]
    struct StreamedParts {
        thinking: Vec<(String, String)>,
        redacted_thinking: Vec<String>,
        text: Vec<String>,
        tool_calls: Vec<String>,
        tool_errors: Vec<String>,
    }

    async fn collect_stream(events: &str) -> StreamedParts {
        let mut parts = StreamedParts::default();

        for result in collect_stream_results(events).await {
            if let Ok((Some(msg), _usage)) = result {
                for c in &msg.content {
                    match c {
                        MessageContent::Thinking(t) => {
                            parts
                                .thinking
                                .push((t.thinking.clone(), t.signature.clone()));
                        }
                        MessageContent::RedactedThinking(r) => {
                            parts.redacted_thinking.push(r.data.clone());
                        }
                        MessageContent::Text(t) => {
                            parts.text.push(t.text.clone());
                        }
                        MessageContent::ToolRequest(req) => match &req.tool_call {
                            Ok(call) => parts.tool_calls.push(call.name.to_string()),
                            Err(e) => parts.tool_errors.push(e.message.to_string()),
                        },
                        _ => {}
                    }
                }
            }
        }
        parts
    }

    #[tokio::test]
    async fn test_streaming_thinking_and_text() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me analyze"}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":" this problem."}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig_abc"}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"123"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Here is the answer."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":1}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.thinking.len(), 1);
        assert_eq!(parts.thinking[0].0, "Let me analyze this problem.");
        assert_eq!(parts.thinking[0].1, "sig_abc123");
        assert_eq!(parts.text, vec!["Here is the answer."]);
    }

    #[tokio::test]
    async fn test_streaming_thinking_from_start_block_without_signature() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"Initial reasoning "}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"continues."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.thinking.len(), 1);
        assert_eq!(parts.thinking[0].0, "Initial reasoning continues.");
        assert_eq!(parts.thinking[0].1, "");
    }

    #[tokio::test]
    async fn test_streaming_redacted_thinking() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_2","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":5,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"redacted_thinking","data":"opaque_base64_data"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Done."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":1}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.redacted_thinking, vec!["opaque_base64_data"]);
        assert_eq!(parts.text, vec!["Done."]);
    }

    #[tokio::test]
    async fn test_streaming_thinking_text_then_tool_call() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_3","role":"assistant","content":[],"model":"claude-sonnet-4-6","usage":{"input_tokens":8,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"I should search for this."}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"tool_sig_xyz"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Let me search for that."}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":1}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"tool_1","name":"search","input":{}}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"query\":\"rust\"}"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":2}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":15}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.thinking.len(), 1);
        assert_eq!(
            parts.thinking[0],
            (
                "I should search for this.".to_string(),
                "tool_sig_xyz".to_string()
            )
        );
        assert_eq!(parts.text, vec!["Let me search for that."]);
        assert_eq!(parts.tool_calls, vec!["search"]);
    }

    async fn collect_stream_results(
        events: &str,
    ) -> Vec<anyhow::Result<(Option<Message>, Option<ProviderUsage>)>> {
        use futures::StreamExt;

        let lines: Vec<Result<String, anyhow::Error>> =
            events.lines().map(|l| Ok(l.to_string())).collect();
        let stream = Box::pin(futures::stream::iter(lines));
        response_to_streaming_message(stream).collect().await
    }

    #[tokio::test]
    async fn test_streaming_preserves_cache_tokens_through_delta_merge() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":7,"cache_creation_input_tokens":10000,"cache_read_input_tokens":5000,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":25}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let usage = collect_stream_results(events)
            .await
            .into_iter()
            .filter_map(|r| r.ok().and_then(|(_, usage)| usage))
            .next_back()
            .expect("stream should yield usage");

        assert_eq!(usage.usage.input_tokens, Some(15007));
        assert_eq!(usage.usage.output_tokens, Some(25));
        assert_eq!(usage.usage.cache_read_input_tokens, Some(5000));
        assert_eq!(usage.usage.cache_write_input_tokens, Some(10000));
    }

    #[tokio::test]
    async fn test_streaming_delta_usage_is_cumulative_and_wins() {
        // Server tool use grows input during the turn: the final
        // message_delta usage is authoritative, not message_start.
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":2679,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":3}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"input_tokens":10682,"cache_creation_input_tokens":100,"cache_read_input_tokens":200,"output_tokens":510}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let usage = collect_stream_results(events)
            .await
            .into_iter()
            .filter_map(|r| r.ok().and_then(|(_, usage)| usage))
            .next_back()
            .expect("stream should yield usage");

        assert_eq!(usage.usage.input_tokens, Some(10982)); // 10682 + 100 + 200
        assert_eq!(usage.usage.output_tokens, Some(510));
        assert_eq!(usage.usage.cache_read_input_tokens, Some(200));
        assert_eq!(usage.usage.cache_write_input_tokens, Some(100));
    }

    #[test]
    fn test_merge_delta_usage_raw_input_inherits_start_cache() {
        let start =
            Usage::new(Some(15007), Some(3), None).with_cache_tokens(Some(5000), Some(10000));
        let delta_data = json!({"input_tokens": 8, "output_tokens": 510});
        let delta = get_usage(&delta_data).unwrap();

        let merged = merge_delta_usage(&start, &delta, &delta_data);
        assert_eq!(merged.input_tokens, Some(15008)); // 8 + 5000 + 10000
        assert_eq!(merged.output_tokens, Some(510));
        assert_eq!(merged.cache_read_input_tokens, Some(5000));
        assert_eq!(merged.cache_write_input_tokens, Some(10000));
    }

    fn expect_refusal(
        results: Vec<anyhow::Result<(Option<Message>, Option<ProviderUsage>)>>,
    ) -> (String, Option<String>) {
        let err = results
            .into_iter()
            .find_map(|r| r.err())
            .expect("refusal should surface as a stream error");
        match err.downcast_ref::<ProviderError>() {
            Some(ProviderError::Refusal { details, category }) => {
                (details.clone(), category.clone())
            }
            other => panic!("expected ProviderError::Refusal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_streaming_refusal() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal","stop_details":{"explanation":"This request violates the usage policy.","category":"cyber"}},"usage":{"output_tokens":5}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let results = collect_stream_results(events).await;
        let usage = results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .find_map(|(_, usage)| usage.clone())
            .expect("a refused request should still yield its usage");
        assert_eq!(usage.usage.input_tokens, Some(10));
        assert_eq!(usage.usage.output_tokens, Some(5));

        let (details, category) = expect_refusal(results);
        assert_eq!(details, "This request violates the usage policy.");
        assert_eq!(category.as_deref(), Some("cyber"));
    }

    #[tokio::test]
    async fn test_streaming_refusal_forwards_unrecognized_stop_details() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"refusal","stop_details":{"code":42}},"usage":{"output_tokens":5}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let (details, category) = expect_refusal(collect_stream_results(events).await);
        assert!(details.contains("\"code\":42"), "details: {details}");
        assert_eq!(category, None);
    }

    #[tokio::test]
    async fn test_streaming_forwards_unhandled_stop_details() {
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_1","role":"assistant","content":[],"model":"claude-opus-4-6","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"model_context_window_exceeded","stop_details":{"reason":"context_window"}},"usage":{"output_tokens":5}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.text.len(), 1);
        assert!(
            parts.text[0].starts_with("The provider ended the response with:"),
            "text: {}",
            parts.text[0]
        );
        assert!(parts.text[0].contains("context_window"));
    }

    #[tokio::test]
    async fn test_streaming_truncated_tool_args_in_content_block_stop() {
        // Block is closed by content_block_stop, but the concatenated deltas form
        // truncated JSON (each fragment is valid; together they're unterminated).
        let events = concat!(
            r##"data: {"type":"message_start","message":{"id":"msg_t","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"##,
            "\n",
            r##"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_t","name":"write","input":{}}}"##,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/some/path.md\","}}"#,
            "\n",
            r##"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"content\":\"# Very long markdown"}}"##,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":4096}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(
            parts.tool_errors.len(),
            1,
            "expected one tool error, got: {:?}",
            parts.tool_errors
        );
        let msg = &parts.tool_errors[0];
        assert!(
            msg.contains("truncated") || msg.contains("output token limit"),
            "expected actionable truncation message, got: {}",
            msg
        );
        assert!(
            msg.contains("max_tokens") || msg.contains("smaller steps"),
            "expected guidance to increase max_tokens or break up the task, got: {}",
            msg
        );
    }

    #[tokio::test]
    async fn test_streaming_truncated_tool_args_no_content_block_stop() {
        // The stream ends with the tool_use block still open (no content_block_stop),
        // which is what happens when the model is cut off mid-tool-call.
        let events = concat!(
            r##"data: {"type":"message_start","message":{"id":"msg_t2","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"##,
            "\n",
            r##"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_t2","name":"write","input":{}}}"##,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/report.md\","}}"#,
            "\n",
            r##"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"content\":\"# Big report that got cut off mid"}}"##,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":8192}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(
            parts.tool_errors.len(),
            1,
            "expected one tool error for the dropped/truncated tool call, got: {:?}",
            parts.tool_errors
        );
        let msg = &parts.tool_errors[0];
        assert!(
            msg.contains("truncated") || msg.contains("output token limit"),
            "expected actionable truncation message, got: {}",
            msg
        );
    }

    #[tokio::test]
    async fn test_streaming_complete_tool_call_unaffected() {
        // Regression guard: a normal, complete tool call must still parse and
        // produce no error even though stop_reason handling is added.
        let events = concat!(
            r#"data: {"type":"message_start","message":{"id":"msg_ok","role":"assistant","content":[],"model":"glm-4.7","usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"tool_ok","name":"write","input":{}}}"#,
            "\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/ok.md\",\"content\":\"hello\"}"}}"#,
            "\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":15}}"#,
            "\n",
            r#"data: {"type":"message_stop"}"#,
        );

        let parts = collect_stream(events).await;
        assert_eq!(parts.tool_calls, vec!["write"]);
        assert!(parts.tool_errors.is_empty());
    }
}
