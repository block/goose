use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use crate::providers::errors::ProviderError;
use crate::providers::utils::{
    convert_image, is_valid_function_name, sanitize_function_name, ImageFormat,
};
use anyhow::{anyhow, Error};
use mcp_core::ToolError;
use mcp_core::{Content, Role, Tool, ToolCall};
use serde_json::{json, Value};

/// Convert internal Message format to OpenAI's API message specification
///   some openai compatible endpoints use the anthropic image spec at the content level
///   even though the message structure is otherwise following openai, the enum switches this
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
                    if !text.text.is_empty() {
                        converted["content"] = json!(text.text);
                    }
                }
                MessageContent::ToolRequest(request) => match &request.tool_call {
                    Ok(tool_call) => {
                        let sanitized_name = sanitize_function_name(&tool_call.name);
                        let tool_calls = converted
                            .as_object_mut()
                            .unwrap()
                            .entry("tool_calls")
                            .or_insert(json!([]));

                        tool_calls.as_array_mut().unwrap().push(json!({
                            "id": request.id,
                            "type": "function",
                            "function": {
                                "name": sanitized_name,
                                "arguments": tool_call.arguments.to_string(),
                            }
                        }));
                    }
                    Err(e) => {
                        output.push(json!({
                            "role": "tool",
                            "content": format!("Error: {}", e),
                            "tool_call_id": request.id
                        }));
                    }
                },
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
                                .map(|content| content.unannotated())
                                .collect();

                            // Process all content, replacing images with placeholder text
                            let mut tool_content = Vec::new();
                            let mut image_messages = Vec::new();

                            for content in abridged {
                                match content {
                                    Content::Image(image) => {
                                        // Add placeholder text in the tool response
                                        tool_content.push(Content::text("This tool result included an image that is uploaded in the next message."));

                                        // Create a separate image message
                                        image_messages.push(json!({
                                            "role": "user",
                                            "content": [convert_image(&image, image_format)]
                                        }));
                                    }
                                    Content::Resource(resource) => {
                                        tool_content.push(Content::text(resource.get_text()));
                                    }
                                    _ => {
                                        tool_content.push(content);
                                    }
                                }
                            }
                            let tool_response_content: Value = json!(tool_content
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
                            // Then add any image messages that need to follow
                            output.extend(image_messages);
                        }
                        Err(e) => {
                            // A tool result error is shown as output so the model can interpret the error message
                            output.push(json!({
                                "role": "tool",
                                "content": format!("The tool call returned the following error:\n{}", e),
                                "tool_call_id": response.id
                            }));
                        }
                    }
                }
                MessageContent::Image(image) => {
                    // Handle direct image content
                    converted["content"] = json!([convert_image(image, image_format)]);
                }
            }
        }

        if converted.get("content").is_some() || converted.get("tool_calls").is_some() {
            output.insert(0, converted);
        }
        messages_spec.extend(output);
    }

    messages_spec
}

/// Convert internal Tool format to OpenAI's API tool specification
pub fn format_tools(tools: &[Tool]) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
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

/// Convert OpenAI's API response to internal Message format
pub fn response_to_message(response: Value) -> anyhow::Result<Message> {
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
                let base_name = parts[..parts.len()-1].join("-");
                (base_name, Some(last_part.to_string()))
            },
            _ => (model_config.model_name.to_string(), Some("medium".to_string()))
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
        "role": if is_o1 || is_o3{ "developer" } else { "system" },
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
        payload.as_object_mut().unwrap()
            .insert("reasoning_effort".to_string(), json!(effort));
    }

    // Add tools if present
    if !tools_spec.is_empty() {
        payload.as_object_mut().unwrap()
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
    use mcp_core::content::Content;
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

    const EPSILON: f64 = 1e-6;  // More lenient epsilon for float comparison

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
        let message = Message::user().with_text("Hello");
        let spec = format_messages(&[message], &ImageFormat::OpenAi);

        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["role"], "user");
        assert_eq!(spec[0]["content"], "Hello");
        Ok(())
    }

    #[test]
    fn test_format_tools() -> anyhow::Result<()> {
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
        let spec = format_tools(&[])?;
        assert!(spec.is_empty());
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
        assert!(result.unwrap_err().to_string().contains("o1-mini model is not currently supported"));
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
