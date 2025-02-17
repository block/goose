use anyhow::{Error, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{emit_debug_trace, get_model, handle_response_openai_compat};
use crate::message::{Message, MessageContent, ToolRequest};
use crate::model::ModelConfig;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use mcp_core::{content::TextContent, tool::ToolCall, role::Role};
use mcp_core::tool::Tool;
use url::Url;

// Helper function to create a message with text content
fn create_text_message(text: String) -> Message {
    let mut msg = Message::assistant();
    msg.content = vec![MessageContent::Text(TextContent {
        text,
        annotations: None,
    })];
    msg
}

// Helper function to create a message with a general tool request
fn create_tool_message(tool_name: String, args: Value) -> Message {
    let mut msg = Message::assistant();
    msg.content = vec![MessageContent::ToolRequest(ToolRequest {
        id: "1".to_string(), // Possibly refine if multiple calls are needed
        tool_call: Ok(ToolCall {
            name: tool_name,
            arguments: args,
        }),
    })];
    msg
}

/// Attempts to parse multiple tool usages of the form:
/// <tool_name>
///   <paramA>valueA</paramA>
///   <paramB>valueB</paramB>
///   ...
/// </tool_name>
/// <another_tool>
///   ...
/// </another_tool>
///
/// Returns a Vec<Message>, each containing a tool call.
fn parse_tool_usages(content: &str) -> Vec<Message> {
    let mut messages = Vec::new();
    let mut search_start = 0;

    // First normalize newlines to spaces to handle multi-line format
    let content = content.replace('\n', " ");

    while let Some(start_idx) = content[search_start..].find('<') {
        // Adjust to absolute index
        let start_idx = start_idx + search_start;
        let after_lt = &content[start_idx + 1..];
        // Find '>' to extract the tool name
        let Some(end_tool_name_idx) = after_lt.find('>') else {
            break;
        };
        let tool_name = after_lt[..end_tool_name_idx].trim();
        if tool_name.is_empty() {
            break;
        }

        println!("Found tool: {}", tool_name); // Debug trace

        let closing_tag = format!("</{}>", tool_name);
        let after_tool_start = &after_lt[end_tool_name_idx + 1..];
        let Some(closing_idx) = after_tool_start.find(&closing_tag) else {
            break;
        };

        let inner_content = &after_tool_start[..closing_idx];
        let mut args = json!({});
        let mut param_search_start = 0;

        // Parse <paramName>value</paramName>
        while let Some(param_open_idx) = inner_content[param_search_start..].find('<') {
            let param_open_idx = param_open_idx + param_search_start;
            let after_param_lt = &inner_content[param_open_idx + 1..];
            if let Some(param_close_idx) = after_param_lt.find('>') {
                let param_name = after_param_lt[..param_close_idx].trim();
                if param_name.is_empty() {
                    break;
                }
                let param_closing_tag = format!("</{}>", param_name);
                let after_param_start = &after_param_lt[param_close_idx + 1..];
                if let Some(param_closing_idx) = after_param_start.find(&param_closing_tag) {
                    let param_value = &after_param_start[..param_closing_idx].trim();
                    println!("  Param: {} = {}", param_name, param_value); // Debug trace
                    args[param_name] = json!(param_value);

                    param_search_start = param_open_idx
                        + 1
                        + param_close_idx
                        + 1
                        + param_closing_idx
                        + param_closing_tag.len();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Build the tool message
        messages.push(create_tool_message(tool_name.to_string(), args));

        // Advance to beyond the closing tag
        search_start = start_idx + 1 + end_tool_name_idx + 1 + closing_idx + closing_tag.len();
    }

    // Debug trace of parsed messages
    println!("\n=== Parsed Tool Messages ===");
    for (i, msg) in messages.iter().enumerate() {
        println!("\nMessage {}: ", i + 1);
        match &msg.content[0] {
            MessageContent::ToolRequest(tool_req) => {
                if let Ok(tool_call) = &tool_req.tool_call {
                    println!("  Tool: {}", tool_call.name);
                    println!("  Args: {}", 
                        serde_json::to_string_pretty(&tool_call.arguments)
                            .unwrap_or_else(|_| "Failed to format args".to_string())
                    );
                }
            },
            MessageContent::Text(text) => {
                println!("  Text: {}", text.text);
            },
            _ => println!("  Other content type"),
        }
    }
    println!("========================\n");

    messages
}

pub const OPENROUTER_DEFAULT_MODEL: &str = "anthropic/claude-3.5-sonnet";
pub const OPENROUTER_MODEL_PREFIX_ANTHROPIC: &str = "anthropic";

// OpenRouter can run many models, we suggest the default
pub const OPENROUTER_KNOWN_MODELS: &[&str] = &[OPENROUTER_DEFAULT_MODEL];
pub const OPENROUTER_DOC_URL: &str = "https://openrouter.ai/models";

#[derive(serde::Serialize)]
pub struct OpenRouterProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    api_key: String,
    model: ModelConfig,
}

impl Default for OpenRouterProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OpenRouterProvider::metadata().default_model);
        OpenRouterProvider::from_env(model).expect("Failed to initialize OpenRouter provider")
    }
}

