use crate::message::{Message, MessageContent};
use crate::model::ModelConfig;
use anyhow::Result;
use indoc::formatdoc;
use mcp_core::tool::{Tool, ToolCall};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;
use super::errors::ProviderError;
use crate::providers::formats::openai::create_request;

/// A trait for models that can convert text to structured tool calls
#[async_trait::async_trait]
pub trait ToolInterpreter {
    /// Interpret potential tool calls from text and convert them to proper tool call format
    async fn interpret_to_tool_calls(&self, content: &str, tools: &[Tool]) -> Result<Vec<ToolCall>, ProviderError>;
}

/// Configuration for the tool interpretation shim
/// 
/// Environment variables that affect behavior:
/// - GOOSE_TOOL_SHIM: When set to "true" or "1", enables using the tool shim in the standard OllamaProvider (default: false)
/// - GOOSE_TOOLSHIM_OLLAMA_MODEL: Ollama model to use as the tool interpreter
#[derive(Clone, Debug)]
pub struct ToolShimConfig {
    /// Model configuration for the interpreter model
    pub model: ModelConfig,
    /// Custom system prompt to use for interpretation (if None, a default will be used)
    pub system_prompt: Option<String>,
    /// Schema to use for structured output (if None, a default will be used)
    pub format_schema: Option<Value>,
}

impl Default for ToolShimConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig::new("phi4".to_string()),
            system_prompt: None,
            format_schema: None,
        }
    }
}

