//! OpenAI API format handling module
//!
//! This module provides functionality to format messages and requests according to the OpenAI API specifications.
//! It handles various types of content including text, images, tool requests, and tool responses, ensuring they
//! are properly formatted for the OpenAI API endpoints.
//!
//! Key features:
//! - Message formatting for OpenAI's chat completion API
//! - Support for O1/O3 model families with reasoning effort handling
//! - Tool request and response formatting
//! - Image content handling with format conversion
//! - Error handling for tool calls and responses

use crate::message::{Message, MessageContent, ToolRequest, ToolResponse};
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{
    convert_image, is_valid_function_name, sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Error};
use mcp_core::content::TextContent;
use mcp_core::ToolError;
use mcp_core::{Content, Role, Tool, ToolCall};
use serde_json::{json, Value};

/// Formats text content into an OpenAI API compatible message.
///
/// This function processes text content and converts it into a format suitable for the OpenAI API.
/// Empty text content is handled by returning None, which allows the caller to skip adding empty
/// messages to the final payload.
///
/// # Arguments
/// * `text` - The text content to format
///
/// # Returns
/// * `Option<Value>` - The formatted text as a JSON value, or None if the text is empty
///
/// # Example
/// ```
/// use mcp_core::TextContent;
/// use serde_json::json;
/// use goose::providers::formats::openai::format_text_content;
/// let text = TextContent { text: "Hello".to_string(), annotations: None };
/// let formatted = format_text_content(&text);
/// assert_eq!(formatted, Some(json!("Hello")));
/// ```
pub fn format_text_content(text: &TextContent) -> Option<Value> {
    if !text.text.is_empty() {
        Some(json!(text.text))
    } else {
        None
    }
}

/// Formats a tool request into OpenAI's function call format.
///
/// This function handles both successful tool calls and errors, producing appropriate
/// JSON structures for the OpenAI API. For successful calls, it creates a function
/// call object. For errors, it creates an error message in the tool response format.
///
/// # Arguments
/// * `request` - The tool request to format, containing either a successful tool call or an error
///
/// # Returns
/// * `(Option<Value>, Vec<Value>)` - A tuple containing:
///   - The formatted tool call as a JSON value (if successful)
///   - A vector of any error messages as JSON values
///
/// # Example
/// ```
/// use serde_json::json;
/// use mcp_core::ToolCall;
/// use goose::message::ToolRequest;
/// use goose::providers::formats::openai::format_tool_request;
/// let request = goose::message::ToolRequest {
///     id: "123".to_string(),
///     tool_call: Ok(ToolCall {
///         name: "search".to_string(),
///         arguments: json!({"query": "test"})
///     })
/// };
/// let (tool_call, errors) = format_tool_request(&request);
/// assert!(tool_call.is_some());
/// assert!(errors.is_empty());
/// ```
pub fn format_tool_request(request: &ToolRequest) -> (Option<Value>, Vec<Value>) {
    let mut output = Vec::new();
    let mut tool_calls = None;

    match &request.tool_call {
        Ok(tool_call) => {
            let sanitized_name = sanitize_function_name(&tool_call.name);
            let tool_call = json!({
                "id": request.id,
                "type": "function",
                "function": {
                    "name": sanitized_name,
                    "arguments": tool_call.arguments.to_string(),
                }
            });
            tool_calls = Some(tool_call);
        }
        Err(e) => {
            output.push(json!({
                "role": "tool",
                "content": format!("Error: {}", e),
                "tool_call_id": request.id
            }));
        }
    }

    (tool_calls, output)
}

