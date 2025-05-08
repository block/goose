use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::formats::snowflake::{create_request, get_usage, response_to_message};
use super::oauth;
use super::utils::{get_model, ImageFormat};
use crate::config::ConfigError;
use crate::message::Message;
use crate::model::ModelConfig;
use mcp_core::tool::Tool;
use url::Url;

const DEFAULT_CLIENT_ID: &str = "snowflake-cli";
const DEFAULT_REDIRECT_URL: &str = "http://localhost:8020";
// "offline_access" scope is used to request an OAuth 2.0 Refresh Token
// https://openid.net/specs/openid-connect-core-1_0.html#OfflineAccess
const DEFAULT_SCOPES: &[&str] = &["all-apis", "offline_access"];

pub const SNOWFLAKE_DEFAULT_MODEL: &str = "claude-3-5-sonnet";
// Snowflake can passthrough to a wide range of models, we only provide the default
pub const SNOWFLAKE_KNOWN_MODELS: &[&str] = &["claude-3-5-sonnet", "snowflake-llama-3.1-405b"];

pub const SNOWFLAKE_DOC_URL: &str =
    "https://docs.snowflake.com/en/user-guide/snowflake-cortex/llm-functions#choosing-a-model";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnowflakeAuth {
    Token(String),
    OAuth {
        host: String,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl SnowflakeAuth {
    /// Create a new OAuth configuration with default values
    pub fn oauth(host: String) -> Self {
        Self::OAuth {
            host,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }
    pub fn token(token: String) -> Self {
        Self::Token(token)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SnowflakeProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    auth: SnowflakeAuth,
    model: ModelConfig,
    image_format: ImageFormat,
}

impl Default for SnowflakeProvider {
    fn default() -> Self {
        let model = ModelConfig::new(SnowflakeProvider::metadata().default_model);
        SnowflakeProvider::from_env(model).expect("Failed to initialize Snowflake provider")
    }
}

impl SnowflakeProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        // For compatibility for now we check both config and secret for snowflake host
        // but it is not actually a secret value
        let mut host: Result<String, ConfigError> = config.get_param("SNOWFLAKE_HOST");

        if host.is_err() {
            host = config.get_secret("SNOWFLAKE_HOST")
        }

        if host.is_err() {
            return Err(ConfigError::NotFound(
                "Did not find SNOWFLAKE_HOST in either config file or keyring".to_string(),
            )
            .into());
        }

        let host = host?;

        let mut user: Result<String, ConfigError> = config.get_param("SNOWFLAKE_USER");

        if user.is_err() {
            user = config.get_secret("SNOWFLAKE_USER")
        }

        if user.is_err() {
            return Err(ConfigError::NotFound(
                "Did not find SNOWFLAKE_TOKEN in either config file or keyring".to_string(),
            )
            .into());
        }

        let mut token: Result<String, ConfigError> = config.get_param("SNOWFLAKE_TOKEN");

        if token.is_err() {
            token = config.get_secret("SNOWFLAKE_TOKEN")
        }

        if token.is_err() {
            return Err(ConfigError::NotFound(
                "Did not find SNOWFLAKE_TOKEN in either config file or keyring".to_string(),
            )
            .into());
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        // If we find a snowflake token we prefer that
        if let Ok(api_key) = config.get_secret("SNOWFLAKE_TOKEN") {
            return Ok(Self {
                client,
                host,
                auth: SnowflakeAuth::token(api_key),
                model,
                image_format: ImageFormat::OpenAi,
            });
        }

        // Otherwise use Oauth flow
        Ok(Self {
            client,
            auth: SnowflakeAuth::oauth(host.clone()),
            host,
            model,
            image_format: ImageFormat::OpenAi,
        })
    }

    async fn ensure_auth_header(&self) -> Result<String> {
        match &self.auth {
            SnowflakeAuth::Token(token) => Ok(format!("Snowflake Token=\"{}\"", token)),
            SnowflakeAuth::OAuth {
                host,
                client_id,
                redirect_url,
                scopes,
            } => {
                let token =
                    oauth::get_oauth_token_async(host, client_id, redirect_url, scopes).await?;
                Ok(format!("Bearer {}", token))
            }
        }
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url_str =
            if !self.host.starts_with("https://") && !self.host.starts_with("http://") {
                format!("https://{}", self.host)
            } else {
                self.host.clone()
            };
        let base_url = Url::parse(&base_url_str)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let path = "api/v2/cortex/inference:complete";
        let url = base_url.join(&path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let auth_header = self.ensure_auth_header().await?;
        let response = self
            .client
            .post(url)
            .header("Authorization", auth_header)
            .header("User-Agent", "Goose")
            .json(&payload)
            .send()
            .await?;

        let status = response.status();

        let payload_text: String = response.text().await.ok().unwrap_or_default();

        if status == StatusCode::OK {
            if let Ok(payload) = serde_json::from_str::<Value>(&payload_text) {
                if payload.get("code").is_some() {
                    return Err(ProviderError::RequestFailed(format!(
                        "{} - {}",
                        payload.get("code").unwrap(),
                        payload.get("message").unwrap()
                    )));
                }
            }
        }

        let lines = payload_text.lines().collect::<Vec<_>>();

        let mut text = String::new();
        let mut tool_name = String::new();
        let mut tool_input = String::new();
        let mut tool_use_id = String::new();
        for line in lines.iter() {
            if line.is_empty() {
                continue;
            }
            let json_line: Value =
                serde_json::from_str(line.strip_prefix("data: ").unwrap()).unwrap();
            let choices = json_line.get("choices").unwrap();
            let choice = choices.get(0).unwrap();
            let delta = choice.get("delta").unwrap();
            let content_list = delta.get("content_list").unwrap();
            let content = content_list.get(0).unwrap();
            if delta.get("type").unwrap() == "text" {
                text.push_str(content.get("text").unwrap().as_str().unwrap());
            } else if delta.get("type").unwrap() == "tool_use" {
                if content.get("tool_use_id").is_some() {
                    tool_name.push_str(content.get("name").unwrap().as_str().unwrap());
                    tool_use_id.push_str(content.get("tool_use_id").unwrap().as_str().unwrap());
                }
                if content.get("input").is_some() {
                    tool_input.push_str(content.get("input").unwrap().as_str().unwrap());
                }
            }
        }

        let answer_payload = json!(
            {
                "role": "assistant",
                "content": text,
                "content_list": [
                    {
                        "type": "tool_use",
                        "tool_use_id": tool_use_id,
                        "name": tool_name,
                        "input": tool_input
                    }
                ]
            }
        );

        match status {
            StatusCode::OK => Ok(answer_payload),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ProviderError::Authentication(format!("Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                    Status: {}. Response: {:?}", status, payload)))
            }
            StatusCode::BAD_REQUEST => {
                // Snowflake provides a generic 'error' but also includes 'external_model_message' which is provider specific
                // We try to extract the error message from the payload and check for phrases that indicate context length exceeded
                let payload_str = serde_json::to_string(&payload).unwrap_or_default().to_lowercase();
                let check_phrases = [
                    "too long",
                    "context length",
                    "context_length_exceeded",
                    "reduce the length",
                    "token count",
                    "exceeds",
                ];
                if check_phrases.iter().any(|c| payload_str.contains(c)) {
                    return Err(ProviderError::ContextLengthExceeded(payload_str));
                }

                // try to convert message to string, if that fails use external_model_message
                let error_msg = payload
                        .get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| {
                            payload.get("external_model_message")
                                .and_then(|ext| ext.get("message"))
                                .and_then(|m| m.as_str())
                        })
                        .unwrap_or("Unknown error").to_string();
                tracing::debug!(
                    "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
                );
                Err(ProviderError::RequestFailed(format!("Request failed with status: {}. Message: {}", status, error_msg)))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
            }
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                Err(ProviderError::ServerError(format!("{:?}", payload)))
            }
            _ => {
                tracing::debug!(
                    "{}", format!("Provider request failed with status: {}. Payload: {:?}", status, payload)
                );
                Err(ProviderError::RequestFailed(format!("Request failed with status: {}", status)))
            }
        }
    }
}

#[async_trait]
impl Provider for SnowflakeProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "snowflake",
            "Snowflake",
            "Access several models using Snowflake Cortex services.",
            SNOWFLAKE_DEFAULT_MODEL,
            SNOWFLAKE_KNOWN_MODELS.to_vec(),
            SNOWFLAKE_DOC_URL,
            vec![
                ConfigKey::new("SNOWFLAKE_HOST", true, false, None),
                ConfigKey::new("SNOWFLAKE_USER", true, false, None),
                ConfigKey::new("SNOWFLAKE_TOKEN", true, true, None),
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
        let payload = create_request(&self.model, system, messages, tools)?;

        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = get_usage(&response)?;
        let model = get_model(&response);
        super::utils::emit_debug_trace(&self.model, &payload, &response, &usage);

        Ok((message, ProviderUsage::new(model, usage)))
    }
}
