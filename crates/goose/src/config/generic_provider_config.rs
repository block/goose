//! Configuration schema for Generic HTTP LLM Provider
//!
//! This module defines the configuration structure for connecting to arbitrary LLM APIs
//! that don't follow standard OpenAI/Anthropic formats.

use crate::providers::base::ModelInfo;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Configuration for a generic HTTP LLM provider
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenericProviderConfig {
    /// Unique identifier for this provider (e.g., "custom_my_llm")
    pub name: String,

    /// Display name shown in UI (e.g., "My Company LLM")
    pub display_name: String,

    /// Description of the provider
    pub description: String,

    /// Engine type - must be "generic_http" for this provider type
    #[serde(default = "default_engine")]
    pub engine: String,

    /// Endpoint configuration
    pub endpoint: EndpointConfig,

    /// Authentication configuration
    pub auth: AuthConfig,

    /// Additional headers to include in requests
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request format configuration
    pub request: RequestConfig,

    /// Response parsing configuration
    pub response: ResponseConfig,

    /// Streaming configuration (optional)
    #[serde(default)]
    pub streaming: Option<StreamingConfig>,

    /// Tool injection configuration for LLMs that don't support native tool calling
    #[serde(default)]
    pub tool_injection: Option<ToolInjectionConfig>,

    /// Configuration keys that users need to provide
    pub config_keys: Vec<ConfigKeyDef>,

    /// Available models
    pub models: Vec<ModelInfo>,

    /// Environment variable prefix for config keys (optional)
    /// If not specified, uses provider name in uppercase (e.g., "MY_LLM" -> "MY_LLM_TOKEN")
    /// Example: "GOOSE_CUSTOM" -> "GOOSE_CUSTOM_TOKEN"
    #[serde(default)]
    pub env_prefix: Option<String>,
}

fn default_engine() -> String {
    "generic_http".to_string()
}

impl GenericProviderConfig {
    /// Check if this config is for generic HTTP provider
    pub fn is_generic_http(&self) -> bool {
        self.engine == "generic_http"
    }

    /// Get the provider ID
    pub fn id(&self) -> &str {
        &self.name
    }

    /// Get the display name
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Get available models
    pub fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    /// Get default model name
    pub fn default_model(&self) -> Option<&str> {
        self.models.first().map(|m| m.name.as_str())
    }

    /// Get environment variable prefix for config keys
    /// Returns env_prefix if specified, otherwise uses provider name in uppercase
    pub fn get_env_prefix(&self) -> String {
        self.env_prefix
            .clone()
            .unwrap_or_else(|| self.name.to_uppercase().replace('-', "_"))
    }
}

/// Endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EndpointConfig {
    /// URL template with variable substitution support
    /// Example: "https://${DOMAIN}/api/v1/chat"
    pub url_template: String,

    /// HTTP method (default: POST)
    #[serde(default = "default_method")]
    pub method: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Skip SSL certificate verification (equivalent to curl -k)
    /// Use with caution - only for self-signed certificates or testing
    #[serde(default)]
    pub skip_ssl_verify: bool,
}

fn default_method() -> String {
    "POST".to_string()
}

fn default_timeout() -> u64 {
    120
}

/// Authentication configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// Custom header authentication
    /// Example: x-openapi-token: Bearer ${TOKEN}
    Header {
        header_name: String,
        value_template: String,
    },

    /// Bearer token authentication (Authorization: Bearer ${TOKEN})
    Bearer { token_template: String },

    /// Basic authentication
    Basic {
        username_template: String,
        password_template: String,
    },

    /// Query parameter authentication
    Query {
        param_name: String,
        value_template: String,
    },

    /// No authentication required
    #[default]
    None,
}

/// Request format configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RequestConfig {
    /// JSON template for request body
    /// Supports variable substitution: ${PROMPT}, ${MODEL}, ${IS_STREAM}, etc.
    pub template: Value,

    /// Custom prompt format (optional)
    /// Default: "${SYSTEM}\n\n${HISTORY}\n\nUser: ${USER_QUERY}"
    #[serde(default)]
    pub prompt_format: Option<String>,

    /// Message history format (optional)
    /// Default: "${ROLE}: ${CONTENT}"
    #[serde(default)]
    pub message_format: Option<String>,

    /// Role name mappings (optional)
    #[serde(default)]
    pub role_mappings: Option<RoleMappings>,
}

/// Role name mappings for message formatting
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct RoleMappings {
    #[serde(default = "default_user_role")]
    pub user: String,
    #[serde(default = "default_assistant_role")]
    pub assistant: String,
    #[serde(default = "default_system_role")]
    pub system: String,
}

fn default_user_role() -> String {
    "User".to_string()
}

fn default_assistant_role() -> String {
    "Assistant".to_string()
}

fn default_system_role() -> String {
    "System".to_string()
}

/// Response parsing configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResponseConfig {
    /// JSONPath to extract content from response
    /// Example: "$.content" or "$.choices[0].message.content"
    pub content_path: String,

    /// JSONPath to extract error message (optional)
    #[serde(default)]
    pub error_path: Option<String>,

    /// Usage/token counting configuration (optional)
    #[serde(default)]
    pub usage: Option<UsageConfig>,
}

/// Token usage extraction configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageConfig {
    /// JSONPath for input token count
    #[serde(default)]
    pub input_tokens_path: Option<String>,

    /// JSONPath for output token count
    #[serde(default)]
    pub output_tokens_path: Option<String>,

    /// JSONPath for total token count
    #[serde(default)]
    pub total_tokens_path: Option<String>,
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StreamingConfig {
    /// Whether streaming is supported
    #[serde(default)]
    pub enabled: bool,

    /// Streaming format
    #[serde(default)]
    pub format: StreamingFormat,

    /// JSONPath to extract content from each chunk
    #[serde(default)]
    pub content_path: Option<String>,

    /// JSONPath to detect end of stream
    #[serde(default)]
    pub done_path: Option<String>,

    /// Usage/token counting configuration for streaming (optional)
    /// Token usage is typically included in the final chunk when done=true
    #[serde(default)]
    pub usage: Option<UsageConfig>,
}