/// Processes individual tool content items, with special handling for images and resources.
///
/// This function handles different types of content that can appear in tool responses,
/// with particular focus on:
/// - Converting images into the appropriate format with placeholder text
/// - Converting resources into text
/// - Preserving other content types as-is
///
/// # Arguments
/// * `content` - The content item to process
/// * `image_format` - The desired format for image content
///
/// # Returns
/// * `(Vec<Content>, Vec<Value>)` - A tuple containing:
///   - Processed content items
///   - Any separate image messages that need to be sent
///
/// # Example
/// ```
/// use mcp_core::{Content, TextContent};
/// use goose::providers::utils::ImageFormat;
/// use goose::providers::formats::openai::process_tool_content;
/// let content = Content::Text(TextContent { text: "test".to_string(), annotations: None });
/// let (processed, images) = process_tool_content(&content, &ImageFormat::OpenAi);
/// assert_eq!(processed.len(), 1);
/// assert!(images.is_empty());
/// ```
pub fn process_tool_content(
    content: &Content,
    image_format: &ImageFormat,
) -> (Vec<Content>, Vec<Value>) {
    let mut tool_content = Vec::new();
    let mut image_messages = Vec::new();

    match content {
        Content::Image(image) => {
            // Add placeholder text in the tool response
            tool_content.push(Content::text(
                "This tool result included an image that is uploaded in the next message.",
            ));

            // Create a separate image message
            image_messages.push(json!({
                "role": "user",
                "content": [convert_image(image, image_format)]
            }));
        }
        Content::Resource(resource) => {
            tool_content.push(Content::text(resource.get_text()));
        }
        _ => {
            tool_content.push(content.clone());
        }
    }

    (tool_content, image_messages)
}

/// Formats a tool response into OpenAI API compatible messages.
///
/// This function handles the complete processing of tool responses, including:
/// - Filtering content based on audience
/// - Processing images and other content types
/// - Handling success and error cases
/// - Creating properly structured response messages
///
/// # Arguments
/// * `response` - The tool response to format
/// * `image_format` - The desired format for any images in the response
///
/// # Returns
/// * `Vec<Value>` - A vector of formatted messages ready for the OpenAI API
///
/// # Example
/// ```
/// use mcp_core::{Content, TextContent};
/// use goose::message::ToolResponse;
/// use goose::providers::utils::ImageFormat;
/// use goose::providers::formats::openai::format_tool_response;
/// let response = goose::message::ToolResponse {
///     id: "123".to_string(),
///     tool_result: Ok(vec![Content::Text(TextContent { text: "test".to_string(), annotations: None })])
/// };
/// let messages = format_tool_response(&response, &ImageFormat::OpenAi);
/// assert_eq!(messages.len(), 1);
/// ```
pub fn format_tool_response(response: &ToolResponse, image_format: &ImageFormat) -> Vec<Value> {
    let mut output = Vec::new();

    match &response.tool_result {
        Ok(contents) => {
            // Send only contents with no audience or with Assistant in the audience
            let abridged: Vec<_> = contents
                .iter()
                .filter(|content| {
                    content
                        .audience()
                        .is_none_or(|audience| audience.contains(&Role::Assistant))
                })
                .map(|content| content.unannotated())
                .collect();

            let mut all_tool_content = Vec::new();
            let mut all_image_messages = Vec::new();

            // Process all content, replacing images with placeholder text
            for content in abridged {
                let (tool_content, image_messages) = process_tool_content(&content, image_format);
                all_tool_content.extend(tool_content);
                all_image_messages.extend(image_messages);
            }

            let tool_response_content: Value = json!(all_tool_content
                .iter()
                .map(|content| match content {
                    Content::Text(text) => text.text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<String>>()
                .join(" "));

            // First add the tool response with all content
            output.push(json!({
                "role": "tool",
                "content": tool_response_content,
                "tool_call_id": response.id
            }));

            // Then add any image messages
            output.extend(all_image_messages);
        }
        Err(e) => {
            output.push(json!({
                "role": "tool",
                "content": format!("Error: {}", e),
                "tool_call_id": response.id
            }));
        }
    }

    output
}

/// Convert internal Message format to OpenAI's API message specification.
///
/// This function serves as the main entry point for converting internal message formats
/// to OpenAI's API format. It handles various types of content and ensures proper
/// formatting for all OpenAI API requirements.
///
/// Some OpenAI-compatible endpoints use the Anthropic image spec at the content level
/// even though the message structure otherwise follows OpenAI conventions. The image_format
/// parameter controls this behavior.
///
/// # Arguments
/// * `messages` - The messages to format
/// * `image_format` - The desired format for image content
///
/// # Returns
/// * `Vec<Value>` - A vector of formatted messages ready for the OpenAI API
///
/// # Example
/// ```
/// use mcp_core::{Role, TextContent};
/// use goose::message::{Message, MessageContent};
/// use goose::providers::utils::ImageFormat;
/// use goose::providers::formats::openai::format_messages;
/// let message = goose::message::Message {
///     role: Role::User,
///     content: vec![MessageContent::Text(TextContent { text: "Hello".to_string(), annotations: None })],
///     created: 0
/// };
/// let formatted = format_messages(&[message], &ImageFormat::OpenAi);
/// assert_eq!(formatted.len(), 1);
/// ```
pub fn format_messages(messages: &[Message], image_format: &ImageFormat) -> Vec<Value> {
    let mut messages_spec = Vec::new();

    for message in messages {
        let mut converted = json!({
            "role": message.role
        });

        let mut output = Vec::new();

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if let Some(content) = format_text_content(text) {
                        converted["content"] = content;
                    }
                }
                MessageContent::ToolRequest(request) => {
                    let (tool_calls, mut request_output) = format_tool_request(request);
                    if let Some(tool_call) = tool_calls {
                        let tool_calls_array = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));
                        tool_calls_array.as_array_mut().unwrap().push(tool_call);
                    }
                    output.append(&mut request_output);
                }
                MessageContent::ToolResponse(response) => {
                    output.extend(format_tool_response(response, image_format));
                }
                MessageContent::Image(image) => {
                    // Handle direct image content
                    converted["content"] = json!([convert_image(image, image_format)]);
                }
            }
        }

        if !converted["content"].is_null() || converted.get("tool_calls").is_some() {
            messages_spec.push(converted);
        }
        messages_spec.extend(output);
    }

    messages_spec
}

