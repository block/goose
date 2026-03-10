use crate::config::paths::Paths;
use crate::providers::api_client::{ApiClient, AuthMethod};
use crate::providers::openai_compatible::{handle_status_openai_compat, stream_openai_compat};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use axum::http;
use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::base::{Provider, ProviderDef, ProviderMetadata, ProviderUsage, Usage};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::openai_compatible::handle_response_openai_compat;
use super::retry::ProviderRetry;
use super::utils::{get_model, ImageFormat, RequestLog};

use crate::config::{Config, ConfigError};
use crate::conversation::message::Message;

use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, MessageStream};
use futures::future::BoxFuture;
use rmcp::model::Tool;

const GITHUB_COPILOT_PROVIDER_NAME: &str = "github_copilot";
pub const GITHUB_COPILOT_DEFAULT_MODEL: &str = "gpt-4.1";
pub const GITHUB_COPILOT_KNOWN_MODELS: &[&str] = &[
    "gpt-4.1",
    "gpt-5-mini",
    "gpt-5",
    "gpt-4o",
    "grok-code-fast-1",
    "gpt-5-codex",
    "claude-sonnet-4",
    "claude-sonnet-4.5",
    "claude-haiku-4.5",
    "gemini-2.5-pro",
];

pub const GITHUB_COPILOT_STREAM_MODELS: &[&str] = &[
    "gpt-4.1",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-codex",
    "gemini-2.5-pro",
    "grok-code-fast-1",
];

const GITHUB_COPILOT_DOC_URL: &str =
    "https://docs.github.com/en/copilot/using-github-copilot/ai-models";
const GITHUB_COPILOT_CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const GITHUB_COPILOT_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_COPILOT_ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_COPILOT_API_KEY_URL: &str = "https://api.github.com/copilot_internal/v2/token";

#[derive(Debug, Deserialize)]
struct DeviceCodeInfo {
    device_code: String,
    user_code: String,
    verification_uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CopilotTokenEndpoints {
    api: String,
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)] // useful for debugging
struct CopilotTokenInfo {
    token: String,
    expires_at: i64,
    refresh_in: i64,
    endpoints: CopilotTokenEndpoints,
    #[serde(flatten)]
    _extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CopilotState {
    expires_at: DateTime<Utc>,
    info: CopilotTokenInfo,
}

#[derive(Debug)]
struct DiskCache {
    cache_path: PathBuf,
}

impl DiskCache {
    fn new() -> Self {
        let cache_path = Paths::in_config_dir("githubcopilot/info.json");
        Self { cache_path }
    }

    async fn load(&self) -> Option<CopilotState> {
        if let Ok(contents) = tokio::fs::read_to_string(&self.cache_path).await {
            if let Ok(info) = serde_json::from_str::<CopilotState>(&contents) {
                return Some(info);
            }
        }
        None
    }

    async fn save(&self, info: &CopilotState) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let contents = serde_json::to_string(info)?;
        tokio::fs::write(&self.cache_path, contents).await?;
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct GithubCopilotProvider {
    #[serde(skip)]
    client: Client,
    #[serde(skip)]
    cache: DiskCache,
    #[serde(skip)]
    mu: tokio::sync::Mutex<RefCell<Option<CopilotState>>>,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
}

impl GithubCopilotProvider {
    fn payload_contains_image(payload: &Value) -> bool {
        payload
            .get("messages")
            .and_then(|m| m.as_array())
            .is_some_and(|messages| {
                messages.iter().any(|msg| {
                    msg.get("content").is_some_and(|content| {
                        content
                            .as_array()
                            .map(|arr| arr.iter().collect::<Vec<_>>())
                            .unwrap_or_else(|| vec![content])
                            .iter()
                            .any(|item| {
                                matches!(
                                    item.get("type").and_then(|v| v.as_str()),
                                    Some("image_url") | Some("image")
                                )
                            })
                    })
                })
            })
    }

    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;
        let cache = DiskCache::new();
        let mu = tokio::sync::Mutex::new(RefCell::new(None));
        Ok(Self {
            client,
            cache,
            mu,
            model,
            name: GITHUB_COPILOT_PROVIDER_NAME.to_string(),
        })
    }

