use super::{anthropic, google};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use anyhow::{Context, Result};
use rmcp::model::Tool;
use serde_json::Value;

use std::fmt;

/// Sensible default values of Google Cloud Platform (GCP) locations for model deployment.
///
/// Each variant corresponds to a specific GCP region where models can be hosted.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum GcpLocation {
    /// Represents the us-central1 region in Iowa
    Iowa,
    /// Represents the us-east5 region in Ohio
    Ohio,
}

impl fmt::Display for GcpLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Iowa => write!(f, "us-central1"),
            Self::Ohio => write!(f, "us-east5"),
        }
    }
}

impl TryFrom<&str> for GcpLocation {
    type Error = ModelError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "us-central1" => Ok(Self::Iowa),
            "us-east5" => Ok(Self::Ohio),
            _ => Err(ModelError::UnsupportedLocation(s.to_string())),
        }
    }
}

/// Represents errors that can occur during model operations.
///
/// This enum encompasses various error conditions that might arise when working
/// with GCP Vertex AI models, including unsupported models, invalid requests,
/// and unsupported locations.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    /// Error when an unsupported Vertex AI model is specified
    #[error("Unsupported Vertex AI model: {0}")]
    UnsupportedModel(String),
    /// Error when the request structure is invalid
    #[error("Invalid request structure: {0}")]
    InvalidRequest(String),
    /// Error when an unsupported GCP location is specified
    #[error("Unsupported GCP location: {0}")]
    UnsupportedLocation(String),
}

/// Represents available GCP Vertex AI models for goose.
///
/// This enum encompasses different model families and their versions
/// that are supported in the GCP Vertex AI platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcpVertexAIModel {
    /// Claude model family with specific versions
    Claude(ClaudeVersion),
    /// Gemini model family with specific versions
    Gemini(GeminiVersion),
    /// MaaS (Model as a Service) models from Model Garden
    /// Contains (publisher, full_model_name)
    MaaS(String, String),
}

/// Represents available versions of the Claude model for goose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeVersion {
    /// Claude 3.7 Sonnet
    Sonnet37,
    /// Claude Sonnet 4
    Sonnet4,
    /// Claude Opus 4
    Opus4,
    /// Generic Claude model for custom or new versions
    Generic(String),
}

/// Represents available versions of the Gemini model for goose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeminiVersion {
    /// Gemini 1.5 Pro version
    Pro15,
    /// Gemini 2.0 Flash version
    Flash20,
    /// Gemini 2.0 Pro Experimental version
    Pro20Exp,
    /// Gemini 2.5 Pro Experimental version
    Pro25Exp,
    /// Gemini 2.5 Flash Preview version
    Flash25Preview,
    /// Gemini 2.5 Pro Preview version
    Pro25Preview,
    /// Gemini 2.5 Flash version
    Flash25,
    /// Gemini 2.5 Pro version
    Pro25,
    /// Generic Gemini model for custom or new versions
    Generic(String),
}

impl fmt::Display for GcpVertexAIModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let model_id = match self {
            Self::Claude(version) => match version {
                ClaudeVersion::Sonnet37 => "claude-3-7-sonnet@20250219",
                ClaudeVersion::Sonnet4 => "claude-sonnet-4@20250514",
                ClaudeVersion::Opus4 => "claude-opus-4@20250514",
                ClaudeVersion::Generic(name) => name,
            },
            Self::Gemini(version) => match version {
                GeminiVersion::Pro15 => "gemini-1.5-pro-002",
                GeminiVersion::Flash20 => "gemini-2.0-flash-001",
                GeminiVersion::Pro20Exp => "gemini-2.0-pro-exp-02-05",
                GeminiVersion::Pro25Exp => "gemini-2.5-pro-exp-03-25",
                GeminiVersion::Flash25Preview => "gemini-2.5-flash-preview-05-20",
                GeminiVersion::Pro25Preview => "gemini-2.5-pro-preview-05-06",
                GeminiVersion::Flash25 => "gemini-2.5-flash",
                GeminiVersion::Pro25 => "gemini-2.5-pro",
                GeminiVersion::Generic(name) => name,
            },
            Self::MaaS(_, model_name) => model_name,
        };
        write!(f, "{model_id}")
    }
}

impl GcpVertexAIModel {
    /// Returns the default GCP location for the model.
    ///
    /// Each model family has a well-known location based on availability:
    /// - Claude models default to Ohio (us-east5)
    /// - Gemini models default to Iowa (us-central1)
    /// - MaaS models default to Iowa (us-central1)
    pub fn known_location(&self) -> GcpLocation {
        match self {
            Self::Claude(_) => GcpLocation::Ohio,
            Self::Gemini(_) => GcpLocation::Iowa,
            Self::MaaS(_, _) => GcpLocation::Iowa,
        }
    }
}

