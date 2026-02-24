use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};

use super::api_client::{ApiClient, AuthMethod};
use super::base::{
    ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage,
};
use super::errors::ProviderError;
use super::openai_compatible::map_http_error_to_provider_error;
use super::retry::ProviderRetry;
use crate::conversation::message::{Message, MessageContent};

use crate::mcp_utils::ToolResult;
use crate::model::ModelConfig;
use futures::future::BoxFuture;
use rmcp::model::{object, CallToolRequestParams, Role, Tool};

// ---------- Capability Flags ----------
#[derive(Debug)]
struct CapabilityFlags(String);

impl CapabilityFlags {
    fn from_json(value: &serde_json::Value) -> Self {
        let caps = &value["model_spec"]["capabilities"];
        let mut s = String::with_capacity(6);
        macro_rules! flag {
            ($json_key:literal, $letter:literal) => {
                if caps
                    .get($json_key)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    s.push($letter);
                }
            };
        }
        flag!("optimizedForCode", 'c'); // code
        flag!("supportsVision", 'v'); // vision
        flag!("supportsFunctionCalling", 'f');
        flag!("supportsResponseSchema", 's');
        flag!("supportsWebSearch", 'w');
        flag!("supportsReasoning", 'r');
        CapabilityFlags(s)
    }
}

impl std::fmt::Display for CapabilityFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.0) // e.g. "[cvfsw]"
    }
}
// ---------- END Capability Flags ----------

// ---------- Helpers ----------
/// Return the raw model id (everything before the first space).
fn strip_flags(model: &str) -> &str {
    model.split_whitespace().next().unwrap_or(model)
}
// ---------- END Helpers ----------

const VENICE_PROVIDER_NAME: &str = "venice";
pub const VENICE_DOC_URL: &str = "https://docs.venice.ai/";
pub const VENICE_DEFAULT_MODEL: &str = "llama-3.3-70b";
pub const VENICE_DEFAULT_HOST: &str = "https://api.venice.ai";
pub const VENICE_DEFAULT_BASE_PATH: &str = "api/v1/chat/completions";
pub const VENICE_DEFAULT_MODELS_PATH: &str = "api/v1/models";


