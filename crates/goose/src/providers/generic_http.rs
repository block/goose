//! Generic HTTP LLM Provider
//!
//! A flexible provider that can connect to any HTTP-based LLM API using
//! configuration-driven request/response mapping.

use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::generic_http::{
    build_variables, extract_by_path, extract_usage, generate_tool_injection, parse_tool_calls,
    substitute_json_template, substitute_template, ParsedToolCall,
};
use crate::config::generic_provider_config::{AuthConfig, GenericProviderConfig, StreamingFormat};
use crate::config::Config;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{object, CallToolRequestParam, Tool};
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::time::Duration;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

/// Generic HTTP LLM Provider
///
/// This provider can connect to any HTTP-based LLM API by using
/// a configuration file that defines:
/// - Endpoint URL and authentication
/// - Request format (with template substitution)
/// - Response parsing (with JSONPath)
/// - Optional tool injection for LLMs without native tool support
#[derive(Debug)]
pub struct GenericHttpProvider {
    /// The provider configuration
    config: GenericProviderConfig,
    /// HTTP client
    client: reqwest::Client,
    /// Resolved configuration values
    resolved_values: HashMap<String, String>,
    /// Model configuration
    model: ModelConfig,
    /// Cached provider name
    name: String,
}

impl GenericHttpProvider {
    /// Create a new GenericHttpProvider from configuration
    pub fn from_config(config: GenericProviderConfig, model: ModelConfig) -> Result<Self> {
        let global_config = Config::global();

        // Resolve all config keys
        let mut resolved_values = HashMap::new();
        let env_prefix = config.get_env_prefix();
        for key_def in &config.config_keys {
            let prefixed_name = format!("{}_{}", env_prefix, key_def.name.to_uppercase());

            let value = if key_def.secret {
                global_config.get_secret(&prefixed_name).ok()
            } else {
                global_config.get_param::<String>(&prefixed_name).ok()
            };

            let value = value
                .or_else(|| key_def.default.clone())
                .ok_or_else(|| anyhow::anyhow!("Missing required config: {}", prefixed_name))?;

            resolved_values.insert(key_def.name.clone(), value);
        }

        // Build HTTP client
        let timeout = Duration::from_secs(config.endpoint.timeout_seconds);
        let mut client_builder = reqwest::Client::builder().timeout(timeout);

        // Skip SSL verification if configured (equivalent to curl -k)
        if config.endpoint.skip_ssl_verify {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let client = client_builder.build()?;

        let name = config.name.clone();

        Ok(Self {
            config,
            client,
            resolved_values,
            model,
            name,
        })
    }

    /// Build the request URL from template
    fn build_url(&self) -> Result<String> {
        substitute_template(&self.config.endpoint.url_template, &self.resolved_values)
    }

    /// Build authentication header
    fn build_auth_header(&self) -> Result<Option<(String, String)>> {
        match &self.config.auth {
            AuthConfig::Header {
                header_name,
                value_template,
            } => {
                let value = substitute_template(value_template, &self.resolved_values)?;
                Ok(Some((header_name.clone(), value)))
            }
            AuthConfig::Bearer { token_template } => {
                let token = substitute_template(token_template, &self.resolved_values)?;
                Ok(Some((
                    "Authorization".to_string(),
                    format!("Bearer {}", token),
                )))
            }
            AuthConfig::Basic {
                username_template,
                password_template,
            } => {
                let username = substitute_template(username_template, &self.resolved_values)?;
                let password = substitute_template(password_template, &self.resolved_values)?;
                let credentials = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    format!("{}:{}", username, password),
                );
                Ok(Some((
                    "Authorization".to_string(),
                    format!("Basic {}", credentials),
                )))
            }
            AuthConfig::Query { .. } => {
                // Query params are handled in URL building
                Ok(None)
            }
            AuthConfig::None => Ok(None),
        }
    }