    async fn post(
        &self,
        session_id: Option<&str>,
        payload: &mut Value,
    ) -> Result<Response, ProviderError> {
        let (endpoint, token) = self.get_api_info().await?;
        let auth = AuthMethod::BearerToken(token);
        let mut headers = self.get_github_headers();
        if Self::payload_contains_image(payload) {
            headers.insert("Copilot-Vision-Request", "true".parse().unwrap());
        }
        let api_client = ApiClient::new(endpoint.clone(), auth)?.with_headers(headers)?;

        api_client
            .response_post(session_id, "chat/completions", payload)
            .await
            .map_err(|e| e.into())
    }

    async fn get_api_info(&self) -> Result<(String, String)> {
        let guard = self.mu.lock().await;

        if let Some(state) = guard.borrow().as_ref() {
            if state.expires_at > Utc::now() {
                return Ok((state.info.endpoints.api.clone(), state.info.token.clone()));
            }
        }

        if let Some(state) = self.cache.load().await {
            if guard.borrow().is_none() {
                guard.replace(Some(state.clone()));
            }
            if state.expires_at > Utc::now() {
                return Ok((state.info.endpoints.api, state.info.token));
            }
        }

        const MAX_ATTEMPTS: i32 = 3;
        for attempt in 0..MAX_ATTEMPTS {
            tracing::trace!("attempt {} to refresh api info", attempt + 1);
            let info = match self.refresh_api_info().await {
                Ok(data) => data,
                Err(err) => {
                    tracing::warn!("failed to refresh api info: {}", err);
                    continue;
                }
            };
            let expires_at = Utc::now() + chrono::Duration::seconds(info.refresh_in);
            let new_state = CopilotState { info, expires_at };
            self.cache.save(&new_state).await?;
            guard.replace(Some(new_state.clone()));
            return Ok((new_state.info.endpoints.api, new_state.info.token));
        }
        Err(anyhow!("failed to get api info after 3 attempts"))
    }

    async fn refresh_api_info(&self) -> Result<CopilotTokenInfo> {
        let config = Config::global();
        let token = match config.get_secret::<String>("GITHUB_COPILOT_TOKEN") {
            Ok(token) => token,
            Err(err) => match err {
                ConfigError::NotFound(_) => {
                    let token = self
                        .get_access_token()
                        .await
                        .context("unable to login into github")?;
                    config.set_secret("GITHUB_COPILOT_TOKEN", &token)?;
                    token
                }
                _ => return Err(err.into()),
            },
        };
        let resp = self
            .client
            .get(GITHUB_COPILOT_API_KEY_URL)
            .headers(self.get_github_headers())
            .header(http::header::AUTHORIZATION, format!("bearer {}", &token))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        tracing::trace!("copilot token response: {}", resp);
        let info: CopilotTokenInfo = serde_json::from_str(&resp)?;
        Ok(info)
    }

    async fn get_access_token(&self) -> Result<String> {
        for attempt in 0..3 {
            tracing::trace!("attempt {} to get access token", attempt + 1);
            match self.login().await {
                Ok(token) => return Ok(token),
                Err(err) => tracing::warn!("failed to get access token: {}", err),
            }
        }
        Err(anyhow!("failed to get access token after 3 attempts"))
    }

    async fn login(&self) -> Result<String> {
        let device_code_info = self.get_device_code().await?;

        println!(
            "Please visit {} and enter code {}",
            device_code_info.verification_uri, device_code_info.user_code
        );

        self.poll_for_access_token(&device_code_info.device_code)
            .await
    }

    async fn get_device_code(&self) -> Result<DeviceCodeInfo> {
        #[derive(Serialize)]
        struct DeviceCodeRequest {
            client_id: String,
            scope: String,
        }
        self.client
            .post(GITHUB_COPILOT_DEVICE_CODE_URL)
            .headers(self.get_github_headers())
            .json(&DeviceCodeRequest {
                client_id: GITHUB_COPILOT_CLIENT_ID.to_string(),
                scope: "read:user".to_string(),
            })
            .send()
            .await
            .context("failed to send request to get device code")?
            .error_for_status()
            .context("failed to get device code")?
            .json::<DeviceCodeInfo>()
            .await
            .context("failed to parse device code response")
    }

