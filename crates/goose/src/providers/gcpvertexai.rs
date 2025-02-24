use std::format;
use std::time::Duration;
use std::vec;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};

use crate::providers::errors::ProviderError;
use crate::providers::formats::gcpvertexai::{
    create_request,
    get_usage,
    response_to_message,
    ClaudeVersion,
    GcpVertexAIModel,
    GeminiVersion,
    ModelProvider,
    RequestContext,
};

use crate::providers::gcpauth::GcpAuth;
use crate::providers::utils::emit_debug_trace;
use mcp_core::tool::Tool;

/// Base URL for GCP Vertex AI documentation
const GCP_VERTEX_AI_DOC_URL: &str = "https://cloud.google.com/vertex-ai";
/// Fallback default GCP region for model deployment
const GCP_DEFAULT_LOCATION: &str = "us-central1";
/// Default timeout for API requests in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Represents errors specific to GCP Vertex AI operations.
///
/// This enum encompasses various error conditions that might arise when working
/// with the GCP Vertex AI provider, particularly around URL construction and authentication.
#[derive(Debug, thiserror::Error)]
enum GcpVertexAIError {
    /// Error when URL construction fails
    #[error("Invalid URL configuration: {0}")]
    InvalidUrl(String),

    /// Error during GCP authentication
    #[error("Authentication error: {0}")]
    AuthError(String),
}

/// Provider implementation for Google Cloud Platform's Vertex AI service.
///
/// This provider enables interaction with various AI models hosted on GCP Vertex AI,
/// including Claude and Gemini model families. It handles authentication, request routing,
/// and response processing for the Vertex AI API endpoints.
#[derive(Debug, serde::Serialize)]
pub struct GcpVertexAIProvider {
    /// HTTP client for making API requests
    #[serde(skip)]
    client: Client,
    /// GCP authentication handler
    #[serde(skip)]
    auth: GcpAuth,
    /// Base URL for the Vertex AI API
    host: String,
    /// GCP project identifier
    project_id: String,
    /// GCP region for model deployment
    location: String,
    /// Configuration for the specific model being used
    model: ModelConfig,
}

impl GcpVertexAIProvider {
    /// Creates a new provider instance from environment configuration.
    ///
    /// This is a convenience method that initializes the provider using
    /// environment variables and default settings.
    ///
    /// # Arguments
    /// * `model` - Configuration for the model to be used
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        futures::executor::block_on(Self::new(model))
    }

    /// Creates a new provider instance with the specified model configuration.
    ///
    /// Initializes the provider with custom settings and establishes necessary
    /// client connections and authentication.
    ///
    /// # Arguments
    /// * `model` - Configuration for the model to be used
    pub async fn new(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let project_id = config.get("GCP_PROJECT_ID")?;
        let location = Self::determine_location(&config, &model)?;
        let host = format!("https://{}-aiplatform.googleapis.com", location);

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        let auth = GcpAuth::new().await?;

        Ok(Self {
            client,
            auth,
            host,
            project_id,
            location,
            model,
        })
    }

    /// Determines the appropriate GCP location for model deployment.
    ///
    /// Location is determined in the following order:
    /// 1. Custom location from GCP_LOCATION environment variable
    /// 2. Model's default location
    /// 3. Global default location (us-central1)
    fn determine_location(config: &crate::config::Config, model: &ModelConfig) -> Result<String> {
        Ok(config
            .get("GCP_LOCATION")
            .ok()
            .filter(|loc: &String| !loc.trim().is_empty() && loc != "default")
            .unwrap_or_else(|| {
                GcpVertexAIModel::try_from(model.model_name.as_str())
                    .map(|m| m.default_location().to_string())
                    .unwrap_or_else(|_| GCP_DEFAULT_LOCATION.to_string())
            }))
    }

    /// Retrieves an authentication token for API requests.
    ///
    /// # Returns
    /// * `Result<String>` - Bearer token for authentication
    async fn get_auth_header(&self) -> Result<String, GcpVertexAIError> {
        self.auth
            .get_token()
            .await
            .map(|token| format!("Bearer {}", token.token_value))
            .map_err(|e| GcpVertexAIError::AuthError(e.to_string()))
    }

    /// Constructs the appropriate API endpoint URL for a given provider.
    ///
    /// # Arguments
    /// * `provider` - The model provider (Anthropic or Google)
    ///
    /// # Returns
    /// * `Result<Url>` - Fully qualified API endpoint URL
    fn build_request_url(&self, provider: ModelProvider) -> Result<Url, GcpVertexAIError> {
        let base_url = Url::parse(&self.host)
            .map_err(|e| GcpVertexAIError::InvalidUrl(e.to_string()))?;

        let path = format!(
            "v1/projects/{}/locations/{}/publishers/{}/models/{}:{}",
            self.project_id,
            self.location,
            provider.as_str(),
            self.model.model_name,
            match provider {
                ModelProvider::Anthropic => "streamRawPredict",
                ModelProvider::Google => "generateContent",
            }
        );

        base_url
            .join(&path)
            .map_err(|e| GcpVertexAIError::InvalidUrl(e.to_string()))
    }

    /// Makes an authenticated POST request to the Vertex AI API.
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    /// * `context` - Request context containing model information
    ///
    /// # Returns
    /// * `Result<Value>` - JSON response from the API
    async fn post(&self, payload: Value, context: RequestContext) -> Result<Value, ProviderError> {
        let url = self.build_request_url(context.provider())
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let auth_header = self.get_auth_header()
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        let response = self.client
            .post(url)
            .json(&payload)
            .header("Authorization", auth_header)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let response_json = response
            .json::<Value>()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to parse response: {e}")))?;

        match status {
            StatusCode::OK => Ok(response_json),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                tracing::debug!("Authentication failed. Status: {status}, Payload: {payload:?}");
                Err(ProviderError::Authentication(format!(
                    "Authentication failed: {response_json:?}"
                )))
            }
            _ => {
                tracing::debug!("Request failed. Status: {status}, Response: {response_json:?}");
                Err(ProviderError::RequestFailed(format!(
                    "Request failed with status {status}: {response_json:?}"
                )))
            }
        }
    }
}