impl TryFrom<&str> for GcpVertexAIModel {
    type Error = ModelError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Known models
        match s {
            "claude-3-7-sonnet@20250219" => Ok(Self::Claude(ClaudeVersion::Sonnet37)),
            "claude-sonnet-4@20250514" => Ok(Self::Claude(ClaudeVersion::Sonnet4)),
            "claude-opus-4@20250514" => Ok(Self::Claude(ClaudeVersion::Opus4)),
            "gemini-1.5-pro-002" => Ok(Self::Gemini(GeminiVersion::Pro15)),
            "gemini-2.0-flash-001" => Ok(Self::Gemini(GeminiVersion::Flash20)),
            "gemini-2.0-pro-exp-02-05" => Ok(Self::Gemini(GeminiVersion::Pro20Exp)),
            "gemini-2.5-pro-exp-03-25" => Ok(Self::Gemini(GeminiVersion::Pro25Exp)),
            "gemini-2.5-flash-preview-05-20" => Ok(Self::Gemini(GeminiVersion::Flash25Preview)),
            "gemini-2.5-pro-preview-05-06" => Ok(Self::Gemini(GeminiVersion::Pro25Preview)),
            "gemini-2.5-flash" => Ok(Self::Gemini(GeminiVersion::Flash25)),
            "gemini-2.5-pro" => Ok(Self::Gemini(GeminiVersion::Pro25)),
            // MaaS models (Model as a Service from Model Garden)
            _ if s.ends_with("-maas") => {
                let publisher = s
                    .split('-')
                    .next()
                    .ok_or_else(|| ModelError::UnsupportedModel(s.to_string()))?
                    .to_string();
                Ok(Self::MaaS(publisher, s.to_string()))
            }
            // Generic models based on prefix matching
            _ if s.starts_with("claude-") => {
                Ok(Self::Claude(ClaudeVersion::Generic(s.to_string())))
            }
            _ if s.starts_with("gemini-") => {
                Ok(Self::Gemini(GeminiVersion::Generic(s.to_string())))
            }
            _ => Err(ModelError::UnsupportedModel(s.to_string())),
        }
    }
}

/// Holds context information for a model request since the Vertex AI platform
/// supports multiple model families.
///
/// This structure maintains information about the model being used
/// and provides utility methods for handling model-specific operations.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// The GCP Vertex AI model being used
    pub model: GcpVertexAIModel,
}

impl RequestContext {
    /// Creates a new RequestContext from a model ID string.
    ///
    /// # Arguments
    /// * `model_id` - The string identifier of the model
    ///
    /// # Returns
    /// * `Result<Self>` - A new RequestContext if the model ID is valid
    pub fn new(model_id: &str) -> Result<Self> {
        Ok(Self {
            model: GcpVertexAIModel::try_from(model_id)
                .with_context(|| format!("Failed to parse model ID: {model_id}"))?,
        })
    }

    /// Returns the provider associated with the model.
    pub fn provider(&self) -> ModelProvider {
        match &self.model {
            GcpVertexAIModel::Claude(_) => ModelProvider::Anthropic,
            GcpVertexAIModel::Gemini(_) => ModelProvider::Google,
            GcpVertexAIModel::MaaS(publisher, _) => ModelProvider::MaaS(publisher.clone()),
        }
    }
}

/// Represents available model providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelProvider {
    /// Anthropic provider (Claude models)
    Anthropic,
    /// Google provider (Gemini models)
    Google,
    /// MaaS provider (Model as a Service from Model Garden)
    MaaS(String),
}

impl ModelProvider {
    /// Returns the string representation of the provider.
    pub fn as_str(&self) -> String {
        match self {
            Self::Anthropic => "anthropic".to_string(),
            Self::Google => "google".to_string(),
            Self::MaaS(publisher) => publisher.clone(),
        }
    }
}

/// Creates an Anthropic-specific Vertex AI request payload.
///
/// # Arguments
/// * `model_config` - Configuration for the model
/// * `system` - System prompt
/// * `messages` - Array of messages
/// * `tools` - Array of available tools
///
/// # Returns
/// * `Result<Value>` - JSON request payload for Anthropic API
fn create_anthropic_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let mut request = anthropic::create_request(model_config, system, messages, tools)?;

    let obj = request
        .as_object_mut()
        .ok_or_else(|| ModelError::InvalidRequest("Request is not a JSON object".to_string()))?;

    // Note: We don't need to specify the model in the request body
    // The model is determined by the endpoint URL in GCP Vertex AI
    obj.remove("model");
    obj.insert(
        "anthropic_version".to_string(),
        Value::String("vertex-2023-10-16".to_string()),
    );

    Ok(request)
}

/// Creates a Gemini-specific Vertex AI request payload.
///
/// # Arguments
/// * `model_config` - Configuration for the model
/// * `system` - System prompt
/// * `messages` - Array of messages
/// * `tools` - Array of available tools
///
/// # Returns
/// * `Result<Value>` - JSON request payload for Google API
fn create_google_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    google::create_request(model_config, system, messages, tools)
}