    /// Build additional headers from config
    fn build_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        for (key, value_template) in &self.config.headers {
            let value = substitute_template(value_template, &self.resolved_values)?;
            let header_name = HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| anyhow::anyhow!("Invalid header name '{}': {}", key, e))?;
            let header_value = HeaderValue::from_str(&value)
                .map_err(|e| anyhow::anyhow!("Invalid header value for '{}': {}", key, e))?;
            headers.insert(header_name, header_value);
        }

        Ok(headers)
    }

    /// Build request body from template
    fn build_request_body(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        is_stream: bool,
    ) -> Result<Value> {
        // Build system prompt with tool injection if enabled
        let tool_injection = self
            .config
            .tool_injection
            .as_ref()
            .map(|cfg| generate_tool_injection(tools, cfg))
            .unwrap_or_default();

        let full_system = if tool_injection.is_empty() {
            system.to_string()
        } else {
            format!("{}{}", system, tool_injection)
        };

        // Build variables for template substitution
        let variables = build_variables(
            &full_system,
            messages,
            &self.model.model_name,
            is_stream,
            &self.resolved_values,
            self.config.request.prompt_format.as_deref(),
            self.config.request.message_format.as_deref(),
            self.config.request.role_mappings.as_ref(),
        );

        // Substitute template
        substitute_json_template(&self.config.request.template, &variables)
    }

    /// Parse response to extract content
    fn parse_response(
        &self,
        response: &Value,
    ) -> Result<(String, Vec<ParsedToolCall>), ProviderError> {
        // Check for error first
        if let Some(error_path) = &self.config.response.error_path {
            if let Some(error) = extract_by_path(response, error_path) {
                if !error.is_null() {
                    let error_msg = match error {
                        Value::String(s) => s,
                        Value::Array(arr) => {
                            // Handle array of error objects
                            arr.iter()
                                .map(|e| {
                                    let error_type =
                                        e.get("type").and_then(|v| v.as_str()).unwrap_or("error");
                                    let msg = e.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                                    format!("[{}] {}", error_type, msg)
                                })
                                .collect::<Vec<_>>()
                                .join("; ")
                        }
                        _ => serde_json::to_string(&error).unwrap_or_default(),
                    };
                    return Err(ProviderError::RequestFailed(error_msg));
                }
            }
        }

        // Extract content
        let content = extract_by_path(response, &self.config.response.content_path)
            .map(|v| match v {
                Value::String(s) => s,
                _ => v.to_string(),
            })
            .ok_or_else(|| {
                ProviderError::RequestFailed(format!(
                    "Failed to extract content from response using path: {}",
                    self.config.response.content_path
                ))
            })?;

        // Parse tool calls if injection is enabled
        let (remaining_content, tool_calls) = if let Some(tool_config) = &self.config.tool_injection
        {
            parse_tool_calls(&content, tool_config)
        } else {
            (content, Vec::new())
        };

        Ok((remaining_content, tool_calls))
    }

    /// Extract usage information from response
    fn extract_usage(&self, response: &Value) -> Usage {
        if let Some(usage_config) = &self.config.response.usage {
            extract_usage(
                response,
                usage_config.input_tokens_path.as_deref(),
                usage_config.output_tokens_path.as_deref(),
                usage_config.total_tokens_path.as_deref(),
            )
        } else {
            Usage::default()
        }
    }

    /// Build a request and return the reqwest::RequestBuilder
    fn build_request(&self, body: &Value) -> Result<reqwest::RequestBuilder, ProviderError> {
        let url = self
            .build_url()
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to build URL: {}", e)))?;

        let headers = self
            .build_headers()
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to build headers: {}", e)))?;

        let mut request = self.client.post(&url).headers(headers).json(body);

        // Add auth header
        if let Some((header_name, header_value)) = self.build_auth_header().map_err(|e| {
            ProviderError::Authentication(format!("Failed to build auth header: {}", e))
        })? {
            request = request.header(&header_name, &header_value);
        }

        // Add query auth if configured
        if let AuthConfig::Query {
            param_name,
            value_template,
        } = &self.config.auth
        {
            let value = substitute_template(value_template, &self.resolved_values)
                .map_err(|e| ProviderError::Authentication(e.to_string()))?;
            request = request.query(&[(param_name, value)]);
        }

        Ok(request)
    }
}