impl Default for GcpVertexAIProvider {
    fn default() -> Self {
        let model = ModelConfig::new(Self::metadata().default_model);
        futures::executor::block_on(Self::new(model))
            .expect("Failed to initialize VertexAI provider")
    }
}

#[async_trait]
impl Provider for GcpVertexAIProvider {
    /// Returns metadata about the GCP Vertex AI provider.
    ///
    /// This includes information about supported models, configuration requirements,
    /// and documentation links.
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        let known_models = vec![
            GcpVertexAIModel::Claude(ClaudeVersion::Sonnet35),
            GcpVertexAIModel::Claude(ClaudeVersion::Sonnet35V2),
            GcpVertexAIModel::Gemini(GeminiVersion::Pro15),
            GcpVertexAIModel::Gemini(GeminiVersion::Flash20),
            GcpVertexAIModel::Gemini(GeminiVersion::Pro20Exp),
        ]
        .into_iter()
        .map(|model| model.to_string())
        .collect();

        ProviderMetadata::new(
            "gcp_vertex_ai",
            "GCP Vertex AI",
            "Access variety of AI models such as Claude, Gemini through Vertex AI",
            GcpVertexAIModel::Claude(ClaudeVersion::Sonnet35V2).to_string().as_str(),
            known_models,
            GCP_VERTEX_AI_DOC_URL,
            vec![
                ConfigKey::new("GCP_PROJECT_ID", true, false, None),
                ConfigKey::new("GCP_LOCATION", false, false, None),
            ],
        )
    }

    /// Completes a model interaction by sending a request and processing the response.
    ///
    /// # Arguments
    /// * `system` - System prompt or context
    /// * `messages` - Array of previous messages in the conversation
    /// * `tools` - Array of available tools for the model
    ///
    /// # Returns
    /// * `Result<(Message, ProviderUsage)>` - Tuple of response message and usage statistics
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
        let (request, context) = create_request(&self.model, system, messages, tools)?;
        let response = self.post(request.clone(), context.clone()).await?;
        let usage = get_usage(&response, &context)?;

        emit_debug_trace(self, &request, &response, &usage);

        let message = response_to_message(response.clone(), context.clone())?;
        let provider_usage = ProviderUsage::new(self.model.model_name.clone(), usage);

        Ok((message, provider_usage))
    }

    /// Returns the current model configuration.
    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_provider_conversion() {
        assert_eq!(ModelProvider::Anthropic.as_str(), "anthropic");
        assert_eq!(ModelProvider::Google.as_str(), "google");
    }

    #[tokio::test]
    async fn test_url_construction() {
        let model = ModelConfig::new("claude-3-5-sonnet-v2@20241022".to_string());
        let provider = GcpVertexAIProvider {
            client: Client::new(),
            auth: GcpAuth::new().await.expect("Failed to create GcpAuth"),
            host: "https://us-east5-aiplatform.googleapis.com".to_string(),
            project_id: "test-project".to_string(),
            location: "us-east5".to_string(),
            model,
        };

        let url = provider
            .build_request_url(ModelProvider::Anthropic)
            .unwrap()
            .to_string();

        assert!(url.contains("publishers/anthropic"));
        assert!(url.contains("projects/test-project"));
        assert!(url.contains("locations/us-east5"));
    }

    #[test]
    fn test_provider_metadata() {
        let metadata = GcpVertexAIProvider::metadata();
        assert!(metadata.known_models.contains(&"claude-3-5-sonnet-v2@20241022".to_string()));
        assert!(metadata.known_models.contains(&"gemini-1.5-pro-002".to_string()));
        assert_eq!(metadata.config_keys.len(), 2);
    }
}