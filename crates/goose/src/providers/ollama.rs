use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::utils::{get_model, handle_response_openai_compat};
use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use mcp_core::tool::ToolCall;
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use serde_json::json;
use anyhow::Result;
use async_trait::async_trait;
use indoc::formatdoc;
use mcp_core::tool::Tool;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use url::Url;
use uuid::Uuid;

pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5";
// Ollama can run many models, we only provide the default
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[OLLAMA_DEFAULT_MODEL];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    model: ModelConfig,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let model = ModelConfig::new(OllamaProvider::metadata().default_model);
        OllamaProvider::from_env(model).expect("Failed to initialize Ollama provider")
    }
}

impl OllamaProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(Self {
            client,
            host,
            model,
        })
    }

    /// Get the base URL for Ollama API calls
    fn get_base_url(&self) -> Result<Url, ProviderError> {
        // OLLAMA_HOST is sometimes just the 'host' or 'host:port' without a scheme
        let base = if self.host.starts_with("http://") || self.host.starts_with("https://") {
            self.host.clone()
        } else {
            format!("http://{}", self.host)
        };

        let mut base_url = Url::parse(&base)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;

        // Set the default port if missing
        let explicit_default_port = self.host.ends_with(":80") || self.host.ends_with(":443");
        if base_url.port().is_none() && !explicit_default_port {
            base_url.set_port(Some(OLLAMA_DEFAULT_PORT)).map_err(|_| {
                ProviderError::RequestFailed("Failed to set default port".to_string())
            })?;
        }
        
        Ok(base_url)
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = self.get_base_url()?;
        
        let url = base_url.join("v1/chat/completions").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let response = self.client.post(url).json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
    
    /// Post to Ollama API with structured output format
    async fn post_structured(&self, messages: &[Message], format_schema: Value, system_prompt: Option<&str>) -> Result<Value, ProviderError> {
        let base_url = self.get_base_url()?;
        
        let url = base_url.join("api/chat").map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct structured API endpoint URL: {e}"))
        })?;
        
        // Create a Vec to store all ollama messages
        let mut ollama_messages: Vec<Value> = Vec::new();
        
        // Add system prompt if provided
        if let Some(system) = system_prompt {
            ollama_messages.push(json!({
                "role": "system",
                "content": system
            }));
        }
        
        // Convert user messages to Ollama format and add them
        ollama_messages.extend(messages.iter()
            .map(|msg| {
                let role = match msg.role {
                    mcp_core::role::Role::User => "user",
                    mcp_core::role::Role::Assistant => "assistant",
                    // System role doesn't exist in mcp_core, but we'll handle it as user
                };
                
                // Extract text content from the message
                let content = msg.content.iter()
                    .filter_map(|c| {
                        if let MessageContent::Text(text) = c {
                            Some(text.text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                json!({
                    "role": role,
                    "content": content
                })
            }));
        
        // Build the structured output request using a capable model for tool call interpretation
        let payload = json!({
            "model": "phi4", // Use qwen2.5 for consistent tool call interpretation
            "messages": ollama_messages,
            "stream": false,
            "format": format_schema
        });
        
        tracing::warn!("Sending structured output request to Ollama: {}", 
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "Could not serialize payload".to_string()));
        
        let response = self.client.post(url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(ProviderError::RequestFailed(format!(
                "Ollama structured API returned error status {}: {}", 
                status, error_text
            )));
        }
        
        // Parse the response
        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse Ollama structured API response: {e}"))
        })?;
        
        tracing::warn!("Received structured response: {}", 
            serde_json::to_string_pretty(&response_json).unwrap_or_else(|_| "Could not serialize response".to_string()));
        
        Ok(response_json)
    }

    async fn interpret_tool_call(
        &self,
        response: &Value,
        tools: &[Tool],
    ) -> Result<Message, ProviderError> {
    // tracing::warn!("Interpreting potential tool calls from response: {}", serde_json::to_string_pretty(response).unwrap_or_else(|_| "Could not serialize response".to_string()));
    
    // First, get the original message from the response
    let original_message = response_to_message(response.clone())?;
    
    // If there are no tools or the response is empty, return the original message
    if tools.is_empty() || response.is_null() {
        tracing::warn!("No tools available or empty response, skipping interpretation");
        return Ok(original_message);
    }
    
    // Extract content from the original message
    let content_opt = original_message.content.iter().find_map(|content| {
        if let MessageContent::Text(text) = content {
            Some(text.text.as_str())
        } else {
            None
        }
    });
    
    // If there's no text content or it's already a tool request, return the original message
    let content = match content_opt {
        Some(text) => {
            // tracing::warn!("Extracted content for tool call interpretation: {}", text);
            text
        },
        None => {
            tracing::warn!("No text content found in the message, skipping interpretation");
            return Ok(original_message);
        },
    };
    
    // Check if there's already a tool request
    if original_message.content.iter().any(|content| {
        matches!(content, MessageContent::ToolRequest(_))
    }) {
        tracing::warn!("Message already contains a tool request, skipping interpretation");
        return Ok(original_message);
    }
    
    // Create descriptions of tools (for logging purposes)
    let _tools_descriptions = tools
        .iter()
        .map(|tool| {
            format!(
                "Name: {}\nDescription: {}\nSchema: {}",
                tool.name,
                tool.description,
                serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
        
    // We could include tools_descriptions in the system prompt in the future
    // to give more context about available tools to the interpreter
    
    let system_prompt = format!(
        "Rewrite detectable attempts at JSON-formatted tool requests into proper JSON tool calls.

If there is a SINGLE tool call, use this format:
{{
  \"name\": \"tool_name\",
  \"arguments\": {{
    \"param1\": \"value1\",
    \"param2\": \"value2\"
  }}
}}

If there are MULTIPLE tool calls, use this format:
[
  {{
    \"name\": \"first_tool_name\",
    \"arguments\": {{
      \"param1\": \"value1\"
    }}
  }},
  {{
    \"name\": \"second_tool_name\",
    \"arguments\": {{
      \"param1\": \"value1\",
      \"param2\": \"value2\"
    }}
  }}
]