#[async_trait]
impl Provider for GenericHttpProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        // This is a placeholder - actual metadata comes from config
        ProviderMetadata::empty()
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let body = self
            .build_request_body(system, messages, tools, false)
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to build request body: {}", e))
            })?;

        let request = self.build_request(&body)?;

        // Log request
        tracing::debug!(
            "GenericHttpProvider request: {}",
            serde_json::to_string(&body).unwrap_or_default()
        );

        // Send request
        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => ProviderError::Authentication(format!(
                    "Authentication failed ({}): {}",
                    status, error_body
                )),
                429 => ProviderError::RateLimitExceeded {
                    details: error_body,
                    retry_delay: None,
                },
                500..=599 => {
                    ProviderError::ServerError(format!("Server error ({}): {}", status, error_body))
                }
                _ => ProviderError::RequestFailed(format!(
                    "Request failed ({}): {}",
                    status, error_body
                )),
            });
        }

        // Parse response
        let response_body: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse response JSON: {}", e))
        })?;

        tracing::debug!(
            "GenericHttpProvider response: {}",
            serde_json::to_string(&response_body).unwrap_or_default()
        );

        let (content, tool_calls) = self.parse_response(&response_body)?;
        let usage = self.extract_usage(&response_body);

        // Build message
        let mut message = Message::assistant();

        if !content.is_empty() {
            message = message.with_text(&content);
        }

        for tc in tool_calls {
            message = message.with_tool_request(
                tc.id,
                Ok(CallToolRequestParam {
                    name: tc.name.into(),
                    arguments: Some(object(tc.arguments)),
                }),
            );
        }

        Ok((
            message,
            ProviderUsage::new(model_config.model_name.clone(), usage),
        ))
    }

    fn supports_streaming(&self) -> bool {
        self.config
            .streaming
            .as_ref()
            .map(|s| s.enabled)
            .unwrap_or(false)
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let body = self
            .build_request_body(system, messages, tools, true)
            .map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to build request body: {}", e))
            })?;

        let request = self.build_request(&body)?;

        tracing::debug!(
            "GenericHttpProvider streaming request: {}",
            serde_json::to_string(&body).unwrap_or_default()
        );

        let response = request
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 | 403 => ProviderError::Authentication(format!(
                    "Authentication failed ({}): {}",
                    status, error_body
                )),
                429 => ProviderError::RateLimitExceeded {
                    details: error_body,
                    retry_delay: None,
                },
                500..=599 => {
                    ProviderError::ServerError(format!("Server error ({}): {}", status, error_body))
                }
                _ => ProviderError::RequestFailed(format!(
                    "Request failed ({}): {}",
                    status, error_body
                )),
            });
        }

        // Get streaming config and validate format
        let streaming_config = self.config.streaming.clone();

        // Only SSE and Chunked formats are supported for streaming
        if let Some(ref cfg) = streaming_config {
            match cfg.format {
                StreamingFormat::Sse | StreamingFormat::Chunked => {} // OK
                StreamingFormat::Ndjson => {
                    return Err(ProviderError::RequestFailed(
                        "NDJSON streaming format is not supported.".to_string(),
                    ));
                }
            }
        }

        let response_config = self.config.response.clone();
        let tool_injection_config = self.config.tool_injection.clone();
        let model_name = self.model.model_name.clone();

        // Convert response to byte stream
        let byte_stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(byte_stream);
            let mut framed = FramedRead::new(stream_reader, LinesCodec::new())
                .map_err(anyhow::Error::from);

            let mut accumulated_content = String::new();
            let mut final_usage = Usage::default();

            while let Some(line_result) = framed.next().await {
                let line = line_result.map_err(|e| {
                    ProviderError::RequestFailed(format!("Stream read error: {}", e))
                })?;

                // Skip empty lines
                if line.trim().is_empty() {
                    continue;
                }

                // Handle SSE format: data: {...}
                let data_str = if line.starts_with("data: ") {
                    line.strip_prefix("data: ").unwrap_or(&line)
                } else {
                    continue;
                };

                // Handle end of stream markers
                if data_str.trim() == "[DONE]" {
                    break;
                }

                // Parse JSON chunk
                let chunk: Value = match serde_json::from_str(data_str) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::debug!("Failed to parse streaming chunk: {} - {}", e, data_str);
                        continue;
                    }
                };

                // Extract content using streaming config path or response config path
                let content_path = streaming_config.as_ref()
                    .and_then(|s| s.content_path.as_deref())
                    .unwrap_or(&response_config.content_path);

                let content = extract_by_path(&chunk, content_path)
                    .map(|v| match v {
                        Value::String(s) => s,
                        _ => v.to_string(),
                    })
                    .unwrap_or_default();

                // Check if done
                let done = streaming_config.as_ref()
                    .and_then(|s| s.done_path.as_ref())
                    .and_then(|done_path| extract_by_path(&chunk, done_path))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Extract usage from streaming chunk (typically in the final chunk when done=true)
                if let Some(ref stream_cfg) = streaming_config {
                    if let Some(ref usage_cfg) = stream_cfg.usage {
                        let usage = extract_usage(
                            &chunk,
                            usage_cfg.input_tokens_path.as_deref(),
                            usage_cfg.output_tokens_path.as_deref(),
                            usage_cfg.total_tokens_path.as_deref(),
                        );
                        // Update final_usage if we got valid token counts
                        if usage.input_tokens.is_some() || usage.output_tokens.is_some() {
                            final_usage = usage;
                        }
                    }
                }

                if !content.is_empty() {
                    accumulated_content.push_str(&content);

                    // Yield text message for each chunk
                    let message = Message::assistant().with_text(&content);
                    yield (Some(message), None);
                }

                if done {
                    break;
                }
            }

            // Parse tool calls from accumulated content if tool injection is enabled
            if let Some(tool_config) = &tool_injection_config {
                let (_remaining_content, tool_calls) = parse_tool_calls(&accumulated_content, tool_config);

                if !tool_calls.is_empty() {
                    let mut final_message = Message::assistant();

                    for tc in tool_calls {
                        final_message = final_message.with_tool_request(
                            tc.id,
                            Ok(CallToolRequestParam {
                                name: tc.name.into(),
                                arguments: Some(object(tc.arguments)),
                            }),
                        );
                    }

                    yield (Some(final_message), Some(ProviderUsage::new(model_name.clone(), final_usage)));
                }
            }

            // Yield final usage if we have it
            if final_usage.input_tokens.is_some() || final_usage.output_tokens.is_some() {
                yield (None, Some(ProviderUsage::new(model_name, final_usage)));
            }

        }))
    }
}

