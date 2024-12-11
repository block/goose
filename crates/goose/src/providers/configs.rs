use super::utils::ImageFormat;
use serde::{Deserialize, Serialize};

const DEFAULT_CLIENT_ID: &str = "databricks-cli";
const DEFAULT_REDIRECT_URL: &str = "http://localhost:8020";
const DEFAULT_SCOPES: &[&str] = &["all-apis"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderConfig {
    OpenAi(OpenAiProviderConfig),
    Databricks(DatabricksProviderConfig),
    Ollama(OllamaProviderConfig),
    Anthropic(AnthropicProviderConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabricksAuth {
    Token(String),
    OAuth {
        host: String,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl DatabricksAuth {
    /// Create a new OAuth configuration with default values
    pub fn oauth(host: String) -> Self {
        Self::OAuth {
            host,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksProviderConfig {
    pub host: String,
    pub model: String,
    pub auth: DatabricksAuth,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub image_format: ImageFormat,
}

impl DatabricksProviderConfig {
    /// Create a new configuration with token authentication
    pub fn with_token(host: String, model: String, token: String) -> Self {
        Self {
            host,
            model,
            auth: DatabricksAuth::Token(token),
            temperature: None,
            max_tokens: None,
            image_format: ImageFormat::Anthropic,
        }
    }

    /// Create a new configuration with OAuth authentication using default settings
    pub fn with_oauth(host: String, model: String) -> Self {
        Self {
            host: host.clone(),
            model,
            auth: DatabricksAuth::oauth(host),
            temperature: None,
            max_tokens: None,
            image_format: ImageFormat::Anthropic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProviderConfig {
    pub host: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaProviderConfig {
    pub host: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicProviderConfig {
    pub host: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
}