#[derive(Debug, Serialize)]
pub struct VeniceProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    models_path: String,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl VeniceProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("VENICE_API_KEY")?;
        let host: String = config
            .get_param("VENICE_HOST")
            .unwrap_or_else(|_| VENICE_DEFAULT_HOST.to_string());
        let base_path: String = config
            .get_param("VENICE_BASE_PATH")
            .unwrap_or_else(|_| VENICE_DEFAULT_BASE_PATH.to_string());
        let models_path: String = config
            .get_param("VENICE_MODELS_PATH")
            .unwrap_or_else(|_| VENICE_DEFAULT_MODELS_PATH.to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?;

        let instance = Self {
            api_client,
            base_path,
            models_path,
            model,
            name: VENICE_PROVIDER_NAME.to_string(),
        };

        Ok(instance)
    }

    async fn post(
        &self,
        session_id: Option<&str>,
        path: &str,
        payload: &Value,
    ) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(session_id, path, payload)
            .await?;

        let status = response.status();
        tracing::debug!("Venice response status: {}", status);

        if !status.is_success() {
            // Read response body for more details on error
            let error_body = response.text().await.unwrap_or_default();

            // Log full error response for debugging
            tracing::debug!("Full Venice error response: {}", error_body);

            // Try to parse the error response
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_body) {
                // Print the full JSON error for better debugging
                println!(
                    "Venice API error response: {}",
                    serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string())
                );

                // Check for tool support errors
                if let Some(details) = json.get("details") {
                    // Specifically look for tool support issues
                    if let Some(tools) = details.get("tools") {
                        if let Some(errors) = tools.get("_errors") {
                            if errors.to_string().contains("not supported by this model") {
                                let model_name = self.model.model_name.clone();
                                return Err(ProviderError::RequestFailed(
                                    format!("The selected model '{}' does not support tool calls. Please select a model that supports tools, such as 'llama-3.3-70b' or 'mistral-31-24b'.", model_name)
                                ));
                            }
                        }
                    }
                }

                // Check for specific error message in context.issues
                if let Some(context) = json.get("context") {
                    if let Some(issues) = context.get("issues") {
                        if let Some(issues_array) = issues.as_array() {
                            for issue in issues_array {
                                if let Some(message) = issue.get("message").and_then(|m| m.as_str())
                                {
                                    if message.contains("tools is not supported by this model") {
                                        let model_name = self.model.model_name.clone();
                                        return Err(ProviderError::RequestFailed(
                                            format!("The selected model '{}' does not support tool calls. Please select a model that supports tools, such as 'llama-3.3-70b' or 'mistral-31-24b'.", model_name)
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Use the common error mapping function
            let error_json = serde_json::from_str::<Value>(&error_body).ok();
            return Err(map_http_error_to_provider_error(status, error_json));
        }

        let response_text = response.text().await?;
        serde_json::from_str(&response_text).map_err(|e| {
            ProviderError::RequestFailed(format!(
                "Failed to parse JSON: {}\nResponse: {}",
                e, response_text
            ))
        })
    }
}

impl ProviderDef for VeniceProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::from_canonical(
            VENICE_PROVIDER_NAME,
            "Venice.ai",
            "Venice.ai models (Llama, DeepSeek, Mistral) with function calling",
            VENICE_DEFAULT_MODEL,
            vec![],
            VENICE_DOC_URL,
            vec![
                ConfigKey::new("VENICE_API_KEY", true, true, None, true),
                ConfigKey::new("VENICE_HOST", true, false, Some(VENICE_DEFAULT_HOST), false),
                ConfigKey::new(
                    "VENICE_BASE_PATH",
                    true,
                    false,
                    Some(VENICE_DEFAULT_BASE_PATH),
                    false,
                ),
                ConfigKey::new(
                    "VENICE_MODELS_PATH",
                    true,
                    false,
                    Some(VENICE_DEFAULT_MODELS_PATH),
                    false,
                ),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for VeniceProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, &self.models_path)
            .response_get()
            .await?;
        let json: serde_json::Value = response.json().await?;

        let mut models = json["data"]
            .as_array()
            .ok_or_else(|| ProviderError::RequestFailed("No data field in JSON".to_string()))?
            .iter()
            .filter_map(|model| {
                let id = model["id"].as_str()?.to_owned();
                // Build flags from capabilities
                let flags = CapabilityFlags::from_json(model);
                // Only include models that support function calling (have 'f' flag)
                if flags.0.contains('f') {
                    Some(format!("{id} {flags}"))
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        models.sort();
        Ok(models)
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = if session_id.is_empty() {
            None
        } else {
            Some(session_id)
        };
        // Create properly formatted messages for Venice API
        let mut formatted_messages = Vec::new();

        // Add the system message if present
        if !system.is_empty() {
            formatted_messages.push(json!({
                "role": "system",
                "content": system
            }));
        }

        // Format regular messages according to Venice API requirements
        for msg in messages {
            // Venice API expects 'content' to be a string, not an array of MessageContent
            let content = match msg.role {
                Role::User => {
                    // For user messages, concatenate all text content
                    let text_content: String = msg
                        .content
                        .iter()
                        .filter_map(|c| c.as_text())
                        .collect::<Vec<_>>()
                        .join("\n");

                    // If we have text content, use it directly
                    if !text_content.is_empty() {
                        text_content
                    } else {
                        // Otherwise, try to get a reasonable string representation
                        msg.as_concat_text()
                    }
                }
                _ => {
                    // For assistant messages, handle possible tool calls
                    let has_tool_calls = msg
                        .content
                        .iter()
                        .any(|c| matches!(c, MessageContent::ToolRequest(_)));

                    if has_tool_calls {
                        // If there are tool calls, we'll handle them separately
                        // Just use an empty string for content
                        "".to_string()
                    } else {
                        // Otherwise use text content
                        msg.as_concat_text()
                    }
                }
            };

            // Create basic message with content as string
            let mut venice_msg = json!({
                "role": match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                "content": content
            });

            // Add debug information to tracing
            tracing::debug!(
                "Venice message format: role={:?}, content_len={}, has_tool_calls={}",
                msg.role,
                content.len(),
                msg.content
                    .iter()
                    .any(|c| matches!(c, MessageContent::ToolRequest(_)))
            );

            // For assistant messages with tool calls, add them in Venice format
            if msg.role == Role::Assistant {
                let tool_calls: Vec<_> = msg
                    .content
                    .iter()
                    .filter_map(|c| c.as_tool_request())
                    .collect();

                if !tool_calls.is_empty() {
                    // Transform our tool calls to Venice format
                    let venice_tool_calls: Vec<Value> = tool_calls
                        .iter()
                        .filter_map(|tr| {
                            if let ToolResult::Ok(tool_call) = &tr.tool_call {
                                // Safely convert arguments to a JSON string
                                let args_str = tool_call
                                    .arguments
                                    .as_ref() // borrow the Option contents
                                    .map(|map| serde_json::to_string(map).unwrap_or_default())
                                    .unwrap_or_default();

                                // Log tool call details for debugging
                                tracing::debug!(
                                    "Tool call conversion: id={}, name={}, args_len={}",
                                    tr.id,
                                    tool_call.name,
                                    args_str.len()
                                );

                                // Convert to Venice format
                                Some(json!({
                                    "id": tr.id,
                                    "type": "function",
                                    "function": {
                                        "name": tool_call.name,
                                        "arguments": args_str
                                    }
                                }))
                            } else {
                                tracing::warn!("Skipping tool call with error: id={}", tr.id);
                                None
                            }
                        })
                        .collect();

                    if !venice_tool_calls.is_empty() {
                        tracing::debug!("Adding {} tool calls to message", venice_tool_calls.len());
                        venice_msg["tool_calls"] = json!(venice_tool_calls);
                    }
                }
            }

            // For tool messages with tool responses, add required tool_call_id
            // Check for tool responses regardless of role - they should have an ID
            // that corresponds to the tool call they're responding to
            {
                let tool_responses: Vec<_> = msg
                    .content
                    .iter()
                    .filter_map(|c| c.as_tool_response())
                    .collect();

                if !tool_responses.is_empty() && !tool_responses[0].id.is_empty() {
                    venice_msg["tool_call_id"] = json!(tool_responses[0].id);
                    // Venice expects tool messages to have 'role' = 'tool'
                    venice_msg["role"] = json!("tool");
                }
            }

            formatted_messages.push(venice_msg);
        }

        // Build Venice-specific payload
        let mut payload = json!({
            "model": strip_flags(&model_config.model_name),
            "messages": formatted_messages,
            "stream": false,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        if !tools.is_empty() {
            // Format tools specifically for Venice API
            let formatted_tools: Vec<serde_json::Value> = tools
                .iter()
                .map(|tool| {
                    // Format each tool in the expected Venice format
                    json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.input_schema
                        }
                    })
                })
                .collect();

            payload["tools"] = json!(formatted_tools);
        }

        tracing::debug!("Sending request to Venice API");
        tracing::debug!("Venice request payload: {}", payload.to_string());

        // Send request with retry
        let response = self
            .with_retry(|| self.post(session_id, &self.base_path, &payload))
            .await?;

        // Parse the response - response is already a Value from our post method
        let response_json = response;

        // Handle tool calls from the response if present
        let tool_calls = response_json["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["tool_calls"].as_array());

        if let Some(tool_calls) = tool_calls {
            if !tool_calls.is_empty() {
                // Extract tool calls and format for our internal model
                let mut content = Vec::new();

                for tool_call in tool_calls {
                    let id = tool_call["id"].as_str().unwrap_or("unknown").to_string();
                    let function = tool_call["function"].clone();
                    let name = function["name"].as_str().unwrap_or("unknown").to_string();

                    // Parse arguments string to Value if it's a string
                    let arguments = if let Some(args_str) = function["arguments"].as_str() {
                        serde_json::from_str::<Value>(args_str)
                            .unwrap_or(function["arguments"].clone())
                    } else {
                        function["arguments"].clone()
                    };

                    let tool_call = CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: name.into(),
                        arguments: Some(object(arguments)),
                    };

                    // Create a ToolRequest MessageContent
                    let tool_request = MessageContent::tool_request(id, ToolResult::Ok(tool_call));

                    content.push(tool_request);
                }

                // Create message and add each content item
                let mut message = Message::assistant();
                for item in content {
                    message = message.with_content(item);
                }

                let provider_usage = ProviderUsage::new(
                    strip_flags(&model_config.model_name).to_string(),
                    Usage::default(),
                );
                return Ok(super::base::stream_from_single_message(
                    message,
                    provider_usage,
                ));
            }
        }

        // If we get here, it's a regular text response
        // Extract content
        let content = response_json["choices"]
            .get(0)
            .and_then(|choice| choice["message"]["content"].as_str())
            .ok_or_else(|| {
                tracing::error!("Invalid response format: {:?}", response_json);
                ProviderError::RequestFailed("Invalid response format: missing content".to_string())
            })?
            .to_string();

        // Create a vector with a single text content item
        let content = vec![MessageContent::text(content)];

        // Extract usage
        let usage_data = &response_json["usage"];
        let usage = Usage::new(
            usage_data["prompt_tokens"].as_i64().map(|v| v as i32),
            usage_data["completion_tokens"].as_i64().map(|v| v as i32),
            usage_data["total_tokens"].as_i64().map(|v| v as i32),
        );

        let message = Message::new(Role::Assistant, Utc::now().timestamp(), content);
        let provider_usage =
            ProviderUsage::new(strip_flags(&self.model.model_name).to_string(), usage);
        Ok(super::base::stream_from_single_message(
            message,
            provider_usage,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_structure() {
        let metadata = VeniceProvider::metadata();

        assert_eq!(metadata.default_model, "llama-3.3-70b");
        assert!(!metadata.known_models.is_empty());

        assert_eq!(metadata.config_keys.len(), 4);
        assert_eq!(metadata.config_keys[0].name, "VENICE_API_KEY");
        assert_eq!(metadata.config_keys[1].name, "VENICE_HOST");
        assert_eq!(metadata.config_keys[2].name, "VENICE_BASE_PATH");
        assert_eq!(metadata.config_keys[3].name, "VENICE_MODELS_PATH");
    }
}