impl OpenRouterProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENROUTER_API_KEY")?;
        let host: String = config
            .get("OPENROUTER_HOST")
            .unwrap_or_else(|_| "https://openrouter.ai".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            api_key,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join("api/v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/block/goose")
            .header("X-Title", "Goose")
            .json(&payload)
            .send()
            .await?;

        handle_response_openai_compat(response).await
    }
}

/// Update the request when using anthropic model.
/// For anthropic model, we can enable prompt caching to save cost. Since openrouter is the OpenAI compatible
/// endpoint, we need to modify the open ai request to have anthropic cache control field.
fn update_request_for_anthropic(original_payload: &Value) -> Value {
    let mut payload = original_payload.clone();

    if let Some(messages_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|messages| messages.as_array_mut())
    {
        // Add "cache_control" to the last and second-to-last "user" messages.
        // During each turn, we mark the final message with cache_control so the conversation can be
        // incrementally cached. The second-to-last user message is also marked for caching with the
        // cache_control parameter, so that this checkpoint can read from the previous cache.
        let mut user_count = 0;
        for message in messages_spec.iter_mut().rev() {
            if message.get("role") == Some(&json!("user")) {
                if let Some(content) = message.get_mut("content") {
                    if let Some(content_str) = content.as_str() {
                        *content = json!([{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]);
                    }
                }
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        // Update the system message to have cache_control field.
        if let Some(system_message) = messages_spec
            .iter_mut()
            .find(|msg| msg.get("role") == Some(&json!("system")))
        {
            if let Some(content) = system_message.get_mut("content") {
                if let Some(content_str) = content.as_str() {
                    *system_message = json!({
                        "role": "system",
                        "content": [{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]
                    });
                }
            }
        }
    }

    if let Some(tools_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("tools"))
        .and_then(|tools| tools.as_array_mut())
    {
        // Add "cache_control" to the last tool spec, if any. This means that all tool definitions,
        // will be cached as a single prefix.
        if let Some(last_tool) = tools_spec.last_mut() {
            if let Some(function) = last_tool.get_mut("function") {
                function
                    .as_object_mut()
                    .unwrap()
                    .insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
            }
        }
    }
    payload
}

fn create_request_based_on_model(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> anyhow::Result<Value, Error> {
    // For deepseek models, we want to include tools in the system prompt instead
    if model_config.model_name.contains("deepseek-r1") {
        let tool_instructions = if !tools.is_empty() {
            let tool_descriptions: Vec<String> = tools.iter()
                .map(|tool| format!("- {}: {}", tool.name, tool.description))
                .collect();

            // println!("\nTools being provided:\n{}", tool_descriptions.join("\n"));

            format!(
                "\n\nAvailable tools:\n{}\n\n# Reminder: Instructions for Tool Use\n\nTool uses are formatted using XML-style tags. The tool name is enclosed in opening and closing tags. Here's the structure:\n\n<tool_name>\n<parameter1_name>value1</parameter1_name>\n<parameter2_name>value2</parameter2_name>\n...\n</tool_name>\n\nFor example, to use the shell tool:\n\n<developer__shell>\n<command>ls -l</command>\n</developer__shell>\n\nAlways adhere to this format for all tool uses to ensure proper parsing and execution.\n",
                tool_descriptions.join("\n")
            )
        } else {
            String::new()
        };

        let enhanced_system = format!("{}{}", system, tool_instructions);
        println!("\nEnhanced system prompt:\n{}", enhanced_system);

        // Find the last user message and enhance it
        let mut modified_messages = messages.to_vec();
        if let Some(last_user_msg_idx) = modified_messages.iter().rposition(|msg| {
            // Only consider user messages that don't have a tool_call_id
            msg.role == Role::User && !msg.content.iter().any(|content| {
                matches!(content, MessageContent::ToolResponse(_))
            })
        }) {
            let last_user_msg = &modified_messages[last_user_msg_idx];
            // Get the text content from the last user message
            let user_text = last_user_msg.content.iter().find_map(|content| {
                if let MessageContent::Text(text) = content {
                    Some(text.text.clone())
                } else {
                    None
                }
            }).unwrap_or_default();

            // Create new message with enhanced system prompt prepended
            let enhanced_msg = Message::user().with_text(format!("{}\n{}", enhanced_system, user_text));
            modified_messages[last_user_msg_idx] = enhanced_msg;
        }

        let payload = create_request(
            model_config,
            "", // Empty system prompt since we included it in the user message
            &modified_messages,
            &[], // Pass empty tools array since we're handling them in the system prompt
            &super::utils::ImageFormat::OpenAi,
        )?;
        return Ok(payload);
    }

    let mut payload = create_request(
        model_config,
        system,
        messages,
        tools,
        &super::utils::ImageFormat::OpenAi,
    )?;

    if model_config
        .model_name
        .starts_with(OPENROUTER_MODEL_PREFIX_ANTHROPIC)
    {
        payload = update_request_for_anthropic(&payload);
    }

    Ok(payload)
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "openrouter",
            "OpenRouter",
            "Router for many model providers",
            OPENROUTER_DEFAULT_MODEL,
            OPENROUTER_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            OPENROUTER_DOC_URL,
            vec![
                ConfigKey::new("OPENROUTER_API_KEY", true, true, None),
                ConfigKey::new(
                    "OPENROUTER_HOST",
                    false,
                    false,
                    Some("https://openrouter.ai"),
                ),
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
        // Create the base payload
        let mut payload = create_request_based_on_model(&self.model, system, messages, tools)?;
        // payload["provider"] = json!({"order": ["Avian"], "allow_fallbacks": false});
        println!("Request Payload: {}\n", serde_json::to_string_pretty(&payload).unwrap());
        // Make request
        let response = self.post(payload.clone()).await?;
        

        // Parse response - special handling for deepseek models
        let message = if self.model.model_name.contains("deepseek-r1") {
            let content = response["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            println!("Response payload:\n{}", serde_json::to_string_pretty(&response).unwrap());
            println!("\nExtracted content:\n{}", content);

            // Attempt to parse multiple tool usability from the content
            let calls = parse_tool_usages(&content);

            if calls.is_empty() {
                // No tool calls found, treat entire content as text
                create_text_message(content)
            } else {
                // For demonstration, return the FIRST tool call.
                // If you want to handle multiple calls, see the parse_tool_usages doc.
                calls[0].clone()
            }
        } else {
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
        emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