If NO tools are asked for:
{{}}
",
    );
    
    // Create messages for interpretation with explicit instruction to output tool calls as JSON
    let enhanced_content = format!("{}\n\nWrite valid json if there is detectable json or an attempt at json", content);
    tracing::info!("Enhanced content for tool call interpretation: {}", enhanced_content);
    
    let messages = vec![
        Message::user().with_text(enhanced_content),
    ];
    
    // Define the JSON schema for tool call format that supports both single and multiple tool calls
    let tool_call_schema = json!({
        "oneOf": [
            // Schema for a single tool call
            {
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "message for user"
                    },
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
            },
            // Schema for multiple tool calls in an array
            {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "message for user"
                        },
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
        ]
    });
    
    // tracing::info!("Using Ollama structured output with schema: {}", 
    //     serde_json::to_string_pretty(&tool_call_schema).unwrap_or_else(|_| "Could not serialize schema".to_string()));
    
    // Make a call to ollama with structured output
    tracing::warn!("Sending structured interpretation request to Ollama");
    
    // Process the response directly from the structured output API
    let interpreter_result = self.post_structured(&messages, tool_call_schema, Some(&system_prompt)).await;
    
    let interpreter_response = match interpreter_result {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to get structured response from Ollama: {}", e);
            // Fall back to the standard approach if structured output fails
            let interpreter_model = ModelConfig::new("phi4".to_string());
            let payload = create_request(
                &interpreter_model,
                &system_prompt, 
                &messages,
                tools,
                &super::utils::ImageFormat::OpenAi,
            )?;
            
            tracing::warn!("Falling back to standard API with tools: {}", tools.len());
            self.post(payload).await?
        }
    };
    
    tracing::warn!("Received interpreter response: {}", 
        serde_json::to_string_pretty(&interpreter_response).unwrap_or_else(|_| "Could not serialize response".to_string()));
    
    // Create the message we'll build upon
    let mut final_message = original_message.clone();
    let mut tool_calls_processed = false;
    
    // Handle case 1: Direct single tool call format {name: "foo", arguments: {...}}
    if interpreter_response.get("name").is_some() && interpreter_response.get("arguments").is_some() {
        let tool_name = interpreter_response["name"].as_str().unwrap_or_default();
        let tool_arguments = interpreter_response["arguments"].clone();
        
        if !tool_name.is_empty() {
            tracing::info!("Processing direct tool call: name={}, arguments={}", 
                tool_name, 
                serde_json::to_string_pretty(&tool_arguments).unwrap_or_default());
            
            let id = Uuid::new_v4().to_string();
            let tool_call = ToolCall::new(tool_name, tool_arguments);
            final_message = final_message.with_tool_request(id, Ok(tool_call));
            tool_calls_processed = true;
        }
    }
    
    // Handle case 2: Structured message format with content containing JSON tool call(s)
    else if interpreter_response.get("message").is_some() {
        let message_obj = &interpreter_response["message"];
        
        // Check if content exists and can be parsed as JSON
        if let Some(content) = message_obj.get("content").and_then(|c| c.as_str()) {
            if let Ok(content_json) = serde_json::from_str::<Value>(content) {
                // Handle case 2a: Single tool call in content
                if content_json.is_object() && content_json.get("name").is_some() && content_json.get("arguments").is_some() {
                    let tool_name = content_json["name"].as_str().unwrap_or_default();
                    let tool_arguments = content_json["arguments"].clone();
                    
                    if !tool_name.is_empty() {
                        tracing::info!("Processing content-embedded tool call: name={}, arguments={}", 
                            tool_name, 
                            serde_json::to_string_pretty(&tool_arguments).unwrap_or_default());
                            
                        let id = Uuid::new_v4().to_string();
                        let tool_call = ToolCall::new(tool_name, tool_arguments);
                        final_message = final_message.with_tool_request(id, Ok(tool_call));
                        tool_calls_processed = true;
                    }
                }
                // Handle case 2b: Array of tool calls in content
                else if content_json.is_array() {
                    for tool_item in content_json.as_array().unwrap() {
                        if tool_item.is_object() && tool_item.get("name").is_some() && tool_item.get("arguments").is_some() {
                            let tool_name = tool_item["name"].as_str().unwrap_or_default();
                            let tool_arguments = tool_item["arguments"].clone();
                            
                            if !tool_name.is_empty() {
                                tracing::info!("Processing array tool call: name={}, arguments={}", 
                                    tool_name, 
                                    serde_json::to_string_pretty(&tool_arguments).unwrap_or_default());
                                    
                                let id = Uuid::new_v4().to_string();
                                let tool_call = ToolCall::new(tool_name, tool_arguments);
                                final_message = final_message.with_tool_request(id, Ok(tool_call));
                                tool_calls_processed = true;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // If we processed any structured tool calls, return the final message
    if tool_calls_processed {
        tracing::info!("Successfully processed structured tool calls");
        return Ok(final_message);
    }
    
    // Otherwise, fall back to standard response handling
    let interpreter_message = response_to_message(interpreter_response.clone())?;
    
    // For standard responses, we need to process them as before
    tracing::info!("Processing standard response format for tool calls");
    
    // Check if the interpreter message has any tool calls
    let has_tool_calls = interpreter_message.content.iter().any(|content| {
        matches!(content, MessageContent::ToolRequest(_))
    });
    
    if !has_tool_calls {
        tracing::info!("No tool calls detected in interpreter response");
        return Ok(original_message);
    }
    
    // Get all tool requests from the interpreter message
    let tool_requests: Vec<_> = interpreter_message.content.iter()
        .filter_map(|content| {
            if let MessageContent::ToolRequest(tool_request) = content {
                Some(tool_request)
            } else {
                None
            }
        })
        .collect();
    
    tracing::info!("Found {} tool calls in interpreter response", tool_requests.len());
    
    if tool_requests.is_empty() {
        return Ok(original_message);
    }
    
    // Create a message with both the original content and the tool requests
    let mut final_message = original_message;
    
    // Add each tool request to the message
    for tool_request in tool_requests {
        if let Ok(tool_call) = &tool_request.tool_call {
            tracing::info!("Adding tool call with name: {} and arguments: {}", 
                tool_call.name, 
                serde_json::to_string_pretty(&tool_call.arguments).unwrap_or_else(|_| "Could not serialize arguments".to_string()));
            
            // Create a copy of the tool call and add it to the final message
            let new_id = Uuid::new_v4().to_string();
            tracing::info!("Adding tool call with ID: {}", new_id);
            
            let new_tool_call = ToolCall::new(&tool_call.name, tool_call.arguments.clone());
            final_message = final_message.with_tool_request(new_id, Ok(new_tool_call));
        }
    }
    
    Ok(final_message)
    
    }

}

#[async_trait]
impl Provider for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "ollama",
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.iter().map(|&s| s.to_string()).collect(),
            OLLAMA_DOC_URL,
            vec![ConfigKey::new(
                "OLLAMA_HOST",
                true,
                false,
                Some(OLLAMA_HOST),
            )],
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
        // Transform the system message to replace developer instructions
        let modified_system = if let Some(dev_section) = system.split("## developer").nth(1) {
            if let (Some(start_idx), Some(end_idx)) = (
                dev_section.find("### Instructions"),
                dev_section.find("operating system:"),
            ) {
                let new_instructions = formatdoc! {r#"
        The Developer extension enables you to edit code files, execute shell commands, and capture screen/window content. These tools allow for various development and debugging workflows.
        Available Tools:
        1. Shell Execution (`developer__shell`)
        Executes commands in the shell and returns the combined output and error messages.
        Use cases:
        - Running scripts: `python script.py`
        - Installing dependencies: `pip install -r requirements.txt`
        - Checking system information: `uname -a`, `df -h`
        - Searching for files or text: **Use `rg` (ripgrep) instead of `find` or `ls -r`**
          - Find a file: `rg --files | rg example.py`
          - Search within files: `rg 'class Example'`
        Best Practices:
        - **Avoid commands with large output** (pipe them to a file if necessary).
        - **Run background processes** if they take a long time (e.g., `uvicorn main:app &`).
        - **git commands can be run on the shell, however if the git extension is installed, you should use the git tool instead.
        - **If the shell command is a rm, mv, or cp, you should verify with the user before running the command.
        2. Text Editor (`developer__text_editor`)
        Performs file-based operations such as viewing, writing, replacing text, and undoing edits.
        Commands:
        - view: Read the content of a file.
        - write: Create or overwrite a file. Caution: Overwrites the entire file!
        - str_replace: Replace a specific string in a file.
        - undo_edit: Revert the last edit.
        Example Usage:
        developer__text_editor(command="view", file_path="/absolute/path/to/file.py")
        developer__text_editor(command="write", file_path="/absolute/path/to/file.py", file_text="print('hello world')")
        developer__text_editor(command="str_replace", file_path="/absolute/path/to/file.py", old_str="hello world", new_str="goodbye world")
        developer__text_editor(command="undo_edit", file_path="/absolute/path/to/file.py")
        Protocol for Text Editor:
        For edit and replace commands, please verify what you are editing with the user before running the command.
        - User: "Please edit the file /absolute/path/to/file.py"
        - Assistant: "Ok sounds good, I'll be editing the file /absolute/path/to/file.py and creating modifications xyz to the file. Let me know whether you'd like to proceed."
        - User: "Yes, please proceed."
        - Assistant: "I've created the modifications xyz to the file /absolute/path/to/file.py"
        3. List Windows (`developer__list_windows`)
        Lists all visible windows with their titles.
        Use this to find window titles for screen capture.
        4. Screen Capture (`developer__screen_capture`)
        Takes a screenshot of a display or specific window.
        Options:
        - Capture display: `developer__screen_capture(display=0)`  # Main display
        - Capture window: `developer__screen_capture(window_title="Window Title")`
        To use tools, ask the user to execute the tools for you by requesting the tool use in the exact JSON format below. 

## Tool Call JSON Format
```json
{{
  "name": "tool_name",
  "arguments": {{
    "parameter1": "value1",
    "parameter2": "value2"
            }}
            }}
```
        Info: at the start of the session, the user's directory is:
        "#};

                let before_dev = system.split("## developer").next().unwrap_or("");
                let after_marker = &dev_section[end_idx..];

                format!(
                    "{}## developer{}### Instructions\n{}{}",
                    before_dev,
                    &dev_section[..start_idx],
                    new_instructions,
                    after_marker
                )
            } else {
                system.to_string()
            }
        } else {
            system.to_string()
        };

        // Create initial messages with modified_system as the content of a user message
        // and add an assistant reply acknowledging it
        let mut initial_messages = vec![
            Message::user().with_text(&modified_system),
            Message::assistant().with_text("I understand. I'm ready to help with any tasks or questions you have.")
        ];
        
        // Append the actual user messages
        initial_messages.extend_from_slice(messages);
        
        // Create request with empty system prompt and the initial messages including the system instructions
        let payload = create_request(
            &self.model,
            "", // No system prompt, using modified_system as user message content instead
            &initial_messages,
            &vec![],
            &super::utils::ImageFormat::OpenAi,
        )?;
        let response = self.post(payload.clone()).await?;
        
        // Call interpret_tool_call to detect and process tool calls in the response
        let message = self.interpret_tool_call(&response, tools).await?;
        
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);
        super::utils::emit_debug_trace(self, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }
}
