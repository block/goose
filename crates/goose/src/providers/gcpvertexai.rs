use std::time::Duration;

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
    create_request, get_usage, response_to_message, ClaudeVersion, GcpVertexAIModel, GeminiVersion,
    ModelProvider, RequestContext,
};

use crate::providers::formats::gcpvertexai::GcpLocation::Iowa;
use crate::providers::gcpauth::GcpAuth;
use crate::providers::utils::emit_debug_trace;
use mcp_core::tool::Tool;

/// Base URL for GCP Vertex AI documentation
const GCP_VERTEX_AI_DOC_URL: &str = "https://cloud.google.com/vertex-ai";
/// Default timeout for API requests in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Represents errors specific to GCP Vertex AI operations.
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
        Self::new(model)
    }

    /// Creates a new provider instance with the specified model configuration.
    ///
    /// # Arguments
    /// * `model` - Configuration for the model to be used
    pub fn new(model: ModelConfig) -> Result<Self> {
        futures::executor::block_on(Self::new_async(model))
    }

    /// Async implementation of new provider instance creation.
    ///
    /// # Arguments
    /// * `model` - Configuration for the model to be used
    async fn new_async(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let project_id = config.get("GCP_PROJECT_ID")?;
        let location = Self::determine_location(&config)?;
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
    /// 2. Global default location (Iowa)
    fn determine_location(config: &crate::config::Config) -> Result<String> {
        Ok(config
            .get("GCP_LOCATION")
            .ok()
            .filter(|location: &String| !location.trim().is_empty())
            .unwrap_or_else(|| Iowa.to_string()))
    }

    /// Retrieves an authentication token for API requests.
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
    /// * `location` - The GCP location for model deployment
    fn build_request_url(
        &self,
        provider: ModelProvider,
        location: &str,
    ) -> Result<Url, GcpVertexAIError> {
        // Create host URL for the specified location
        let host_url = if self.location == location {
            self.host.clone()
        } else {
            // Only allocate a new string if location differs
            self.host.replace(&self.location, location)
        };

        let base_url =
            Url::parse(&host_url).map_err(|e| GcpVertexAIError::InvalidUrl(e.to_string()))?;

        // Determine endpoint based on provider type
        let endpoint = match provider {
            ModelProvider::Anthropic => "streamRawPredict",
            ModelProvider::Google => "generateContent",
        };

        // Construct path for URL
        let path = format!(
            "v1/projects/{}/locations/{}/publishers/{}/models/{}:{}",
            self.project_id,
            location,
            provider.as_str(),
            self.model.model_name,
            endpoint
        );

        base_url
            .join(&path)
            .map_err(|e| GcpVertexAIError::InvalidUrl(e.to_string()))
    }

    /// Makes an authenticated POST request to the Vertex AI API at a specific location.
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    /// * `context` - Request context containing model information
    /// * `location` - The GCP location for the request
    async fn post_with_location(
        &self,
        payload: &Value,
        context: &RequestContext,
        location: &str,
    ) -> Result<Value, ProviderError> {
        let url = self
            .build_request_url(context.provider(), location)
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let auth_header = self
            .get_auth_header()
            .await
            .map_err(|e| ProviderError::Authentication(e.to_string()))?;

        let response = self
            .client
            .post(url)
            .json(payload)
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

    /// Makes an authenticated POST request to the Vertex AI API with fallback for invalid locations.
    ///
    /// # Arguments
    /// * `payload` - The request payload to send
    /// * `context` - Request context containing model information
    async fn post(&self, payload: Value, context: &RequestContext) -> Result<Value, ProviderError> {
        // Try with user-specified location first
        let result = self
            .post_with_location(&payload, context, &self.location)
            .await;

        // If location is already the known location for the model or request succeeded, return result
        if self.location == context.model.known_location().to_string() || result.is_ok() {
            return result;
        }

        // Check if we should retry with the model's known location
        match &result {
            Err(ProviderError::RequestFailed(msg)) => {
                let model_name = context.model.to_string();
                let configured_location = &self.location;
                let known_location = context.model.known_location().to_string();

                tracing::error!(
                    "Trying known location {known_location} for {model_name} instead of {configured_location}: {msg}"
                );

                self.post_with_location(&payload, context, &known_location)
                    .await
            }
            // For any other error, return the original result
            _ => result,
        }
    }
}

impl Default for GcpVertexAIProvider {
    fn default() -> Self {
        let model = ModelConfig::new(Self::metadata().default_model);
        Self::new(model).expect("Failed to initialize VertexAI provider")
    }
}

#[async_trait]
impl Provider for GcpVertexAIProvider {
    /// Returns metadata about the GCP Vertex AI provider.
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        let known_models = vec![
            GcpVertexAIModel::Claude(ClaudeVersion::Sonnet35),
            GcpVertexAIModel::Claude(ClaudeVersion::Sonnet35V2),
            GcpVertexAIModel::Claude(ClaudeVersion::Sonnet37),
            GcpVertexAIModel::Claude(ClaudeVersion::Haiku35),
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
            GcpVertexAIModel::Gemini(GeminiVersion::Flash20)
                .to_string()
                .as_str(),
            known_models,
            GCP_VERTEX_AI_DOC_URL,
            vec![
                ConfigKey::new("GCP_PROJECT_ID", true, false, None),
                ConfigKey::new("GCP_LOCATION", true, false, Some(Iowa.to_string().as_str())),
            ],
        )
    }

    /// Completes a model interaction by sending a request and processing the response.
    ///
    /// # Arguments
    /// * `system` - System prompt or context
    /// * `messages` - Array of previous messages in the conversation
    /// * `tools` - Array of available tools for the model
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
        // Create request and context
        let (request, context) = create_request(&self.model, system, messages, tools)?;

        // Send request and process response
        let response = self.post(request.clone(), &context).await?;
        let usage = get_usage(&response, &context)?;

        emit_debug_trace(self, &request, &response, &usage);

        // Convert response to message
        let message = response_to_message(response, context)?;
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

    #[test]
    fn test_url_construction() {
        use url::Url;

        let model_config = ModelConfig::new("claude-3-5-sonnet-v2@20241022".to_string());
        let context = RequestContext::new(&model_config.model_name).unwrap();
        let api_model_id = context.model.to_string();

        let host = "https://us-east5-aiplatform.googleapis.com";
        let project_id = "test-project";
        let location = "us-east5";

        let path = format!(
            "v1/projects/{}/locations/{}/publishers/{}/models/{}:{}",
            project_id,
            location,
            ModelProvider::Anthropic.as_str(),
            api_model_id,
            "streamRawPredict"
        );

        let url = Url::parse(host).unwrap().join(&path).unwrap();

        assert!(url.as_str().contains("publishers/anthropic"));
        assert!(url.as_str().contains("projects/test-project"));
        assert!(url.as_str().contains("locations/us-east5"));
    }

    #[test]
    fn test_provider_metadata() {
        let metadata = GcpVertexAIProvider::metadata();
        assert!(metadata
            .known_models
            .contains(&"claude-3-5-sonnet-v2@20241022".to_string()));
        assert!(metadata
            .known_models
            .contains(&"gemini-1.5-pro-002".to_string()));
        assert_eq!(metadata.config_keys.len(), 2);
    }
}