    async fn poll_for_access_token(&self, device_code: &str) -> Result<String> {
        #[derive(Serialize)]
        struct AccessTokenRequest {
            client_id: String,
            device_code: String,
            grant_type: String,
        }
        #[derive(Debug, Deserialize)]
        struct AccessTokenResponse {
            access_token: Option<String>,
            error: Option<String>,
            #[serde(flatten)]
            _extra: HashMap<String, Value>,
        }

        const MAX_ATTEMPTS: i32 = 36;
        for attempt in 0..MAX_ATTEMPTS {
            let resp = self
                .client
                .post(GITHUB_COPILOT_ACCESS_TOKEN_URL)
                .headers(self.get_github_headers())
                .json(&AccessTokenRequest {
                    client_id: GITHUB_COPILOT_CLIENT_ID.to_string(),
                    device_code: device_code.to_string(),
                    grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
                })
                .send()
                .await
                .context("failed to make request while polling for access token")?
                .error_for_status()
                .context("error polling for access token")?
                .json::<AccessTokenResponse>()
                .await
                .context("failed to parse response while polling for access token")?;
            if resp.access_token.is_some() {
                tracing::trace!("successful authorization: {:#?}", resp,);
            }
            if let Some(access_token) = resp.access_token {
                return Ok(access_token);
            } else if resp
                .error
                .as_ref()
                .is_some_and(|err| err == "authorization_pending")
            {
                tracing::debug!(
                    "authorization pending (attempt {}/{})",
                    attempt + 1,
                    MAX_ATTEMPTS
                );
            } else {
                tracing::debug!("unexpected response: {:#?}", resp);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
        Err(anyhow!("failed to get access token"))
    }

    fn get_github_headers(&self) -> http::HeaderMap {
        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::ACCEPT, "application/json".parse().unwrap());
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert(
            http::header::USER_AGENT,
            "GithubCopilot/1.155.0".parse().unwrap(),
        );
        headers.insert("editor-version", "vscode/1.85.1".parse().unwrap());
        headers.insert("editor-plugin-version", "copilot/1.155.0".parse().unwrap());
        headers
    }
}

impl ProviderDef for GithubCopilotProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            GITHUB_COPILOT_PROVIDER_NAME,
            "GitHub Copilot",
            "GitHub Copilot. Run `goose configure` and select copilot to set up.",
            GITHUB_COPILOT_DEFAULT_MODEL,
            GITHUB_COPILOT_KNOWN_MODELS.to_vec(),
            GITHUB_COPILOT_DOC_URL,
            vec![ConfigKey::new_oauth(
                "GITHUB_COPILOT_TOKEN",
                true,
                true,
                None,
                false,
            )],
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
impl Provider for GithubCopilotProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Check if this model supports streaming
        let supports_streaming = GITHUB_COPILOT_STREAM_MODELS
            .iter()
            .any(|prefix| model_config.model_name.starts_with(prefix));

        if supports_streaming {
            // Use streaming API
            let payload = create_request(
                model_config,
                system,
                messages,
                tools,
                &ImageFormat::OpenAi,
                true,
            )?;
            let mut log = RequestLog::start(model_config, &payload)?;

            let response = self
                .with_retry(|| async {
                    let mut payload_clone = payload.clone();
                    let resp = self.post(Some(session_id), &mut payload_clone).await?;
                    handle_status_openai_compat(resp).await
                })
                .await
                .inspect_err(|e| {
                    let _ = log.error(e);
                })?;

            stream_openai_compat(response, log)
        } else {
            // Use non-streaming API and wrap result
            let session_id_opt = if session_id.is_empty() {
                None
            } else {
                Some(session_id)
            };
            let payload = create_request(
                model_config,
                system,
                messages,
                tools,
                &ImageFormat::OpenAi,
                false,
            )?;
            let mut log = RequestLog::start(model_config, &payload)?;

            // Make request with retry
            let response = self
                .with_retry(|| async {
                    let mut payload_clone = payload.clone();
                    self.post(session_id_opt, &mut payload_clone).await
                })
                .await?;
            let response = handle_response_openai_compat(response).await?;

            let response = promote_tool_choice(response);

            // Parse response
            let message = response_to_message(&response)?;
            let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data");
                Usage::default()
            });
            let response_model = get_model(&response);
            log.write(&response, Some(&usage))?;

