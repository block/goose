use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use aws_config;
use aws_sdk_bedrockruntime::config::ProvideCredentials;
use aws_sdk_sagemakerruntime::Client as SageMakerClient;
use mcp_core::Tool;
use serde_json::{json, Value};
use tokio::time::sleep;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::emit_debug_trace;
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use mcp_core::content::TextContent;
use mcp_core::role::Role;
use chrono::Utc;

pub const SAGEMAKER_TGI_DOC_LINK: &str = 
    "https://docs.aws.amazon.com/sagemaker/latest/dg/realtime-endpoints.html";

pub const SAGEMAKER_TGI_DEFAULT_MODEL: &str = "sagemaker-tgi-endpoint";

#[derive(Debug, serde::Serialize)]
pub struct SageMakerTgiProvider {
    #[serde(skip)]
    sagemaker_client: SageMakerClient,
    endpoint_name: String,
    model: ModelConfig,
}

impl SageMakerTgiProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        
        // Get SageMaker endpoint name (just the name, not full URL)
        let endpoint_name: String = config.get_param("SAGEMAKER_ENDPOINT_NAME")
            .map_err(|_| anyhow::anyhow!("SAGEMAKER_ENDPOINT_NAME is required for SageMaker TGI provider"))?;

        // Attempt to load config and secrets to get AWS_ prefixed keys
        let set_aws_env_vars = |res: Result<HashMap<String, Value>, _>| {
            if let Ok(map) = res {
                map.into_iter()
                    .filter(|(key, _)| key.starts_with("AWS_"))
                    .filter_map(|(key, value)| value.as_str().map(|s| (key, s.to_string())))
                    .for_each(|(key, s)| std::env::set_var(key, s));
            }
        };

        set_aws_env_vars(config.load_values());
        set_aws_env_vars(config.load_secrets());

        let aws_config = futures::executor::block_on(aws_config::load_from_env());

        // Validate credentials
        futures::executor::block_on(
            aws_config
                .credentials_provider()
                .unwrap()
                .provide_credentials(),
        )?;

        // Create client with longer timeout for model initialization
        let timeout_config = aws_config::timeout::TimeoutConfig::builder()
            .operation_timeout(Duration::from_secs(300)) // 5 minutes for cold starts
            .build();
        
        let config_with_timeout = aws_config.into_builder()
            .timeout_config(timeout_config)
            .build();
            
        let sagemaker_client = SageMakerClient::new(&config_with_timeout);

        Ok(Self {
            sagemaker_client,
            endpoint_name,
            model,
        })
    }

    fn create_tgi_request(&self, system: &str, messages: &[Message], tools: &[Tool]) -> Result<Value> {
        // Check if we should use tool calling format
        let use_tool_calling = self.supports_tool_calling();
        
        if use_tool_calling && !tools.is_empty() {
            // Use OpenAI-compatible format for models that support it
            return self.create_tool_calling_request(system, messages, tools);
        }
        
        // Create a simplified prompt for basic TGI models
        // Skip the complex system prompt and tool descriptions that cause the model to mimic tool formats
        let mut prompt = String::new();
        
        // Use a very simple system prompt if provided
        if !system.is_empty() && !system.contains("Available tools") && system.len() < 200 {
            prompt.push_str(&format!("System: {}\n\n", system));
        } else {
            // Use a minimal system prompt for TGI
            prompt.push_str("System: You are a helpful AI assistant.\n\n");
        }

        // Only include the most recent user messages to avoid overwhelming the model
        let recent_messages: Vec<_> = messages.iter().rev().take(3).collect();
        for message in recent_messages.iter().rev() {
            match &message.role {
                Role::User => {
                    prompt.push_str("User: ");
                    for content in &message.content {
                        if let MessageContent::Text(text) = content {
                            prompt.push_str(&text.text);
                        }
                    }
                    prompt.push_str("\n\n");
                }
                Role::Assistant => {
                    prompt.push_str("Assistant: ");
                    for content in &message.content {
                        if let MessageContent::Text(text) = content {
                            // Skip responses that look like tool descriptions
                            if !text.text.contains("__") && !text.text.contains("Available tools") {
                                prompt.push_str(&text.text);
                            }
                        }
                    }
                    prompt.push_str("\n\n");
                }
            }
        }

        prompt.push_str("Assistant: ");

        // Skip tool descriptions entirely for TGI models to avoid confusion
        // TGI models don't support tools natively and including tool descriptions
        // causes them to mimic that format in their responses

        // Build TGI request with reasonable parameters
        let request = json!({
            "inputs": prompt,
            "parameters": {
                "max_new_tokens": self.model.max_tokens.unwrap_or(150),
                "temperature": self.model.temperature.unwrap_or(0.7),
                "do_sample": true,
                "return_full_text": false
            }
        });

        Ok(request)
    }

    fn supports_tool_calling(&self) -> bool {
        // Check if the model supports tool calling
        // You can configure this via environment variable or model name detection
        std::env::var("GOOSE_SAGEMAKER_SUPPORTS_TOOLS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
    }

    fn create_tool_calling_request(&self, system: &str, messages: &[Message], tools: &[Tool]) -> Result<Value> {
        // Convert messages to OpenAI format
        let mut openai_messages = Vec::new();
        
        if !system.is_empty() {
            openai_messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        for message in messages {
            match &message.role {
                Role::User => {
                    let content = message.content.iter()
                        .filter_map(|c| {
                            if let MessageContent::Text(text) = c {
                                Some(text.text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    openai_messages.push(json!({
                        "role": "user",
                        "content": content
                    }));
                }
                Role::Assistant => {
                    let content = message.content.iter()
                        .filter_map(|c| {
                            if let MessageContent::Text(text) = c {
                                Some(text.text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    if !content.is_empty() {
                        openai_messages.push(json!({
                            "role": "assistant",
                            "content": content
                        }));
                    }
                }
            }
        }

        // Convert tools to OpenAI format, cleaning up unsupported schema formats
        let openai_tools: Vec<Value> = tools.iter().map(|tool| {
            let cleaned_schema = self.clean_schema_for_tgi(&tool.input_schema);
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": cleaned_schema
                }
            })
        }).collect();

        // Create OpenAI-compatible request
        let request = json!({
            "messages": openai_messages,
            "tools": openai_tools,
            "tool_choice": "auto",
            "max_tokens": self.model.max_tokens.unwrap_or(150),
            "temperature": self.model.temperature.unwrap_or(0.7)
        });

        Ok(request)
    }

    fn clean_schema_for_tgi(&self, schema: &Value) -> Value {
        // Recursively clean JSON schema to remove formats unsupported by TGI/Outlines
        match schema {
            Value::Object(obj) => {
                let mut cleaned = serde_json::Map::new();
                for (key, value) in obj {
                    if key == "format" {
                        // Skip format constraints that TGI doesn't support
                        if let Some(format_str) = value.as_str() {
                            match format_str {
                                "uri" | "email" | "date" | "date-time" | "uuid" => {
                                    // Skip these unsupported formats
                                    continue;
                                }
                                _ => {
                                    // Keep supported formats
                                    cleaned.insert(key.clone(), value.clone());
                                }
                            }
                        }
                    } else {
                        // Recursively clean nested objects/arrays
                        cleaned.insert(key.clone(), self.clean_schema_for_tgi(value));
                    }
                }
                Value::Object(cleaned)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.clean_schema_for_tgi(v)).collect())
            }
            _ => schema.clone(),
        }
    }

    async fn invoke_endpoint(&self, payload: Value) -> Result<Value, ProviderError> {
        let body = serde_json::to_string(&payload)
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to serialize request: {}", e)))?;

        let response = self.sagemaker_client
            .invoke_endpoint()
            .endpoint_name(&self.endpoint_name)
            .content_type("application/json")
            .body(body.into_bytes().into())
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("SageMaker invoke failed: {}", e)))?;

        let response_body = response.body.as_ref()
            .ok_or_else(|| ProviderError::RequestFailed("Empty response body".to_string()))?;
        let response_text = std::str::from_utf8(response_body.as_ref())
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to decode response: {}", e)))?;

        serde_json::from_str(response_text)
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse response JSON: {}", e)))
    }

    fn parse_tgi_response(&self, response: Value) -> Result<Message, ProviderError> {
        // Check if this is an OpenAI-compatible response (tool calling format)
        if response.get("choices").is_some() {
            return self.parse_openai_response(response);
        }

        // Handle standard TGI response: [{"generated_text": "..."}]
        let response_array = response.as_array()
            .ok_or_else(|| ProviderError::RequestFailed("Expected array response".to_string()))?;

        if response_array.is_empty() {
            return Err(ProviderError::RequestFailed("Empty response array".to_string()));
        }

        let first_result = &response_array[0];
        let generated_text = first_result.get("generated_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::RequestFailed("No generated_text in response".to_string()))?;

        Ok(Message {
            role: Role::Assistant,
            created: Utc::now().timestamp(),
            content: vec![MessageContent::Text(TextContent {
                text: generated_text.to_string(),
                annotations: None,
            })],
        })
    }

    fn parse_openai_response(&self, response: Value) -> Result<Message, ProviderError> {
        let choices = response.get("choices")
            .and_then(|c| c.as_array())
            .ok_or_else(|| ProviderError::RequestFailed("No choices in OpenAI response".to_string()))?;

        if choices.is_empty() {
            return Err(ProviderError::RequestFailed("Empty choices array".to_string()));
        }

        let choice = &choices[0];
        let message = choice.get("message")
            .ok_or_else(|| ProviderError::RequestFailed("No message in choice".to_string()))?;

        let mut content = Vec::new();

        // Handle text content
        if let Some(text_content) = message.get("content").and_then(|c| c.as_str()) {
            if !text_content.is_empty() {
                content.push(MessageContent::Text(TextContent {
                    text: text_content.to_string(),
                    annotations: None,
                }));
            }
        }

        // Handle tool calls
        if let Some(tool_calls) = message.get("tool_calls").and_then(|tc| tc.as_array()) {
            for tool_call in tool_calls {
                if let (Some(id), Some(function)) = (
                    tool_call.get("id").and_then(|i| i.as_str()),
                    tool_call.get("function")
                ) {
                    if let (Some(name), Some(arguments)) = (
                        function.get("name").and_then(|n| n.as_str()),
                        function.get("arguments").and_then(|a| a.as_str())
                    ) {
                        // Parse the tool call arguments
                        let args: Value = serde_json::from_str(arguments)
                            .unwrap_or_else(|_| json!({}));

                        let tool_call = mcp_core::tool::ToolCall {
                            name: name.to_string(),
                            arguments: args,
                        };

                        content.push(MessageContent::tool_request(id, Ok(tool_call)));
                    }
                }
            }
        }

        Ok(Message {
            role: Role::Assistant,
            created: Utc::now().timestamp(),
            content,
        })
    }
}

impl Default for SageMakerTgiProvider {
    fn default() -> Self {
        let model = ModelConfig::new(SageMakerTgiProvider::metadata().default_model);
        SageMakerTgiProvider::from_env(model).expect("Failed to initialize SageMaker TGI provider")
    }
}

#[async_trait]
impl Provider for SageMakerTgiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "sagemaker_tgi",
            "Amazon SageMaker TGI",
            "Run Text Generation Inference models through Amazon SageMaker endpoints. Requires AWS credentials and a SageMaker endpoint URL.",
            SAGEMAKER_TGI_DEFAULT_MODEL,
            vec![SAGEMAKER_TGI_DEFAULT_MODEL],
            SAGEMAKER_TGI_DOC_LINK,
            vec![
                ConfigKey::new("SAGEMAKER_ENDPOINT_NAME", false, false, None),
                ConfigKey::new("AWS_REGION", true, false, Some("us-east-1")),
                ConfigKey::new("AWS_PROFILE", true, false, Some("default")),
                ConfigKey::new("GOOSE_SAGEMAKER_SUPPORTS_TOOLS", true, false, Some("false")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let model_name = &self.model.model_name;

        let request_payload = self.create_tgi_request(system, messages, tools)
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to create request: {}", e)))?;

        // Retry configuration
        const MAX_RETRIES: u32 = 3;
        const INITIAL_BACKOFF_MS: u64 = 1000; // 1 second
        const MAX_BACKOFF_MS: u64 = 30000; // 30 seconds

        let mut attempts = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        loop {
            attempts += 1;

            match self.invoke_endpoint(request_payload.clone()).await {
                Ok(response) => {
                    let message = self.parse_tgi_response(response)?;

                    // TGI doesn't provide usage statistics, so we estimate
                    let usage = Usage {
                        input_tokens: Some(0), // Would need to tokenize input to get accurate count
                        output_tokens: Some(0), // Would need to tokenize output to get accurate count
                        total_tokens: Some(0),
                    };

                    // Add debug trace
                    let debug_payload = serde_json::json!({
                        "system": system,
                        "messages": messages,
                        "tools": tools
                    });
                    emit_debug_trace(
                        &self.model,
                        &debug_payload,
                        &serde_json::to_value(&message).unwrap_or_default(),
                        &usage,
                    );

                    let provider_usage = ProviderUsage::new(model_name.to_string(), usage);
                    return Ok((message, provider_usage));
                }
                Err(err) => {
                    if attempts > MAX_RETRIES {
                        return Err(err);
                    }

                    // Log retry attempt
                    tracing::warn!(
                        "SageMaker TGI request failed (attempt {}/{}), retrying in {} ms: {:?}",
                        attempts,
                        MAX_RETRIES,
                        backoff_ms,
                        err
                    );

                    // Wait before retry
                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                }
            }
        }
    }
}