/// Streaming response format
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum StreamingFormat {
    /// Server-Sent Events (data: {...}\n\n)
    #[default]
    Sse,
    /// Newline-delimited JSON
    Ndjson,
    /// Raw chunked transfer
    Chunked,
}

/// Tool injection configuration for LLMs without native tool support
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolInjectionConfig {
    /// Whether tool injection is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Format for tool call output
    #[serde(default)]
    pub format: ToolInjectionFormat,

    /// Block name for markdown format (default: "tool_call")
    #[serde(default = "default_block_name")]
    pub block_name: String,

    /// Custom system prompt template for tool injection
    #[serde(default)]
    pub system_template: Option<String>,
}

fn default_block_name() -> String {
    "tool_call".to_string()
}

impl Default for ToolInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            format: ToolInjectionFormat::default(),
            block_name: default_block_name(),
            system_template: None,
        }
    }
}

/// Tool injection output format
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ToolInjectionFormat {
    /// Markdown code block: ```tool_call\n{...}\n```
    #[default]
    MarkdownCodeblock,
    /// XML format: <tool_call>...</tool_call>
    Xml,
    /// Plain JSON object
    Json,
}

/// Configuration key definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfigKeyDef {
    /// Key name (e.g., "TOKEN", "DOMAIN")
    pub name: String,

    /// Whether this key is required
    #[serde(default)]
    pub required: bool,

    /// Whether this should be stored securely (in keyring)
    #[serde(default)]
    pub secret: bool,

    /// Default value (optional)
    #[serde(default)]
    pub default: Option<String>,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,
}

impl ConfigKeyDef {
    /// Convert to provider ConfigKey with custom prefix
    pub fn to_config_key_with_prefix(&self, env_prefix: &str) -> crate::providers::base::ConfigKey {
        let prefixed_name = format!("{}_{}", env_prefix, self.name.to_uppercase());
        crate::providers::base::ConfigKey {
            name: prefixed_name,
            required: self.required,
            secret: self.secret,
            default: self.default.clone(),
            oauth_flow: false,
        }
    }

    /// Convert to provider ConfigKey with prefixed name (uses provider name as prefix)
    pub fn to_config_key(&self, provider_name: &str) -> crate::providers::base::ConfigKey {
        let env_prefix = provider_name.to_uppercase().replace('-', "_");
        self.to_config_key_with_prefix(&env_prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_config() {
        let json = r#"{
            "name": "custom_test",
            "display_name": "Test Provider",
            "description": "A test provider",
            "engine": "generic_http",
            "endpoint": {
                "url_template": "https://${DOMAIN}/api/chat"
            },
            "auth": {
                "type": "bearer",
                "token_template": "${TOKEN}"
            },
            "request": {
                "template": {
                    "prompt": "${PROMPT}",
                    "model": "${MODEL}"
                }
            },
            "response": {
                "content_path": "$.content"
            },
            "config_keys": [
                {"name": "DOMAIN", "required": true, "secret": false},
                {"name": "TOKEN", "required": true, "secret": true}
            ],
            "models": [
                {"name": "default-model", "context_limit": 8000}
            ]
        }"#;

        let config: GenericProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "custom_test");
        assert!(config.is_generic_http());
        assert_eq!(config.config_keys.len(), 2);
    }

    #[test]
    fn test_config_key_prefix() {
        let key_def = ConfigKeyDef {
            name: "TOKEN".to_string(),
            required: true,
            secret: true,
            default: None,
            description: Some("API Token".to_string()),
        };

        let config_key = key_def.to_config_key("custom_my_llm");
        assert_eq!(config_key.name, "CUSTOM_MY_LLM_TOKEN");
        assert!(config_key.required);
        assert!(config_key.secret);
    }

    #[test]
    fn test_generic_http_provider_config() {
        let json = r#"{
            "name": "custom_llm",
            "display_name": "Custom LLM Provider",
            "description": "Custom LLM provider for testing generic HTTP provider",
            "engine": "generic_http",
            "endpoint": {
                "url_template": "http://localhost:8765/v1/chat/messages",
                "method": "POST",
                "timeout_seconds": 120
            },
            "auth": {
                "type": "bearer",
                "token_template": "${TOKEN}"
            },
            "headers": {
                "Content-Type": "application/json"
            },
            "request": {
                "template": {
                    "model": "${MODEL}",
                    "messages": [{"role": "user", "content": "${PROMPT}"}],
                    "stream": "${IS_STREAM:bool}"
                }
            },
            "response": {
                "content_path": "$.content",
                "error_path": "$.error",
                "usage": {
                    "input_tokens_path": "$.usage.input_tokens",
                    "output_tokens_path": "$.usage.output_tokens",
                    "total_tokens_path": "$.usage.total_tokens"
                }
            },
            "tool_injection": {
                "enabled": true,
                "format": "markdown_codeblock",
                "block_name": "tool_call"
            },
            "config_keys": [
                {"name": "TOKEN", "required": true, "secret": true, "description": "API authentication token"}
            ],
            "models": [{"name": "custom-model-v1", "context_limit": 8000}]
        }"#;

        let config: GenericProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "custom_llm");
        assert!(config.is_generic_http());
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.models[0].name, "custom-model-v1");
    }
}
