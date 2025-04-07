use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat, ImageFormat};
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use mcp_core::tool::{Tool, ToolCall};
use mcp_core::{Role, ToolError};

pub const HUGGINGFACE_DEFAULT_MODEL: &str = "deepseek-ai/DeepSeek-V3-0324-fast";
pub const HUGGINGFACE_KNOWN_MODELS: &[&str] = &[
    "deepseek-ai/DeepSeek-V3-0324-fast",
    "mistralai/Mistral-7B-Instruct-v0.2",
    "meta-llama/Llama-2-70b-chat-hf",
    "google/gemma-7b",
    "google/gemma-2b",
];

pub const HUGGINGFACE_DOC_URL: &str = "https://huggingface.co/models";

#[derive(Debug, serde::Serialize)]
pub struct HuggingFaceProvider {
    #[serde(skip)]
    client: Client,
    api_key: String,
    provider: String,
    model: ModelConfig,
}

impl Default for HuggingFaceProvider {
    fn default() -> Self {
        let model = ModelConfig::new(HuggingFaceProvider::metadata().default_model);
        HuggingFaceProvider::from_env(model).expect("Failed to initialize HuggingFace provider")
    }
}

impl HuggingFaceProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("HUGGINGFACE_TOKEN")?;
        let provider: String = config.get_param("HUGGINGFACE_PROVIDER").unwrap_or_else(|_| "nebius".to_string());
        let timeout_secs: u64 = config.get_param("HUGGINGFACE_TIMEOUT").unwrap_or(600);
        
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        println!("Using HuggingFace provider: {}", provider);

        Ok(Self {
            client,
            api_key,
            provider,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = format!("https://router.huggingface.co/{}/v1/chat/completions", self.provider);
        
        let request = self
            .client
            .post(&base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&payload);

        let response = request.send().await?;
        handle_response_openai_compat(response).await
    }
    
    /// Determines if the current model is a DeepSeek model
    fn is_deepseek_model(&self) -> bool {
        self.model.model_name.contains("deepseek") || 
        self.model.model_name.contains("DeepSeek")
    }
    
    /// Creates a system prompt with tools embedded for DeepSeek models
    fn create_system_prompt_with_tools(&self, system: &str, tools: &[Tool]) -> String {
        if tools.is_empty() {
            return system.to_string();
        }
        
        // Start with the original system prompt
        let mut tool_system_prompt = format!("{}\n\n## Tools\n", system);
        
        // Add function section
        tool_system_prompt.push_str("\n### Function\n\n");
        tool_system_prompt.push_str("You have the following functions available:\n\n");
        
        // Add each tool as a function definition in the format shown in the example
        for tool in tools {
            tool_system_prompt.push_str(&format!("- `{}`:\n```json\n{}\n```\n\n", 
                tool.name,
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema
                }).to_string()
            ));
        }
        
        tool_system_prompt
    }
    
    /// Parse DeepSeek model response that contains tool calls in a special format
    fn parse_deepseek_response(&self, content: &str) -> Result<Message, ProviderError> {
        let mut message_content = Vec::new();
        
        // Check if the response contains tool calls
        if content.contains("<｜tool▁calls▁begin｜>") {
            // Extract the tool call section
            if let Some(tool_calls_section) = content.split("<｜tool▁calls▁begin｜>").nth(1) {
                if let Some(tool_calls) = tool_calls_section.split("<｜tool▁calls▁end｜>").next() {
                    // Process each tool call
                    let tool_call_parts: Vec<&str> = tool_calls.split("<｜tool▁call▁begin｜>").collect();
                    
                    // Skip the first part (it's empty or contains text before the first tool call)
                    for part in tool_call_parts.iter().skip(1) {
                        // Extract function name and arguments
                        if let Some(function_part) = part.split("<｜tool▁sep｜>").nth(1) {
                            if let Some((function_name, rest)) = function_part.split_once('\n') {
                                // Extract JSON arguments - look for the text between ```json and ```
                                if let Some(json_block) = rest.split("```json").nth(1) {
                                    if let Some(json_str) = json_block.split("```").next() {
                                        let arguments_str = json_str.trim();
                                        
                                        // Generate a random ID for the tool call
                                        let id = format!("deepseek-{}", uuid::Uuid::new_v4().to_string());
                                        
                                        // Parse arguments as JSON
                                        match serde_json::from_str::<Value>(arguments_str) {
                                            Ok(params) => {
                                                message_content.push(MessageContent::tool_request(
                                                    id,
                                                    Ok(ToolCall::new(function_name, params)),
                                                ));
                                            }
                                            Err(e) => {
                                                let error = ToolError::InvalidParameters(format!(
                                                    "Could not interpret tool use parameters: {}. Raw JSON: {}",
                                                    e, arguments_str
                                                ));
                                                message_content.push(MessageContent::tool_request(id, Err(error)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // If there are no tool calls, treat the content as regular text
            message_content.push(MessageContent::text(content));
        }
        
        // If we couldn't parse any content, fall back to the original text
        if message_content.is_empty() {
            message_content.push(MessageContent::text(content));
        }
        
        Ok(Message {
            role: Role::Assistant,
            created: chrono::Utc::now().timestamp(),
            content: message_content,
        })
    }
}

#[async_trait]
impl Provider for HuggingFaceProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "huggingface",
            "HuggingFace",
            "Access models hosted on HuggingFace through their OpenAI-compatible API",
            HUGGINGFACE_DEFAULT_MODEL,
            HUGGINGFACE_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            HUGGINGFACE_DOC_URL,
            vec![
                ConfigKey::new("HUGGINGFACE_TOKEN", true, true, None),
                ConfigKey::new("HUGGINGFACE_PROVIDER", false, false, Some("nebius")),
                ConfigKey::new("HUGGINGFACE_TIMEOUT", false, false, Some("600")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // For DeepSeek models, embed tools in the system prompt
        let is_deepseek = self.is_deepseek_model();
        let system_prompt = if is_deepseek && !tools.is_empty() {
            println!("Embedding tools in system prompt for DeepSeek model");
            self.create_system_prompt_with_tools(system, tools)
        } else {
            system.to_string()
        };

        // Create a new messages vector that replaces tool responses with text content
        let modified_messages: Vec<Message> = messages.iter().map(|message| {
            //println!("Processing message: {:?}", message);
            if message.is_tool_response() {
                //println!("Converting tool response to text content: {:?}", message.content);
                // Create a completely new message with text content
                let text_content = format!("{:?}", message.content);
                Message {
                    role: Role::User,
                    created: chrono::Utc::now().timestamp(),
                    content: vec![MessageContent::text(text_content)],
                }
            } else if message.is_tool_call() {
                // Create a completely new message with text content
                let text_content = format!("{:?}", message.content);
                Message {
                    role: Role::User,
                    created: chrono::Utc::now().timestamp(),
                    content: vec![MessageContent::text(text_content)],
                }                
            } else {
                // Keep the original message
                message.clone()
            }
        }).collect();
        
        // print out message types for debugging
        for message in &modified_messages {


            // there should be no tool call ones, crash if to
            if message.is_tool_call() {
                panic!("Tool call message found in modified messages: {:?}", message);
            }
            if message.is_tool_response() {
                panic!("Tool call response found in modified messages: {:?}", message);
            }
        }
        

        // Create request with the appropriate system prompt and tools
        let payload = create_request(
            &self.model, 
            &system_prompt, 
            &modified_messages, 
            &[], 
            &ImageFormat::OpenAi
        )?;
        
        // Make the request
        let response = self.post(payload.clone()).await?;
        
        // Parse response
        let message = if is_deepseek {
            // For DeepSeek models, we need to use our custom parser
            if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
                self.parse_deepseek_response(content)?
            } else {
                // Fall back to standard parser if content is not a string
                response_to_message(response.clone())?
            }
        } else {
            // For other models, use the standard parser
            response_to_message(response.clone())?
        };
        
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}