pub fn process_interpreter_response(
    response: &Value,
) -> Result<Vec<ToolCall>, ProviderError> {
    // Create an empty message to use as fallback
    let original_message = Message::assistant();
    
    // First, try to extract text content using response_to_message
    let extracted_message = match super::formats::openai::response_to_message(response.clone()) {
        Ok(message) => message,
        Err(_) => original_message.clone(),
    };
    
    // Extract text content from the message
    let text_content = extracted_message.content.iter()
        .filter_map(|content| {
            if let MessageContent::Text(text) = content {
                Some(text.text.clone())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
    
    // If we have text content, try to parse it as JSON
    if !text_content.is_empty() {
        if let Ok(content_json) = serde_json::from_str::<Value>(&text_content) {
            // Check for the new format with tool_calls array inside an object
            let tool_calls_array = if content_json.is_object() && content_json.get("tool_calls").is_some() {
                // Extract the tool_calls array
                content_json["tool_calls"].as_array()
            } else if content_json.is_array() {
                // Direct array format (for backward compatibility)
                content_json.as_array()
            } else if content_json.is_object() && content_json.get("name").is_some() && content_json.get("arguments").is_some() {
                // Single tool call as an object (for backward compatibility)
                None
            } else {
                None
            };
            
            // Process the tool calls array if found
            if let Some(tool_calls) = tool_calls_array {
                if !tool_calls.is_empty() {
                    // Wrap in OpenAI format and process
                    let wrapped_response = json!({
                        "choices": [{
                            "message": {
                                "tool_calls": tool_calls.iter().map(|item| {
                                    if item.is_object() && item.get("name").is_some() && item.get("arguments").is_some() {
                                        json!({
                                            "id": Uuid::new_v4().to_string(),
                                            "function": {
                                                "name": item["name"],
                                                "arguments": item["arguments"].to_string()
                                            }
                                        })
                                    } else {
                                        json!(null)
                                    }
                                }).filter(|item| !item.is_null()).collect::<Vec<Value>>()
                            }
                        }]
                    });
                    
                    return match super::formats::openai::response_to_message(wrapped_response) {
                        Ok(message) => {
                            // Extract tool calls from the message
                            let tool_calls = message.content.iter()
                                .filter_map(|content| {
                                    if let MessageContent::ToolRequest(tool_request) = content {
                                        if let Ok(tool_call) = &tool_request.tool_call {
                                            Some(tool_call.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            Ok(tool_calls)
                        },
                        Err(_) => Ok(vec![])
                    };
                }
            }
            
            // Handle single tool call as an object (for backward compatibility)
            if content_json.is_object() && content_json.get("name").is_some() && content_json.get("arguments").is_some() {
                // Single tool call in content
                let wrapped_response = json!({
                    "choices": [{
                        "message": {
                            "tool_calls": [{
                                "id": Uuid::new_v4().to_string(),
                                "function": {
                                    "name": content_json["name"],
                                    "arguments": content_json["arguments"].to_string()
                                }
                            }]
                        }
                    }]
                });
                
                return match super::formats::openai::response_to_message(wrapped_response) {
                    Ok(message) => {
                        // Extract tool calls from the message
                        let tool_calls = message.content.iter()
                            .filter_map(|content| {
                                if let MessageContent::ToolRequest(tool_request) = content {
                                    if let Ok(tool_call) = &tool_request.tool_call {
                                        Some(tool_call.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect();
                        Ok(tool_calls)
                    },
                    Err(_) => Ok(vec![])
                };
            }
        }
    }
    
    // If we couldn't extract and parse JSON from the text content, fall back to the original approach
    
    // Check for the new format with tool_calls array inside an object
    if response.is_object() && response.get("tool_calls").is_some() && response["tool_calls"].is_array() {
        let tool_calls = response["tool_calls"].as_array().unwrap();
        
        // Wrap in OpenAI format and process
        let wrapped_response = json!({
            "choices": [{
                "message": {
                    "tool_calls": tool_calls.iter().map(|item| {
                        if item.is_object() && item.get("name").is_some() && item.get("arguments").is_some() {
                            json!({
                                "id": Uuid::new_v4().to_string(),
                                "function": {
                                    "name": item["name"],
                                    "arguments": item["arguments"].to_string()
                                }
                            })
                        } else {
                            json!(null)
                        }
                    }).filter(|item| !item.is_null()).collect::<Vec<Value>>()
                }
            }]
        });
        
        return match super::formats::openai::response_to_message(wrapped_response) {
            Ok(message) => {
                // Extract tool calls from the message
                let tool_calls = message.content.iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(tool_request) = content {
                            if let Ok(tool_call) = &tool_request.tool_call {
                                Some(tool_call.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(tool_calls)
            },
            Err(_) => Ok(vec![])
        };
    }
    // Primary direct array format handling (for backward compatibility)
    else if response.is_array() {
        // Wrap the array in an OpenAI-style response structure that 
        // response_to_message can handle
        let wrapped_response = json!({
            "choices": [{
                "message": {
                    "tool_calls": response.as_array().unwrap().iter().map(|item| {
                        // Convert each {name: "x", arguments: {...}} to OpenAI format
                        if item.is_object() && item.get("name").is_some() && item.get("arguments").is_some() {
                            json!({
                                "id": Uuid::new_v4().to_string(),
                                "function": {
                                    "name": item["name"],
                                    "arguments": item["arguments"].to_string()
                                }
                            })
                        } else {
                            // Skip invalid items
                            json!(null)
                        }
                    }).filter(|item| !item.is_null()).collect::<Vec<Value>>()
                }
            }]
        });

        // Use the OpenAI message parser to handle the array of tool calls
        return match super::formats::openai::response_to_message(wrapped_response) {
            Ok(message) => {
                // Extract tool calls from the message
                let tool_calls = message.content.iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(tool_request) = content {
                            if let Ok(tool_call) = &tool_request.tool_call {
                                Some(tool_call.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(tool_calls)
            },
            Err(_) => Ok(vec![])
        };
    }
    // Handle single direct tool call format (for backward compatibility)
    else if response.is_object() && response.get("name").is_some() && response.get("arguments").is_some() {
        // Convert a single {name: "x", arguments: {...}} to OpenAI format
        let wrapped_response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": Uuid::new_v4().to_string(),
                        "function": {
                            "name": response["name"],
                            "arguments": response["arguments"].to_string()
                        }
                    }]
                }
            }]
        });

        // Use the OpenAI message parser
        return match super::formats::openai::response_to_message(wrapped_response) {
            Ok(message) => {
                // Extract tool calls from the message
                let tool_calls = message.content.iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(tool_request) = content {
                            if let Ok(tool_call) = &tool_request.tool_call {
                                Some(tool_call.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(tool_calls)
            },
            Err(_) => Ok(vec![])
        };
    }
    // OpenAI format might already be in the correct format
    else if response.get("choices").is_some() {
        return match super::formats::openai::response_to_message(response.clone()) {
            Ok(message) => {
                // Extract tool calls from the message
                let tool_calls = message.content.iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(tool_request) = content {
                            if let Ok(tool_call) = &tool_request.tool_call {
                                Some(tool_call.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(tool_calls)
            },
            Err(_) => Ok(vec![])
        };
    }
    // Handle content embedded in a message structure
    else if response.get("message").is_some() && response["message"].get("content").is_some() {
        let content = response["message"]["content"].as_str().unwrap_or_default();
        
        // Try to parse the content as JSON
        if let Ok(content_json) = serde_json::from_str::<Value>(content) {
            // Check for the new format with tool_calls array inside an object
            let tool_calls_array = if content_json.is_object() && content_json.get("tool_calls").is_some() {
                // Extract the tool_calls array
                content_json["tool_calls"].as_array()
            } else if content_json.is_array() {
                // Direct array format (for backward compatibility)
                content_json.as_array()
            } else if content_json.is_object() && content_json.get("name").is_some() && content_json.get("arguments").is_some() {
                // Single tool call as an object (for backward compatibility)
                None
            } else {
                None
            };
            
            // Process the tool calls array if found
            if let Some(tool_calls) = tool_calls_array {
                if !tool_calls.is_empty() {
                    // Wrap in OpenAI format and process
                    let wrapped_response = json!({
                        "choices": [{
                            "message": {
                                "tool_calls": tool_calls.iter().map(|item| {
                                    if item.is_object() && item.get("name").is_some() && item.get("arguments").is_some() {
                                        json!({
                                            "id": Uuid::new_v4().to_string(),
                                            "function": {
                                                "name": item["name"],
                                                "arguments": item["arguments"].to_string()
                                            }
                                        })
                                    } else {
                                        json!(null)
                                    }
                                }).filter(|item| !item.is_null()).collect::<Vec<Value>>()
                            }
                        }]
                    });
                    
                    return match super::formats::openai::response_to_message(wrapped_response) {
                        Ok(message) => {
                            // Extract tool calls from the message
                            let tool_calls = message.content.iter()
                                .filter_map(|content| {
                                    if let MessageContent::ToolRequest(tool_request) = content {
                                        if let Ok(tool_call) = &tool_request.tool_call {
                                            Some(tool_call.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            Ok(tool_calls)
                        },
                        Err(_) => Ok(vec![])
                    };
                }
            }
            
            // Handle single tool call as an object (for backward compatibility)
            if content_json.is_object() && content_json.get("name").is_some() && content_json.get("arguments").is_some() {
                // Single tool call in content
                let wrapped_response = json!({
                    "choices": [{
                        "message": {
                            "tool_calls": [{
                                "id": Uuid::new_v4().to_string(),
                                "function": {
                                    "name": content_json["name"],
                                    "arguments": content_json["arguments"].to_string()
                                }
                            }]
                        }
                    }]
                });
                
                return match super::formats::openai::response_to_message(wrapped_response) {
                    Ok(message) => {
                        // Extract tool calls from the message
                        let tool_calls = message.content.iter()
                            .filter_map(|content| {
                                if let MessageContent::ToolRequest(tool_request) = content {
                                    if let Ok(tool_call) = &tool_request.tool_call {
                                        Some(tool_call.clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect();
                        Ok(tool_calls)
                    },
                    Err(_) => Ok(vec![])
                };
            }
        }
    }
    
    // Default: return empty vec if no valid tool calls were detected
    let processed_message = original_message;
    
    // Extract tool calls from the processed message
    let tool_calls = processed_message.content.iter()
        .filter_map(|content| {
            if let MessageContent::ToolRequest(tool_request) = content {
                if let Ok(tool_call) = &tool_request.tool_call {
                    Some(tool_call.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    
    Ok(tool_calls)
}

pub fn ollama_tool_structured_ouput_format_schema() -> Value {
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

/// Ollama-specific implementation of the ToolInterpreter trait
pub struct OllamaInterpreter {
    client: Client,
    base_url: String,
}

impl OllamaInterpreter {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .expect("Failed to create HTTP client");
            
        Self {
            client,
            base_url,
        }
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
        
        let model_config = ModelConfig::new(model.to_string());
        
        let mut payload = create_request(
            &model_config,
            &system_prompt, 
            &messages,
            &vec![], // No tools
            &super::utils::ImageFormat::OpenAi,
        )?;
        
        payload["stream"] = json!(false); // needed for the /api/chat endpoint to work
        payload["format"] = format_schema;
        
        tracing::warn!("payload: {}", serde_json::to_string_pretty(&payload).unwrap_or_default());
        
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Could not read error response".to_string()
            };
            
            return Err(ProviderError::RequestFailed(format!(
                "Ollama structured API returned error status {}: {}", 
                status, error_text
            )));
        }
        
        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse Ollama structured API response: {e}"))
        })?;
        
        Ok(response_json)
    }
}

#[async_trait::async_trait]
impl ToolInterpreter for OllamaInterpreter {
    async fn interpret_to_tool_calls(&self, last_assistant_msg: &str, tools: &[Tool]) -> Result<Vec<ToolCall>, ProviderError> {
        if tools.is_empty() {
            return Ok(vec![]);
        }
        
        // Create the system prompt
        let system_prompt = "Rewrite detectable attempts at JSON-formatted tool requests into proper JSON tool calls.

Always use an object with a tool_calls array format:
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

If NO tools are asked for, return an object with an empty tool_calls array:
{{
  \"tool_calls\": []
}}
";
        
        // Create enhanced content with instruction to output tool calls as JSON
        let format_instruction = format!("{}\n\nWrite valid json if there is detectable json or an attempt at json", last_assistant_msg);
        
        // Define the JSON schema for tool call format
        let format_schema = ollama_tool_structured_ouput_format_schema();
        
        // Determine which model to use for interpretation (from env var or default)
        let interpreter_model = std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL").unwrap_or_else(|_| "phi4".to_string());
        
        // Make a call to ollama with structured output
        let interpreter_response = self.post_structured(
            &system_prompt,
            &format_instruction,
            format_schema,
            &interpreter_model
        ).await?;
        
        // Process the interpreter response to get tool calls directly
        let tool_calls = process_interpreter_response(&interpreter_response)?;
        
        Ok(tool_calls)
    }
}

/// Helper function to augment a message with tool calls if any are detected
pub async fn augment_message_with_tool_calls<T: ToolInterpreter>(
    interpreter: &T,
    message: Message,
    tools: &[Tool],
) -> Result<Message, ProviderError> {
    // If there are no tools or the message is empty, return the original message
    if tools.is_empty() {
        return Ok(message);
    }
    
    // Extract content from the message
    let content_opt = message.content.iter().find_map(|content| {
        if let MessageContent::Text(text) = content {
            Some(text.text.as_str())
        } else {
            None
        }
    });
    
    // If there's no text content or it's already a tool request, return the original message
    let content = match content_opt {
        Some(text) => text,
        None => return Ok(message),
    };
    
    // Check if there's already a tool request
    if message.content.iter().any(|content| {
        matches!(content, MessageContent::ToolRequest(_))
    }) {
        return Ok(message);
    }
    
    // Use the interpreter to convert the content to tool calls
    let tool_calls = interpreter.interpret_to_tool_calls(content, tools).await?;
    
    // If no tool calls were detected, return the original message
    if tool_calls.is_empty() {
        return Ok(message);
    }
    
    // Add each tool call to the message
    let mut final_message = message;
    for tool_call in tool_calls {
        let id = Uuid::new_v4().to_string();
        final_message = final_message.with_tool_request(id, Ok(tool_call));
    }
    
    Ok(final_message)
}
