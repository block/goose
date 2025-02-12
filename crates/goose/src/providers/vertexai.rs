use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use gcp_sdk_auth::credentials::create_access_token_credential;
use reqwest::Client;
use serde_json::Value;

use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use crate::providers::errors::ProviderError;
use crate::providers::formats::vertexai::{create_request, get_usage, response_to_message};
use crate::providers::utils::emit_debug_trace;
use mcp_core::tool::Tool;

pub const VERTEXAI_DEFAULT_MODEL: &str = "claude-3-5-sonnet-v2@20241022";
pub const VERTEXAI_KNOWN_MODELS: &[&str] = &[
    "claude-3-5-sonnet-v2@20241022",
    "claude-3-5-sonnet@20240620",
];
pub const VERTEXAI_DOC_URL: &str = "https://cloud.google.com/vertex-ai";
pub const VERTEXAI_DEFAULT_REGION: &str = "us-east5";

#[derive(Debug, serde::Serialize)]
pub struct VertexAIProvider {
    #[serde(skip)]
    client: Client,
    host: String,
    project_id: String,
    region: String,
    model: ModelConfig,
}

impl VertexAIProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        let project_id = config.get("VERTEXAI_PROJECT_ID")?;
        let region = config
            .get("VERTEXAI_REGION")
            .unwrap_or_else(|_| VERTEXAI_DEFAULT_REGION.to_string());
        let host = config
            .get("VERTEXAI_API_HOST")
            .unwrap_or_else(|_| format!("https://{}-aiplatform.googleapis.com", region));

        let client = Client::builder()
            .timeout(Duration::from_secs(600))
            .build()?;

        Ok(VertexAIProvider {
            client,
            host,
            project_id,
            region,
            model,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let path = format!(
            "v1/projects/{}/locations/{}/publishers/{}/models/{}:streamRawPredict",
            self.project_id,
            self.region,
            self.get_model_provider(),
            self.model.model_name
        );
        let url = base_url.join(&path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let creds = create_access_token_credential().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to create access token credential: {}", e))
        })?;
        let token = creds.get_token().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to get access token: {}", e))
        })?;

        let response = self
            .client
            .post(url)
            .json(&payload)
            .header("Authorization", format!("Bearer {}", token.token))
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Request failed: {}", e)))?;

        let status = response.status();
        let response_json = response.json::<Value>().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse response: {}", e))
        })?;

        match status {
            reqwest::StatusCode::OK => Ok(response_json),
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
                tracing::debug!(
                    "{}",
                    format!(
                        "Provider request failed with status: {}. Payload: {:?}",
                        status, payload
                    )
                );
                Err(ProviderError::Authentication(format!(
                    "Authentication failed: {:?}",
                    response_json
                )))
            }
            _ => {
                tracing::debug!(
                    "{}",
                    format!("Request failed with status {}: {:?}", status, response_json)
                );
                Err(ProviderError::RequestFailed(format!(
                    "Request failed with status {}: {:?}",
                    status, response_json
                )))
            }
        }
    }

    fn get_model_provider(&self) -> String {
        // TODO: switch this by model_name
        "anthropic".to_string()
    }
}

impl Default for VertexAIProvider {
    fn default() -> Self {
        let model = ModelConfig::new(Self::metadata().default_model);
        VertexAIProvider::from_env(model).expect("Failed to initialize VertexAI provider")
    }
}

#[async_trait]
impl Provider for VertexAIProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            "vertex_ai",
            "Vertex AI",
            "Access variety of AI models such as Claude through Vertex AI",
            VERTEXAI_DEFAULT_MODEL,
            VERTEXAI_KNOWN_MODELS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            VERTEXAI_DOC_URL,
            vec![
                ConfigKey::new("VERTEXAI_PROJECT_ID", true, false, None),
                ConfigKey::new(
                    "VERTEXAI_REGION",
                    true,
                    false,
                    Some(VERTEXAI_DEFAULT_REGION),
                ),
            ],
        )
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
        let request = create_request(&self.model, system, messages, tools)?;
        let response = self.post(request.clone()).await?;
        let usage = get_usage(&response)?;

        emit_debug_trace(self, &request, &response, &usage);

        let message = response_to_message(response.clone())?;
        let provider_usage = ProviderUsage::new(self.model.model_name.clone(), usage);

        Ok((message, provider_usage))
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }
}