/// Convert internal Tool format to OpenAI's API tool specification.
///
/// This function formats tools according to the OpenAI API tool specification.
/// It handles tool names, descriptions, and parameters, ensuring they are properly
/// formatted for the OpenAI API.
///
/// # Arguments
/// * `tools` - The tools to format
///
/// # Returns
/// * `anyhow::Result<Vec<Value>, Error>` - A vector of formatted tools ready for the OpenAI API,
///   or an error if there are duplicate tool names
///
/// # Example
/// ```
/// use serde_json::json;
/// use mcp_core::Tool;
/// use goose::providers::formats::openai::format_tools;
/// let tool = Tool::new(
///     "test_tool",
///     "Test tool",
///     json!({
///         "type": "object",
///         "properties": {
///             "input": {
///                 "type": "string",
///                 "description": "Test parameter"
///             }
///         },
///         "required": ["input"]
///     }),
/// );
/// let formatted = format_tools(&[tool])?;
/// assert_eq!(formatted.len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn format_tools(tools: &[Tool]) -> anyhow::Result<Vec<Value>, Error> {
    let mut tool_names = std::collections::BTreeSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        let mut description = tool.description.clone();
        description.truncate(1024);

        // OpenAI's tool description max str len is 1024
        result.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": description,
                "parameters": tool.input_schema,
            }
        }));
    }

    Ok(result)
}

/// Convert OpenAI's API response to internal Message format.
///
/// This function processes OpenAI's API response and converts it into the internal
/// Message format. It handles text content, tool calls, and errors, ensuring they are
/// properly formatted for internal use.
///
/// # Arguments
/// * `response` - The OpenAI API response to convert
///
/// # Returns
/// * `anyhow::Result<Message, Error>` - The converted message, or an error if the response is invalid
///
/// # Example
/// ```
/// use serde_json::json;
/// use goose::message::Message;
/// use goose::providers::formats::openai::response_to_message;
/// let response = json!({
///     "choices": [{
///         "role": "assistant",
///         "message": {
///             "content": "Hello from John Cena!"
///         }
///     }],
///     "usage": {
///         "input_tokens": 10,
///         "output_tokens": 25,
///         "total_tokens": 35
///     }
/// });
/// let message = response_to_message(response)?;
/// assert_eq!(message.content.len(), 1);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn response_to_message(response: Value) -> anyhow::Result<Message, Error> {
    let original = response["choices"][0]["message"].clone();
    let mut content = Vec::new();

    if let Some(text) = original.get("content") {
        if let Some(text_str) = text.as_str() {
            content.push(MessageContent::text(text_str));
        }
    }

    if let Some(tool_calls) = original.get("tool_calls") {
        if let Some(tool_calls_array) = tool_calls.as_array() {
            for tool_call in tool_calls_array {
                let id = tool_call["id"].as_str().unwrap_or_default().to_string();
                let function_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let arguments = tool_call["function"]["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                if !is_valid_function_name(&function_name) {
                    let error = ToolError::NotFound(format!(
                        "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                        function_name
                    ));
                    content.push(MessageContent::tool_request(id, Err(error)));
                } else {
                    match serde_json::from_str::<Value>(&arguments) {
                        Ok(params) => {
                            content.push(MessageContent::tool_request(
                                id,
                                Ok(ToolCall::new(&function_name, params)),
                            ));
                        }
                        Err(e) => {
                            let error = ToolError::InvalidParameters(format!(
                                "Could not interpret tool use parameters for id {}: {}",
                                id, e
                            ));
                            content.push(MessageContent::tool_request(id, Err(error)));
                        }
                    }
                }
            }
        }
    }

    Ok(Message {
        role: Role::Assistant,
        created: chrono::Utc::now().timestamp(),
        content,
    })
}