/// Creates a provider-specific request payload and context.
///
/// # Arguments
/// * `model_config` - Configuration for the model
/// * `system` - System prompt
/// * `messages` - Array of messages
/// * `tools` - Array of available tools
///
/// # Returns
/// * `Result<(Value, RequestContext)>` - Tuple of request payload and context
pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<(Value, RequestContext)> {
    let context = RequestContext::new(&model_config.model_name)?;

    let request = match &context.model {
        GcpVertexAIModel::Claude(_) => {
            create_anthropic_request(model_config, system, messages, tools)?
        }
        GcpVertexAIModel::Gemini(_) => {
            create_google_request(model_config, system, messages, tools)?
        }
        GcpVertexAIModel::MaaS(_, _) => {
            // TODO: Branch on publisher for format selection once we know which
            // MaaS providers use which formats (e.g., OpenAI vs Google format)
            // For now, default to Google format since most use generateContent endpoint
            create_google_request(model_config, system, messages, tools)?
        }
    };

    Ok((request, context))
}

/// Converts a provider response to a Message.
///
/// # Arguments
/// * `response` - The raw response from the provider
/// * `request_context` - Context information about the request
///
/// # Returns
/// * `Result<Message>` - Converted message
pub fn response_to_message(response: Value, request_context: RequestContext) -> Result<Message> {
    match request_context.provider() {
        ModelProvider::Anthropic => anthropic::response_to_message(&response),
        ModelProvider::Google => google::response_to_message(response),
        ModelProvider::MaaS(_) => google::response_to_message(response),
    }
}

/// Extracts token usage information from the response data.
///
/// # Arguments
/// * `data` - The response data containing usage information
/// * `request_context` - Context information about the request
///
/// # Returns
/// * `Result<Usage>` - Usage statistics
pub fn get_usage(data: &Value, request_context: &RequestContext) -> Result<Usage> {
    match request_context.provider() {
        ModelProvider::Anthropic => anthropic::get_usage(data),
        ModelProvider::Google => google::get_usage(data),
        ModelProvider::MaaS(_) => google::get_usage(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_model_parsing() -> Result<()> {
        let valid_models = [
            "claude-sonnet-4-20250514",
            "claude-3-7-sonnet@20250219",
            "claude-sonnet-4@20250514",
            "gemini-1.5-pro-002",
            "gemini-2.0-flash-001",
            "gemini-2.0-pro-exp-02-05",
            "gemini-2.5-pro-exp-03-25",
            "gemini-2.5-flash-preview-05-20",
            "gemini-2.5-pro-preview-05-06",
        ];

        for model_id in valid_models {
            let model = GcpVertexAIModel::try_from(model_id)?;
            assert_eq!(model.to_string(), model_id);
        }

        assert!(GcpVertexAIModel::try_from("unsupported-model").is_err());
        Ok(())
    }

    #[test]
    fn test_default_locations() -> Result<()> {
        let test_cases = [
            ("claude-sonnet-4-20250514", GcpLocation::Ohio),
            ("claude-3-7-sonnet@20250219", GcpLocation::Ohio),
            ("claude-sonnet-4@20250514", GcpLocation::Ohio),
            ("gemini-1.5-pro-002", GcpLocation::Iowa),
            ("gemini-2.0-flash-001", GcpLocation::Iowa),
            ("gemini-2.0-pro-exp-02-05", GcpLocation::Iowa),
            ("gemini-2.5-pro-exp-03-25", GcpLocation::Iowa),
            ("gemini-2.5-flash-preview-05-20", GcpLocation::Iowa),
            ("gemini-2.5-pro-preview-05-06", GcpLocation::Iowa),
        ];

        for (model_id, expected_location) in test_cases {
            let model = GcpVertexAIModel::try_from(model_id)?;
            assert_eq!(
                model.known_location(),
                expected_location,
                "Model {model_id} should have default location {expected_location:?}",
            );

            let context = RequestContext::new(model_id)?;
            assert_eq!(
                context.model.known_location(),
                expected_location,
                "RequestContext for {model_id} should have default location {expected_location:?}",
            );
        }

        Ok(())
    }

    #[test]
    fn test_generic_model_parsing() -> Result<()> {
        // Test generic Claude models
        let claude_models = [
            "claude-3-8-apex@20250301",
            "claude-new-version",
            "claude-experimental",
        ];

        for model_id in claude_models {
            let model = GcpVertexAIModel::try_from(model_id)?;
            match model {
                GcpVertexAIModel::Claude(ClaudeVersion::Generic(ref name)) => {
                    assert_eq!(name, model_id);
                }
                _ => panic!("Expected Claude generic model for {model_id}"),
            }
            assert_eq!(model.to_string(), model_id);
            assert_eq!(model.known_location(), GcpLocation::Ohio);
        }

        // Test generic Gemini models
        let gemini_models = ["gemini-3-pro", "gemini-2.0-flash", "gemini-experimental"];

        for model_id in gemini_models {
            let model = GcpVertexAIModel::try_from(model_id)?;
            match model {
                GcpVertexAIModel::Gemini(GeminiVersion::Generic(ref name)) => {
                    assert_eq!(name, model_id);
                }
                _ => panic!("Expected Gemini generic model for {model_id}"),
            }
            assert_eq!(model.to_string(), model_id);
            assert_eq!(model.known_location(), GcpLocation::Iowa);
        }

        Ok(())
    }
}
