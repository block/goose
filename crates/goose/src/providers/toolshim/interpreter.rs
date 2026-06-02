use super::*;

/// Environment variables that affect behavior:
/// - GOOSE_TOOLSHIM: When set to "true" or "1", enables using the tool shim in the standard OllamaProvider (default: false)
/// - GOOSE_TOOLSHIM_OLLAMA_MODEL: Ollama model to use as the tool interpreter (default: DEFAULT_INTERPRETER_MODEL)
/// A trait for models that can interpret text into structured tool call JSON format
#[async_trait::async_trait]
pub trait ToolInterpreter {
    /// Interpret potential tool calls from text and convert them to proper tool call JSON format
    async fn interpret_to_tool_calls(
        &self,
        content: &str,
        tools: &[Tool],
    ) -> Result<Vec<CallToolRequestParams>, ProviderError>;
}

/// Ollama-specific implementation of the ToolInterpreter trait
pub struct OllamaInterpreter {
    pub(super) client: Client,
    pub(super) base_url: String,
}

/// Local llama.cpp implementation of the ToolInterpreter trait.
pub struct LocalInterpreter {
    pub(super) model: String,
}

impl LocalInterpreter {
    pub fn new() -> Result<Self, ProviderError> {
        Ok(Self {
            model: resolve_local_interpreter_model()?,
        })
    }

    async fn infer_structured_response(
        &self,
        format_instruction: &str,
    ) -> Result<String, ProviderError> {
        let model_config = ModelConfig::new(&self.model)
            .map_err(|e| ProviderError::RequestFailed(format!("Model config error: {e}")))?
            .with_canonical_limits("local")
            .with_toolshim(false)
            .with_toolshim_model(None);

        let provider = crate::providers::init::create("local", model_config, vec![])
            .await
            .map_err(|e| {
                ProviderError::RequestFailed(format!(
                    "Failed to create local interpreter provider: {e}"
                ))
            })?;

        let request_messages = vec![Message::user().with_text(format_instruction)];
        let mut stream = provider
            .stream(
                &provider.get_model_config(),
                "toolshim-local",
                "",
                &request_messages,
                &[],
            )
            .await?;

        let mut content = String::new();
        while let Some(chunk) = stream.next().await {
            let (message, _) = chunk?;
            if let Some(message) = message {
                for part in message.content {
                    if let MessageContent::Text(text) = part {
                        content.push_str(&text.text);
                    }
                }
            }
        }

        Ok(content)
    }
}

impl OllamaInterpreter {
    pub fn new() -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = Self::get_ollama_base_url()?;