/// Extract usage data from OpenAI's API response.
///
/// This function processes OpenAI's API response and extracts usage data, including
/// input tokens, output tokens, and total tokens.
///
/// # Arguments
/// * `data` - The OpenAI API response to extract usage data from
///
/// # Returns
/// * `Result<Usage, ProviderError>` - The extracted usage data, or an error if the response is invalid
///
/// # Example
/// ```
/// use serde_json::json;
/// use goose::providers::base::Usage;
/// use goose::providers::formats::openai::get_usage;
/// let data = json!({
///     "usage": {
///         "prompt_tokens": 10,
///         "completion_tokens": 25,
///         "total_tokens": 35
///     }
/// });
/// let usage = get_usage(&data)?;
/// assert_eq!(usage.input_tokens, Some(10));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_usage(data: &Value) -> Result<Usage, ProviderError> {
    let usage = data
        .get("usage")
        .ok_or_else(|| ProviderError::UsageError("No usage data in response".to_string()))?;

    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let output_tokens = usage
        .get("completion_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let total_tokens = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .or_else(|| match (input_tokens, output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        });

    Ok(Usage::new(input_tokens, output_tokens, total_tokens))
}

/// Create a request for OpenAI's API.
///
/// This function creates a request for OpenAI's API, handling various parameters such as
/// model name, system message, messages, tools, and image format.
///
/// # Arguments
/// * `model_config` - The model configuration to use
/// * `system` - The system message to include in the request
/// * `messages` - The messages to include in the request
/// * `tools` - The tools to include in the request
/// * `image_format` - The desired format for image content
///
/// # Returns
/// * `anyhow::Result<Value, Error>` - The created request, or an error if the request is invalid
///
/// # Example
/// ```
/// use goose::model::ModelConfig;
/// use goose::providers::utils::ImageFormat;
/// use goose::providers::formats::openai::create_request;
/// let model_config = ModelConfig {
///     model_name: "gpt-4".to_string(),
///     tokenizer_name: "gpt-4".to_string(),
///     context_limit: Some(4096),
///     temperature: Some(0.7),
///     max_tokens: Some(1024),
/// };
/// let system = "system";
/// let messages = vec![];
/// let tools = vec![];
/// let image_format = ImageFormat::OpenAi;
/// let request = create_request(&model_config, system, &messages, &tools, &image_format)?;
/// assert_eq!(request["model"], "gpt-4");
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
) -> anyhow::Result<Value, Error> {
    let is_o1 = model_config.model_name.starts_with("o1");
    let is_o3 = model_config.model_name.starts_with("o3");

    // Only extract reasoning effort for O1/O3 models
    let (model_name, reasoning_effort) = if is_o1 || is_o3 {
        let parts: Vec<&str> = model_config.model_name.split('-').collect();
        let last_part = parts.last().unwrap();

        match *last_part {
            "low" | "medium" | "high" => {
                let base_name = parts[..parts.len() - 1].join("-");
                (base_name, Some(last_part.to_string()))
            }
            _ => (
                model_config.model_name.to_string(),
                Some("medium".to_string()),
            ),
        }
    } else {
        // For non-O family models, use the model name as is and no reasoning effort
        (model_config.model_name.to_string(), None)
    };

    if model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since Goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let system_message = json!({
        // NOTE: per OPENAI docs , With O1 and newer models, `developer`
        // should replace `system` role .
        // https://platform.openai.com/docs/api-reference/chat/create
        "role": if is_o1 || is_o3 { "developer" } else { "system" },
        "content": system
    });

    let messages_spec = format_messages(messages, image_format);
    let tools_spec = if !tools.is_empty() {
        format_tools(tools)?
    } else {
        vec![]
    };

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": model_name,
        "messages": messages_array
    });

    // NOTE: add resoning effort if present
    // e.g if the user chooses `o3-mini-high` as their model name
    // then it will set `reasoning_effort` to `high`.
    // Defaults to medium per openai docs
    // https://platform.openai.com/docs/api-reference/chat/create#chat-create-reasoning_effort
    if let Some(effort) = reasoning_effort {
        payload
            .as_object_mut()
            .unwrap()
            .insert("reasoning_effort".to_string(), json!(effort));
    }

    // Add tools if present
    if !tools_spec.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    // o1, o3 models currently don't support temperature
    if !is_o1 && !is_o3 {
        if let Some(temp) = model_config.temperature {
            payload
                .as_object_mut()
                .unwrap()
                .insert("temperature".to_string(), json!(temp));
        }
    }

    // o1 models use max_completion_tokens instead of max_tokens
    if let Some(tokens) = model_config.max_tokens {
        let key = if is_o1 || is_o3 {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        payload
            .as_object_mut()
            .unwrap()
            .insert(key.to_string(), json!(tokens));
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const OPENAI_TOOL_USE_RESPONSE: &str = r#"{
        "choices": [{
            "role": "assistant",
            "message": {
                "tool_calls": [{
                    "id": "1",
                    "function": {
                        "name": "example_fn",
                        "arguments": "{\"param\": \"value\"}"
                    }
                }]
            }
        }],
        "usage": {
            "input_tokens": 10,
            "output_tokens": 25,
            "total_tokens": 35
        }
    }"#;

    const EPSILON: f64 = 1e-6; // More lenient epsilon for float comparison

    // Test utilities
    struct TestModelConfig {
        model_name: String,
        tokenizer_name: String,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
    }

    impl TestModelConfig {
        fn new(model_name: &str, tokenizer_name: &str) -> Self {
            Self {
                model_name: model_name.to_string(),
                tokenizer_name: tokenizer_name.to_string(),
                temperature: Some(0.7),
                max_tokens: Some(1024),
            }
        }

        fn without_temperature(mut self) -> Self {
            self.temperature = None;
            self
        }

        fn to_model_config(&self) -> ModelConfig {
            ModelConfig {
                model_name: self.model_name.clone(),
                tokenizer_name: self.tokenizer_name.clone(),
                context_limit: Some(4096),
                temperature: self.temperature,
                max_tokens: self.max_tokens,
            }
        }
    }

    fn assert_request(
        model_config: &TestModelConfig,
        expected_model: &str,
        expected_reasoning: Option<&str>,
        expect_max_completion_tokens: bool,
    ) -> anyhow::Result<()> {
        let request = create_request(
            &model_config.to_model_config(),
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
        )?;
        let obj = request.as_object().unwrap();

        // Check model name
        assert_eq!(obj.get("model").unwrap(), expected_model);

        // Check reasoning effort
        match expected_reasoning {
            Some(effort) => assert_eq!(obj.get("reasoning_effort").unwrap(), effort),
            None => assert!(obj.get("reasoning_effort").is_none()),
        }

        // Check max tokens field
        if expect_max_completion_tokens {
            assert_eq!(obj.get("max_completion_tokens").unwrap(), 1024);
            assert!(obj.get("max_tokens").is_none());
        } else {
            assert!(obj.get("max_completion_tokens").is_none());
            assert_eq!(obj.get("max_tokens").unwrap(), 1024);
        }

        // Check temperature if present
        if let Some(expected_temp) = model_config.temperature {
            let temp = obj.get("temperature").unwrap().as_f64().unwrap();
            assert!((temp - f64::from(expected_temp)).abs() < EPSILON);
        } else {
            assert!(obj.get("temperature").is_none());
        }

        Ok(())
    }

    #[test]
    fn test_format_messages() -> anyhow::Result<()> {
        use crate::providers::formats::openai::format_messages;
        let message = Message::user().with_text("Hello");
        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"], "Hello");
        Ok(())
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
        use crate::providers::formats::openai::format_tools;
        use mcp_core::Tool;
        use serde_json::json;
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let spec = format_tools(&[tool])?;

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["type"], "function");
        assert_eq!(spec[0]["function"]["name"], "test_tool");
        Ok(())
    }

    #[test]
    fn test_format_messages_complex() -> anyhow::Result<()> {
        use crate::message::{Message, MessageContent};
        use crate::providers::formats::openai::format_messages;
        use crate::providers::utils::ImageFormat;
        use mcp_core::Content;
        let mut messages = vec![
            Message::assistant().with_text("Hello!"),
            Message::user().with_text("How are you?"),
            Message::assistant().with_tool_request(
                "tool1",
                Ok(ToolCall::new("example", json!({"param1": "value1"}))),
            ),
        ];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[2].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

        let spec = format_messages(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 4);
        assert_eq!(spec[0]["role"], "assistant");
        assert_eq!(spec[0]["content"], "Hello!");
        assert_eq!(spec[1]["role"], "user");
        assert_eq!(spec[1]["content"], "How are you?");
        assert_eq!(spec[2]["role"], "assistant");
        assert!(spec[2]["tool_calls"].is_array());
        assert_eq!(spec[3]["role"], "tool");
        assert_eq!(spec[3]["content"], "Result");
        assert_eq!(spec[3]["tool_call_id"], spec[2]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_messages_multiple_content() -> anyhow::Result<()> {
        use crate::message::{Message, MessageContent};
        use crate::providers::formats::openai::format_messages;
        use crate::providers::utils::ImageFormat;
        use mcp_core::Content;
        let mut messages = vec![Message::assistant().with_tool_request(
            "tool1",
            Ok(ToolCall::new("example", json!({"param1": "value1"}))),
        )];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[0].content[0] {
            request.id.clone()
        } else {
            panic!("should be tool request");
        };

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

        let spec = format_messages(&messages, &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 2);
        assert_eq!(spec[0]["role"], "assistant");
        assert!(spec[0]["tool_calls"].is_array());
        assert_eq!(spec[1]["role"], "tool");
        assert_eq!(spec[1]["content"], "Result");
        assert_eq!(spec[1]["tool_call_id"], spec[0]["tool_calls"][0]["id"]);

        Ok(())
    }

    #[test]
    fn test_format_tools_duplicate() -> anyhow::Result<()> {
        use crate::providers::formats::openai::format_tools;
        use mcp_core::Tool;
        use serde_json::json;
        let tool1 = Tool::new(
            "test_tool",
            "Test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let tool2 = Tool::new(
            "test_tool",
            "Test tool",
            json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "Test parameter"
                    }
                },
                "required": ["input"]
            }),
        );

        let result = format_tools(&[tool1, tool2]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Duplicate tool name"));

        Ok(())
    }

    #[test]
    fn test_format_tools_empty() -> anyhow::Result<()> {
        use crate::providers::formats::openai::format_tools;
        let spec = format_tools(&[])?;
        assert!(spec.is_empty());
        Ok(())
    }

    #[test]
    fn test_response_to_message_text() -> anyhow::Result<()> {
        use crate::providers::formats::openai::response_to_message;
        use serde_json::json;
        let response = json!({
            "choices": [{
                "role": "assistant",
                "message": {
                    "content": "Hello from John Cena!"
                }
            }],
            "usage": {
                "input_tokens": 10,
                "output_tokens": 25,
                "total_tokens": 35
            }
        });

        let message = response_to_message(response)?;
        assert_eq!(message.content.len(), 1);
        if let MessageContent::Text(text) = &message.content[0] {
            assert_eq!(text.text, "Hello from John Cena!");
        } else {
            panic!("Expected Text content");
        }
        assert!(matches!(message.role, Role::Assistant));

        Ok(())
    }

    #[test]
    fn test_response_to_message_valid_toolrequest() -> anyhow::Result<()> {
        let response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        let message = response_to_message(response)?;

        assert_eq!(message.content.len(), 1);
        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, json!({"param": "value"}));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_invalid_func_name() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["name"] =
            json!("invalid fn");

        let message = response_to_message(response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ToolError::NotFound(msg)) => {
                    assert!(msg.starts_with("The provided function name"));
                }
                _ => panic!("Expected ToolNotFound error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_json_decode_error() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            json!("invalid json {");

        let message = response_to_message(response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            match &request.tool_call {
                Err(ToolError::InvalidParameters(msg)) => {
                    assert!(msg.starts_with("Could not interpret tool use parameters"));
                }
                _ => panic!("Expected InvalidParameters error"),
            }
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_create_request_o3_reasoning_effort() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "o3-mini".to_string(),
            tokenizer_name: "o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        assert_eq!(obj.get("model").unwrap(), "o3-mini");
        assert_eq!(obj.get("reasoning_effort").unwrap(), "medium");
        assert_eq!(obj.get("max_completion_tokens").unwrap(), 1024);
        assert!(obj.get("max_tokens").is_none());

        // Test custom reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "o3-mini-high".to_string(),
            tokenizer_name: "o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        assert_eq!(obj.get("model").unwrap(), "o3-mini");
        assert_eq!(obj.get("reasoning_effort").unwrap(), "high");
        assert_eq!(obj.get("max_completion_tokens").unwrap(), 1024);
        assert!(obj.get("max_tokens").is_none());

        // Test invalid suffix defaults to medium
        let model_config = ModelConfig {
            model_name: "o3-mini-invalid".to_string(),
            tokenizer_name: "o3-mini".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        assert_eq!(obj.get("model").unwrap(), "o3-mini-invalid");
        assert_eq!(obj.get("reasoning_effort").unwrap(), "medium");
        assert_eq!(obj.get("max_completion_tokens").unwrap(), 1024);
        assert!(obj.get("max_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_create_request_o1_reasoning_effort() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O1 model
        let model_config = ModelConfig {
            model_name: "o1".to_string(),
            tokenizer_name: "o1".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        assert_eq!(obj.get("model").unwrap(), "o1");
        assert_eq!(obj.get("reasoning_effort").unwrap(), "medium");
        assert_eq!(obj.get("max_completion_tokens").unwrap(), 1024);
        assert!(obj.get("max_tokens").is_none());

        // Test custom reasoning effort for O1 model
        let model_config = ModelConfig {
            model_name: "o1-low".to_string(),
            tokenizer_name: "o1".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        assert_eq!(obj.get("model").unwrap(), "o1");
        assert_eq!(obj.get("reasoning_effort").unwrap(), "low");
        assert_eq!(obj.get("max_completion_tokens").unwrap(), 1024);
        assert!(obj.get("max_tokens").is_none());

        Ok(())
    }

    #[test]
    fn test_o3_default_reasoning_effort() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("o3-mini", "o3-mini").without_temperature(),
            "o3-mini",
            Some("medium"),
            true,
        )
    }

    #[test]
    fn test_o3_custom_reasoning_effort() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("o3-mini-high", "o3-mini").without_temperature(),
            "o3-mini",
            Some("high"),
            true,
        )
    }

    #[test]
    fn test_o3_invalid_suffix_defaults_to_medium() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("o3-mini-invalid", "o3-mini").without_temperature(),
            "o3-mini-invalid",
            Some("medium"),
            true,
        )
    }

    #[test]
    fn test_o1_default_reasoning_effort() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("o1", "o1").without_temperature(),
            "o1",
            Some("medium"),
            true,
        )
    }

    #[test]
    fn test_o1_custom_reasoning_effort() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("o1-low", "o1").without_temperature(),
            "o1",
            Some("low"),
            true,
        )
    }

    #[test]
    fn test_o1_mini_not_supported() -> anyhow::Result<()> {
        let config = TestModelConfig::new("o1-mini", "o1-mini").without_temperature();
        let result = create_request(
            &config.to_model_config(),
            "system",
            &[],
            &[],
            &ImageFormat::OpenAi,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("o1-mini model is not currently supported"));
        Ok(())
    }

    #[test]
    fn test_gpt4_standard_config() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("gpt-4", "gpt-4"),
            "gpt-4",
            None,
            false,
        )
    }

    #[test]
    fn test_gpt4_with_version_suffix() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("gpt-4-0314", "gpt-4"),
            "gpt-4-0314",
            None,
            false,
        )
    }

    #[test]
    fn test_gpt35_turbo_config() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("gpt-3.5-turbo", "gpt-3.5-turbo"),
            "gpt-3.5-turbo",
            None,
            false,
        )
    }

    #[test]
    fn test_non_o_family_with_high_suffix() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("gpt-4-high-performance", "gpt-4"),
            "gpt-4-high-performance",
            None,
            false,
        )
    }

    #[test]
    fn test_non_o_family_with_low_suffix() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("gpt-4-low-latency", "gpt-4"),
            "gpt-4-low-latency",
            None,
            false,
        )
    }

    #[test]
    fn test_non_o_family_with_medium_suffix() -> anyhow::Result<()> {
        assert_request(
            &TestModelConfig::new("gpt-4-medium", "gpt-4"),
            "gpt-4-medium",
            None,
            false,
        )
    }
}
