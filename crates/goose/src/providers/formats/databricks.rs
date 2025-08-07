use crate::conversation::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::utils::{
    convert_image, detect_image_path, is_valid_function_name, load_image_file, safely_parse_json,
    sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Error};
use mcp_core::{ToolCall, ToolError};
use rmcp::model::{AnnotateAble, Content, RawContent, ResourceContents, Role, Tool};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize)]
struct DatabricksMessage {
    content: Value,
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// Convert internal Message format to Databricks' API message specification
///   Databricks is mostly OpenAI compatible, but has some differences (reasoning type, etc)
///   some openai compatible endpoints use the anthropic image spec at the content level
///   even though the message structure is otherwise following openai, the enum switches this
fn format_messages(messages: &[Message], image_format: &ImageFormat) -> Vec<DatabricksMessage> {
    let mut result = Vec::new();
    for message in messages {
        let mut converted = DatabricksMessage {
            content: Value::Null,
            role: match message.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            tool_calls: None,
            tool_call_id: None,
        };

        let mut content_array = Vec::new();
        let mut has_tool_calls = false;
        let mut has_multiple_content = false;

        for content in &message.content {
            match content {
                MessageContent::Text(text) => {
                    if !text.text.is_empty() {
                        // Check for image paths in the text
                        if let Some(image_path) = detect_image_path(&text.text) {
                            has_multiple_content = true;
                            // Try to load and convert the image
                            if let Ok(image) = load_image_file(image_path) {
                                content_array.push(json!({
                                    "type": "text",
                                    "text": text.text
                                }));
                                content_array.push(convert_image(&image, image_format));
                            } else {
                                content_array.push(json!({
                                    "type": "text",
                                    "text": text.text
                                }));
                            }
                        } else {
                            content_array.push(json!({
                                "type": "text",
                                "text": text.text
                            }));
                        }
                    }
                }
                MessageContent::Thinking(content) => {
                    has_multiple_content = true;
                    content_array.push(json!({
                        "type": "reasoning",
                        "summary": [
                            {
                                "type": "summary_text",
                                "text": content.thinking,
                                "signature": content.signature
                            }
                        ]
                    }));
                }
                MessageContent::RedactedThinking(content) => {
                    has_multiple_content = true;
                    content_array.push(json!({
                        "type": "reasoning",
                        "summary": [
                            {
                                "type": "summary_encrypted_text",
                                "data": content.data
                            }
                        ]
                    }));
                }
                MessageContent::ToolRequest(request) => {
                    has_tool_calls = true;
                    match &request.tool_call {
                        Ok(tool_call) => {
                            let sanitized_name = sanitize_function_name(&tool_call.name);

                            let tool_calls = converted.tool_calls.get_or_insert_default();
                            tool_calls.push(json!({
                                "id": request.id,
                                "type": "function",
                                "function": {
                                    "name": sanitized_name,
                                    "arguments": tool_call.arguments.to_string(),
                                }
                            }));
                        }
                        Err(e) => {
                            content_array.push(json!({
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }));
                        }
                    }
                }
                MessageContent::ContextLengthExceeded(_) => {
                    continue;
                }
                MessageContent::SummarizationRequested(_) => {
                    continue;
                }
                MessageContent::ToolResponse(response) => {
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
                                .map(|content| content.raw.clone())
                                .collect();

                            // Process all content, replacing images with placeholder text
                            let mut tool_content = Vec::new();
                            let mut image_messages = Vec::new();

                            for content in abridged {
                                match content {
                                    RawContent::Image(image) => {
                                        tool_content.push(Content::text("This tool result included an image that is uploaded in the next message."));
                                        image_messages.push(DatabricksMessage {
                                            role: "user".to_string(),
                                            content: [convert_image(
                                                &image.no_annotation(),
                                                image_format,
                                            )]
                                            .into(),
                                            tool_calls: None,
                                            tool_call_id: None,
                                        });
                                    }
                                    RawContent::Resource(resource) => {
                                        let text = match &resource.resource {
                                            ResourceContents::TextResourceContents {
                                                text, ..
                                            } => text.clone(),
                                            _ => String::new(),
                                        };
                                        tool_content.push(Content::text(text));
                                    }
                                    _ => {
                                        tool_content.push(content.no_annotation());
                                    }
                                }
                            }
                            let tool_response_content: Value = json!(tool_content
                                .iter()
                                .filter_map(|content| content.as_text().map(|t| t.text.clone()))
                                .collect::<Vec<String>>()
                                .join(" "));

                            result.push(DatabricksMessage {
                                content: tool_response_content,
                                role: "tool".to_string(),
                                tool_call_id: Some(response.id.clone()),
                                tool_calls: None,
                            });
                            // Then add any image messages that need to follow
                            result.extend(image_messages);
                        }
                        Err(e) => {
                            // A tool result error is shown as output so the model can interpret the error message
                            result.push(DatabricksMessage {
                                role: "tool".to_string(),
                                content: format!(
                                    "The tool call returned the following error:\n{}",
                                    e
                                )
                                .into(),
                                tool_call_id: Some(response.id.clone()),
                                tool_calls: None,
                            });
                        }
                    }
                }
                MessageContent::ToolConfirmationRequest(_) => {
                    // Skip tool confirmation requests
                }
                MessageContent::Image(image) => {
                    // Handle direct image content
                    content_array.push(json!({
                        "type": "image_url",
                        "image_url": {
                            "url": convert_image(image, image_format)
                        }
                    }));
                }
                MessageContent::FrontendToolRequest(req) => {
                    // Frontend tool requests are converted to text messages
                    if let Ok(tool_call) = &req.tool_call {
                        content_array.push(json!({
                            "type": "text",
                            "text": format!(
                                "Frontend tool request: {} ({})",
                                tool_call.name,
                                serde_json::to_string_pretty(&tool_call.arguments).unwrap()
                            )
                        }));
                    } else {
                        content_array.push(json!({
                            "type": "text",
                            "text": format!(
                                "Frontend tool request error: {}",
                                req.tool_call.as_ref().unwrap_err()
                            )
                        }));
                    }
                }
            }
        }