/// Create ProviderMetadata from GenericProviderConfig
pub fn metadata_from_config(config: &GenericProviderConfig) -> ProviderMetadata {
    let env_prefix = config.get_env_prefix();
    let config_keys: Vec<ConfigKey> = config
        .config_keys
        .iter()
        .map(|k| k.to_config_key_with_prefix(&env_prefix))
        .collect();

    ProviderMetadata::with_models(
        &config.name,
        &config.display_name,
        &config.description,
        config.default_model().unwrap_or("default"),
        config.models.clone(),
        "",
        config_keys,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let config_json = r#"{
            "name": "test_provider",
            "display_name": "Test",
            "description": "Test provider",
            "endpoint": {
                "url_template": "https://api.example.com/v1/chat"
            },
            "auth": {"type": "none"},
            "request": {
                "template": {"prompt": "${PROMPT}"}
            },
            "response": {
                "content_path": "$.content"
            },
            "config_keys": [],
            "models": [{"name": "test-model", "context_limit": 8000}]
        }"#;

        let config: GenericProviderConfig = serde_json::from_str(config_json).unwrap();
        let model = ModelConfig::new("test-model").unwrap();
        let provider = GenericHttpProvider {
            config,
            client: reqwest::Client::new(),
            resolved_values: HashMap::new(),
            model,
            name: "test_provider".to_string(),
        };

        let url = provider.build_url().unwrap();
        assert_eq!(url, "https://api.example.com/v1/chat");
    }
}