        Ok(Self { client, base_url })
    }

    /// Get the Ollama base URL from existing config or use default values
    fn get_ollama_base_url() -> Result<String, ProviderError> {
        let config = crate::config::Config::global();
        let host: String = config
            .get_param("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());

        // Format the URL correctly with http:// prefix if needed
        let base = if host.starts_with("http://") || host.starts_with("https://") {
            &host
        } else {
            &format!("http://{}", host)
        };

        let mut base_url = url::Url::parse(base)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Set the default port if missing
        // Don't add default port if:
        // 1. URL explicitly ends with standard ports (:80 or :443)
        // 2. URL uses HTTPS (which implicitly uses port 443)
        let explicit_default_port = host.ends_with(":80") || host.ends_with(":443");
        let is_https = base_url.scheme() == "https";

        if base_url.port().is_none() && !explicit_default_port && !is_https {
            base_url.set_port(Some(OLLAMA_DEFAULT_PORT)).map_err(|_| {
                ProviderError::RequestFailed("Failed to set default port".to_string())
            })?;
        }

        Ok(base_url.to_string())
    }

    fn tool_structured_output_format_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "tool_calls": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The name of the tool to call"
                            },
                            "arguments": {
                                "type": "object",
                                "description": "The arguments to pass to the tool"
                            }
                        },
                        "required": ["name", "arguments"]
                    }
                }
            },
            "required": ["tool_calls"]
        })
    }

    async fn post_structured(
        &self,
        system_prompt: &str,
        format_instruction: &str,
        format_schema: Value,
        model: &str,
    ) -> Result<Value, ProviderError> {
        let base_url = self.base_url.trim_end_matches('/');
        let url = format!("{}/api/chat", base_url);

        let mut messages = Vec::new();
        let user_message = Message::user().with_text(format_instruction);
        messages.push(user_message);

        let model_config = ModelConfig::new(model)
            .map_err(|e| ProviderError::RequestFailed(format!("Model config error: {e}")))?
            .with_canonical_limits("ollama");

        let mut payload = create_request(
            &model_config,
            system_prompt,
            &messages,
            &[], // No tools
            &super::utils::ImageFormat::OpenAi,
            false,
        )?;

        payload["stream"] = json!(false); // needed for the /api/chat endpoint to work
        payload["format"] = format_schema;

        tracing::info!(
            "Tool interpreter payload: {}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();

            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Could not read error response".to_string(),
            };

            return Err(ProviderError::RequestFailed(format!(
                "Ollama structured API returned error status {}: {}",
                status, error_text
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse Ollama structured API response: {e}"
            ))
        })?;

        Ok(response_json)
    }

    pub fn process_interpreter_response(
        response: &Value,
    ) -> Result<Vec<CallToolRequestParams>, ProviderError> {
        let mut tool_calls = Vec::new();
        tracing::info!(
            "Tool interpreter response is {}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );
        // Extract tool_calls array from the response
        if response.get("message").is_some() && response["message"].get("content").is_some() {
            let content = response["message"]["content"].as_str().unwrap_or_default();

            // Try to parse the content as JSON
            if let Ok(content_json) = serde_json::from_str::<Value>(content) {
                // Check for the format with tool_calls array inside an object
                if content_json.is_object() && content_json.get("tool_calls").is_some() {
                    // Process each tool call in the array
                    if let Some(tool_calls_array) = content_json["tool_calls"].as_array() {
                        for item in tool_calls_array {
                            if item.is_object()
                                && item.get("name").is_some()
                                && item.get("arguments").is_some()
                            {
                                let name = item["name"].as_str().unwrap_or_default().to_string();
                                let arguments = item["arguments"].clone();

                                // Add the tool call to our result vector
                                tool_calls.push(
                                    CallToolRequestParams::new(name)
                                        .with_arguments(object(arguments)),
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(tool_calls)
    }
}

#[async_trait::async_trait]
impl ToolInterpreter for OllamaInterpreter {
    async fn interpret_to_tool_calls(
        &self,
        last_assistant_msg: &str,
        tools: &[Tool],
    ) -> Result<Vec<CallToolRequestParams>, ProviderError> {
        if tools.is_empty() {
            return Ok(vec![]);
        }

        // Create the system prompt
        let system_prompt = "If there is detectable JSON-formatted tool requests, write them into valid JSON tool calls in the following format:
{{
  \"tool_calls\": [
    {{
      \"name\": \"tool_name\",
      \"arguments\": {{
        \"param1\": \"value1\",
        \"param2\": \"value2\"
      }}
    }}
  ]
}}

Otherwise, if no JSON tool requests are provided, use the no-op tool:
{{
  \"tool_calls\": [
    {{
    \"name\": \"noop\",
      \"arguments\": {{
      }}
    }}]
}}
";

        // Create enhanced content with instruction to output tool calls as JSON
        let format_instruction = format!("{}\nRequest: {}\n\n", system_prompt, last_assistant_msg);

        // Define the JSON schema for tool call format
        let format_schema = OllamaInterpreter::tool_structured_output_format_schema();

        // Determine which model to use for interpretation (from env var or default)
        let interpreter_model = std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL")
            .unwrap_or_else(|_| DEFAULT_INTERPRETER_MODEL_OLLAMA.to_string());

        // Make a call to ollama with structured output
        let interpreter_response = self
            .post_structured("", &format_instruction, format_schema, &interpreter_model)
            .await?;

        // Process the interpreter response to get tool calls directly
        let tool_calls = OllamaInterpreter::process_interpreter_response(&interpreter_response)?;

        Ok(tool_calls)
    }
}

#[async_trait::async_trait]
impl ToolInterpreter for LocalInterpreter {
    async fn interpret_to_tool_calls(
        &self,
        last_assistant_msg: &str,
        tools: &[Tool],
    ) -> Result<Vec<CallToolRequestParams>, ProviderError> {
        if tools.is_empty() {
            return Ok(vec![]);
        }

        let system_prompt = "If there is detectable JSON-formatted tool requests, write them into valid JSON tool calls in the following format:
{{
    \"tool_calls\": [
        {{
            \"name\": \"tool_name\",
            \"arguments\": {{
                \"param1\": \"value1\",
                \"param2\": \"value2\"
            }}
        }}
    ]
}}

Otherwise, if no JSON tool requests are provided, use the no-op tool:
{{
    \"tool_calls\": [
        {{
        \"name\": \"noop\",
            \"arguments\": {{
            }}
        }}]
}}
";

        let format_instruction = format!("{}\nRequest: {}\n\n", system_prompt, last_assistant_msg);
        let content = self.infer_structured_response(&format_instruction).await?;
        let response = json!({ "message": { "content": content } });

        OllamaInterpreter::process_interpreter_response(&response)
    }
}