        if !content_array.is_empty() {
            // If we only have a single text content and no other special content,
            // use the simple string format
            if content_array.len() == 1
                && !has_multiple_content
                && content_array[0]["type"] == "text"
            {
                converted.content = json!(content_array[0]["text"]);
            } else {
                converted.content = json!(content_array);
            }
        }

        if !content_array.is_empty() || has_tool_calls {
            result.push(converted);
        }
    }

    result
}

/// Convert internal Tool format to OpenAI's API tool specification
pub fn format_tools(tools: &[Tool]) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        result.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                // do not silently truncate description
                "description": tool.description,
                "parameters": tool.input_schema,
            }
        }));
    }

    Ok(result)
}

/// Convert Databricks' API response to internal Message format
pub fn response_to_message(response: &Value) -> anyhow::Result<Message> {
    let original = &response["choices"][0]["message"];
    let mut content = Vec::new();

    // Handle array-based content
    if let Some(content_array) = original.get("content").and_then(|c| c.as_array()) {
        for content_item in content_array {
            match content_item.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(text) = content_item.get("text").and_then(|t| t.as_str()) {
                        content.push(MessageContent::text(text));
                    }
                }
                Some("reasoning") => {
                    if let Some(summary_array) =
                        content_item.get("summary").and_then(|s| s.as_array())
                    {
                        for summary in summary_array {
                            match summary.get("type").and_then(|t| t.as_str()) {
                                Some("summary_text") => {
                                    let text = summary
                                        .get("text")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or_default();
                                    let signature = summary
                                        .get("signature")
                                        .and_then(|s| s.as_str())
                                        .unwrap_or_default();
                                    content.push(MessageContent::thinking(text, signature));
                                }
                                Some("summary_encrypted_text") => {
                                    if let Some(data) = summary.get("data").and_then(|d| d.as_str())
                                    {
                                        content.push(MessageContent::redacted_thinking(data));
                                    }
                                }
                                _ => continue,
                            }
                        }
                    }
                }
                _ => continue,
            }
        }
    } else if let Some(text) = original.get("content").and_then(|t| t.as_str()) {
        // Handle legacy single string content
        content.push(MessageContent::text(text));
    }

    // Handle tool calls
    if let Some(tool_calls) = original.get("tool_calls") {
        if let Some(tool_calls_array) = tool_calls.as_array() {
            for tool_call in tool_calls_array {
                let id = tool_call["id"].as_str().unwrap_or_default().to_string();
                let function_name = tool_call["function"]["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // Get the raw arguments string from the LLM.
                let arguments_str = tool_call["function"]["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                // If arguments_str is empty, default to an empty JSON object string.
                let arguments_str = if arguments_str.is_empty() {
                    "{}".to_string()
                } else {
                    arguments_str
                };

                if !is_valid_function_name(&function_name) {
                    let error = ToolError::NotFound(format!(
                        "The provided function name '{}' had invalid characters, it must match this regex [a-zA-Z0-9_-]+",
                        function_name
                    ));
                    content.push(MessageContent::tool_request(id, Err(error)));
                } else {
                    match safely_parse_json(&arguments_str) {
                        Ok(params) => {
                            content.push(MessageContent::tool_request(
                                id,
                                Ok(ToolCall::new(&function_name, params)),
                            ));
                        }
                        Err(e) => {
                            let error = ToolError::InvalidParameters(format!(
                                "Could not interpret tool use parameters for id {}: {}. Raw arguments: '{}'",
                                id, e, arguments_str
                            ));
                            content.push(MessageContent::tool_request(id, Err(error)));
                        }
                    }
                }
            }
        }
    }

    Ok(Message::new(
        Role::Assistant,
        chrono::Utc::now().timestamp(),
        content,
    ))
}

#[derive(Serialize, Deserialize, Debug)]
struct DeltaToolCallFunction {
    name: Option<String>,
    arguments: String, // chunk of encoded JSON,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeltaToolCall {
    id: Option<String>,
    function: DeltaToolCallFunction,
    index: Option<i32>,
    r#type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Delta {
    content: Option<String>,
    role: Option<String>,
    tool_calls: Option<Vec<DeltaToolCall>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StreamingChoice {
    delta: Delta,
    index: Option<i32>,
    finish_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StreamingChunk {
    choices: Vec<StreamingChoice>,
    created: Option<i64>,
    id: Option<String>,
    usage: Option<Value>,
    model: String,
}

/// Validates and fixes tool schemas to ensure they have proper parameter structure.
/// If parameters exist, ensures they have properties and required fields, or removes parameters entirely.
pub fn validate_tool_schemas(tools: &mut [Value]) {
    for tool in tools.iter_mut() {
        if let Some(function) = tool.get_mut("function") {
            if let Some(parameters) = function.get_mut("parameters") {
                if parameters.is_object() {
                    ensure_valid_json_schema(parameters);
                }
            }
        }
    }
}

/// Ensures that the given JSON value follows the expected JSON Schema structure.
fn ensure_valid_json_schema(schema: &mut Value) {
    if let Some(params_obj) = schema.as_object_mut() {
        // Check if this is meant to be an object type schema
        let is_object_type = params_obj
            .get("type")
            .and_then(|t| t.as_str())
            .is_none_or(|t| t == "object"); // Default to true if no type is specified

        // Only apply full schema validation to object types
        if is_object_type {
            // Ensure required fields exist with default values
            params_obj.entry("properties").or_insert_with(|| json!({}));
            params_obj.entry("required").or_insert_with(|| json!([]));
            params_obj.entry("type").or_insert_with(|| json!("object"));

            // Recursively validate properties if it exists
            if let Some(properties) = params_obj.get_mut("properties") {
                if let Some(properties_obj) = properties.as_object_mut() {
                    for (_key, prop) in properties_obj.iter_mut() {
                        if prop.is_object()
                            && prop.get("type").and_then(|t| t.as_str()) == Some("object")
                        {
                            ensure_valid_json_schema(prop);
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_lines)]
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    image_format: &ImageFormat,
) -> anyhow::Result<Value, Error> {
    if model_config.model_name.starts_with("o1-mini") {
        return Err(anyhow!(
            "o1-mini model is not currently supported since Goose uses tool calling and o1-mini does not support it. Please use o1 or o3 models instead."
        ));
    }

    let model_name = model_config.model_name.to_string();
    let is_o1 = model_name.starts_with("o1") || model_name.starts_with("goose-o1");
    let is_o3 = model_name.starts_with("o3") || model_name.starts_with("goose-o3");
    let is_claude_sonnet =
        model_name.contains("claude-3-7-sonnet") || model_name.contains("claude-4-sonnet"); // can be goose- or databricks-

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

    let system_message = DatabricksMessage {
        role: if is_o1 || is_o3 {
            "developer"
        } else {
            "system"
        }
        .to_string(),
        content: system.into(),
        tool_calls: None,
        tool_call_id: None,
    };

    let messages_spec = format_messages(messages, image_format);
    let mut tools_spec = if !tools.is_empty() {
        format_tools(tools)?
    } else {
        vec![]
    };

    // Validate tool schemas
    validate_tool_schemas(&mut tools_spec);

    let mut messages_array = vec![system_message];
    messages_array.extend(messages_spec);

    let mut payload = json!({
        "model": model_name,
        "messages": messages_array
    });

    if let Some(effort) = reasoning_effort {
        payload
            .as_object_mut()
            .unwrap()
            .insert("reasoning_effort".to_string(), json!(effort));
    }

    if !tools_spec.is_empty() {
        payload
            .as_object_mut()
            .unwrap()
            .insert("tools".to_string(), json!(tools_spec));
    }

    // Add thinking parameters for Claude 3.7 Sonnet model when requested
    let is_thinking_enabled = std::env::var("CLAUDE_THINKING_ENABLED").is_ok();
    if is_claude_sonnet && is_thinking_enabled {
        // Minimum budget_tokens is 1024
        let budget_tokens = std::env::var("CLAUDE_THINKING_BUDGET")
            .unwrap_or_else(|_| "16000".to_string())
            .parse()
            .unwrap_or(16000);

        // For Claude models with thinking enabled, we need to add max_tokens + budget_tokens
        // Default to 8192 (Claude max output) + budget if not specified
        let max_completion_tokens = model_config.max_tokens.unwrap_or(8192);
        payload.as_object_mut().unwrap().insert(
            "max_tokens".to_string(),
            json!(max_completion_tokens + budget_tokens),
        );

        payload.as_object_mut().unwrap().insert(
            "thinking".to_string(),
            json!({
                "type": "enabled",
                "budget_tokens": budget_tokens
            }),
        );

        // Temperature is fixed to 2 when using claude 3.7 thinking with Databricks
        payload
            .as_object_mut()
            .unwrap()
            .insert("temperature".to_string(), json!(2));
    } else {
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
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;
    use rmcp::object;
    use serde_json::json;

    #[test]
    fn test_validate_tool_schemas() {
        // Test case 1: Empty parameters object
        // Input JSON with an incomplete parameters object
        let mut actual = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object"
                }
            }
        })];

        // Run the function to validate and update schemas
        validate_tool_schemas(&mut actual);

        // Expected JSON after validation
        let expected = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        })];

        // Compare entire JSON structures instead of individual fields
        assert_eq!(actual, expected);

        // Test case 2: Missing type field
        let mut tools = vec![json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "properties": {}
                }
            }
        })];

        validate_tool_schemas(&mut tools);

        let params = tools[0]["function"]["parameters"].as_object().unwrap();
        assert_eq!(params["type"], "object");

        // Test case 3: Complete valid schema should remain unchanged
        let original_schema = json!({
            "type": "function",
            "function": {
                "name": "test_func",
                "description": "test description",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {
                            "type": "string",
                            "description": "City and country"
                        }
                    },
                    "required": ["location"]
                }
            }
        });

        let mut tools = vec![original_schema.clone()];
        validate_tool_schemas(&mut tools);
        assert_eq!(tools[0], original_schema);
    }

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

    #[test]
    fn test_format_messages() -> anyhow::Result<()> {
        let message = Message::user().with_text("Hello");
        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0].role, "user");
        assert_eq!(spec[0].content, "Hello");
        Ok(())
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
        let tool = Tool::new(
            "test_tool",
            "A test tool",
            object!({
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
            &request.id
        } else {
            panic!("should be tool request");
        };

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

        let as_value =
            serde_json::to_value(format_messages(&messages, &ImageFormat::OpenAi)).unwrap();
        let spec = as_value.as_array().unwrap();

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
        let mut messages = vec![Message::assistant().with_tool_request(
            "tool1",
            Ok(ToolCall::new("example", json!({"param1": "value1"}))),
        )];

        // Get the ID from the tool request to use in the response
        let tool_id = if let MessageContent::ToolRequest(request) = &messages[0].content[0] {
            &request.id
        } else {
            panic!("should be tool request");
        };

        messages
            .push(Message::user().with_tool_response(tool_id, Ok(vec![Content::text("Result")])));

        let as_value =
            serde_json::to_value(format_messages(&messages, &ImageFormat::OpenAi)).unwrap();
        let spec = as_value.as_array().unwrap();

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
        let tool1 = Tool::new(
            "test_tool",
            "Test tool",
            object!({
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
            object!({
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
        let spec = format_tools(&[])?;
        assert!(spec.is_empty());
        Ok(())
    }

    #[test]
    fn test_format_messages_with_image_path() -> anyhow::Result<()> {
        // Create a temporary PNG file with valid PNG magic numbers
        let temp_dir = tempfile::tempdir()?;
        let png_path = temp_dir.path().join("test.png");
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, // PNG magic number
            0x0D, 0x0A, 0x1A, 0x0A, // PNG header
            0x00, 0x00, 0x00, 0x0D, // Rest of fake PNG data
        ];
        std::fs::write(&png_path, png_data)?;
        let png_path_str = png_path.to_str().unwrap();

        // Create message with image path
        let message = Message::user().with_text(format!("Here is an image: {}", png_path_str));
        let as_value =
            serde_json::to_value(format_messages(&[message], &ImageFormat::OpenAi)).unwrap();
        let spec = as_value.as_array().unwrap();

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");

        // Content should be an array with text and image
        let content = spec[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"].as_str().unwrap().contains(png_path_str));
        assert_eq!(content[1]["type"], "image_url");
        assert!(content[1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));

        Ok(())
    }

    #[test]
    fn test_response_to_message_text() -> anyhow::Result<()> {
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

        let message = response_to_message(&response)?;
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
        let message = response_to_message(&response)?;

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

        let message = response_to_message(&response)?;

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

        let message = response_to_message(&response)?;

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
    fn test_response_to_message_empty_argument() -> anyhow::Result<()> {
        let mut response: Value = serde_json::from_str(OPENAI_TOOL_USE_RESPONSE)?;
        response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"] =
            serde_json::Value::String("".to_string());

        let message = response_to_message(&response)?;

        if let MessageContent::ToolRequest(request) = &message.content[0] {
            let tool_call = request.tool_call.as_ref().unwrap();
            assert_eq!(tool_call.name, "example_fn");
            assert_eq!(tool_call.arguments, json!({}));
        } else {
            panic!("Expected ToolRequest content");
        }

        Ok(())
    }

    #[test]
    fn test_create_request_gpt_4o() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "gpt-4o".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "gpt-4o",
            "messages": [
                {
                    "role": "system",
                    "content": "system"
                }
            ],
            "max_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_o1_default() -> anyhow::Result<()> {
        // Test default medium reasoning effort for O1 model
        let model_config = ModelConfig {
            model_name: "o1".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "o1",
            "messages": [
                {
                    "role": "developer",
                    "content": "system"
                }
            ],
            "reasoning_effort": "medium",
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_create_request_o3_custom_reasoning_effort() -> anyhow::Result<()> {
        // Test custom reasoning effort for O3 model
        let model_config = ModelConfig {
            model_name: "o3-mini-high".to_string(),
            context_limit: Some(4096),
            temperature: None,
            max_tokens: Some(1024),
            toolshim: false,
            toolshim_model: None,
        };
        let request = create_request(&model_config, "system", &[], &[], &ImageFormat::OpenAi)?;
        let obj = request.as_object().unwrap();
        let expected = json!({
            "model": "o3-mini",
            "messages": [
                {
                    "role": "developer",
                    "content": "system"
                }
            ],
            "reasoning_effort": "high",
            "max_completion_tokens": 1024
        });

        for (key, value) in expected.as_object().unwrap() {
            assert_eq!(obj.get(key).unwrap(), value);
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_claude_thinking() -> anyhow::Result<()> {
        let response = json!({
            "model": "us.anthropic.claude-3-7-sonnet-20250219-v1:0",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "reasoning",
                            "summary": [
                                {
                                    "type": "summary_text",
                                    "text": "Test thinking content",
                                    "signature": "test-signature"
                                }
                            ]
                        },
                        {
                            "type": "text",
                            "text": "Regular text content"
                        }
                    ]
                },
                "index": 0,
                "finish_reason": "stop"
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContent::Thinking(thinking) = &message.content[0] {
            assert_eq!(thinking.thinking, "Test thinking content");
            assert_eq!(thinking.signature, "test-signature");
        } else {
            panic!("Expected Thinking content");
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Regular text content");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }

    #[test]
    fn test_response_to_message_claude_encrypted_thinking() -> anyhow::Result<()> {
        let response = json!({
            "model": "claude-3-7-sonnet-20250219",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "reasoning",
                            "summary": [
                                {
                                    "type": "summary_encrypted_text",
                                    "data": "E23sQFCkYIARgCKkATCHitsdf327Ber3v4NYUq2"
                                }
                            ]
                        },
                        {
                            "type": "text",
                            "text": "Regular text content"
                        }
                    ]
                },
                "index": 0,
                "finish_reason": "stop"
            }]
        });

        let message = response_to_message(&response)?;
        assert_eq!(message.content.len(), 2);

        if let MessageContent::RedactedThinking(redacted) = &message.content[0] {
            assert_eq!(redacted.data, "E23sQFCkYIARgCKkATCHitsdf327Ber3v4NYUq2");
        } else {
            panic!("Expected RedactedThinking content");
        }

        if let MessageContent::Text(text) = &message.content[1] {
            assert_eq!(text.text, "Regular text content");
        } else {
            panic!("Expected Text content");
        }

        Ok(())
    }
}