            Ok(super::base::stream_from_single_message(
                message,
                ProviderUsage::new(response_model, usage),
            ))
        }
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let (endpoint, token) = self.get_api_info().await?;
        let url = format!("{}/models", endpoint);

        let mut headers = http::HeaderMap::new();
        headers.insert(http::header::ACCEPT, "application/json".parse().unwrap());
        headers.insert(
            http::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        headers.insert("Copilot-Integration-Id", "vscode-chat".parse().unwrap());
        headers.insert(
            http::header::AUTHORIZATION,
            format!("Bearer {}", token).parse().unwrap(),
        );

        let response = self.client.get(url).headers(headers).send().await?;

        let json: serde_json::Value = response.json().await?;

        let arr = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::RequestFailed(
                "Missing 'data' array in GitHub Copilot models response".to_string(),
            )
        })?;
        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| {
                if let Some(s) = m.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = m.as_object() {
                    obj.get("id").and_then(|v| v.as_str()).map(str::to_string)
                } else {
                    None
                }
            })
            .collect();
        models.sort();
        Ok(models)
    }

    async fn configure_oauth(&self) -> Result<(), ProviderError> {
        let config = Config::global();

        // Check if token already exists and is valid
        if config.get_secret::<String>("GITHUB_COPILOT_TOKEN").is_ok() {
            // Try to refresh API info to validate the token
            match self.refresh_api_info().await {
                Ok(_) => return Ok(()), // Token is valid
                Err(_) => {
                    // Token is invalid, continue with OAuth flow
                    tracing::debug!("Existing token is invalid, starting OAuth flow");
                }
            }
        }

        // Start OAuth device code flow
        let token = self
            .get_access_token()
            .await
            .map_err(|e| ProviderError::Authentication(format!("OAuth flow failed: {}", e)))?;

        // Save the token
        config
            .set_secret("GITHUB_COPILOT_TOKEN", &token)
            .map_err(|e| ProviderError::ExecutionError(format!("Failed to save token: {}", e)))?;

        Ok(())
    }
}

// Copilot sometimes returns multiple choices in a completion response for
// Claude models and places the `tool_calls` payload in a non-zero index choice.
// Example:
// - Choice 0: {"finish_reason":"stop","message":{"content":"I'll check the Desktop directory…"}}
// - Choice 1: {"finish_reason":"tool_calls","message":{"tool_calls":[{"function":{"arguments":"{\"command\":
//   \"ls -1 ~/Desktop | wc -l\"}","name":"developer__shell"},…}]}}
// This function ensures the first choice contains tool metadata so the shared formatter emits a
// `ToolRequest` instead of returning only the plain-text choice.
fn promote_tool_choice(response: Value) -> Value {
    let Some(choices) = response.get("choices").and_then(|c| c.as_array()) else {
        return response;
    };

    let tool_choice_idx = choices.iter().position(|choice| {
        choice
            .get("message")
            .and_then(|m| m.get("tool_calls"))
            .and_then(|tc| tc.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    });

    if let Some(idx) = tool_choice_idx {
        if idx != 0 {
            let mut new_response = response;
            if let Some(new_choices) = new_response
                .get_mut("choices")
                .and_then(|c| c.as_array_mut())
            {
                let choice = new_choices.remove(idx);
                new_choices.insert(0, choice);
            }
            return new_response;
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::promote_tool_choice;
    use serde_json::json;

    #[test]
    fn promotes_choice_with_tool_call() {
        let response = json!({
            "choices": [
                {"message": {"content": "plain text"}},
                {"message": {"tool_calls": [{"function": {"name": "foo", "arguments": "{}"}}]}}
            ]
        });

        let promoted = promote_tool_choice(response);
        assert_eq!(
            promoted
                .get("choices")
                .and_then(|c| c.as_array())
                .map(|c| c.len()),
            Some(2)
        );
        let first_choice = promoted
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .unwrap();

        assert!(first_choice
            .get("message")
            .and_then(|m| m.get("tool_calls"))
            .is_some());
    }

    #[test]
    fn leaves_response_when_tool_choice_first() {
        let response = json!({
            "choices": [
                {"message": {"tool_calls": [{"function": {"name": "foo", "arguments": "{}"}}]}},
                {"message": {"content": "plain text"}}
            ]
        });

        let promoted = promote_tool_choice(response.clone());
        assert_eq!(promoted, response);
    }
}
