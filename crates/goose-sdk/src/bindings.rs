//! In-process uniffi bindings for the Goose SDK.
//!
//! This is the API surface exposed to Python and Kotlin. It currently focuses
//! on declarative providers: consumers can construct a provider from JSON and
//! stream completions from it.

use std::sync::{Arc, Mutex};

use futures::StreamExt;
use goose_providers::{
    base::{MessageStream, Provider},
    conversation::message::Message,
    declarative::EnvKeyResolver,
    model::ModelConfig,
};

/// Errors surfaced across the uniffi boundary.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GooseError {
    #[error("{0}")]
    Generic(String),
}

impl From<anyhow::Error> for GooseError {
    fn from(error: anyhow::Error) -> Self {
        Self::Generic(error.to_string())
    }
}

impl From<goose_providers::errors::ProviderError> for GooseError {
    fn from(error: goose_providers::errors::ProviderError) -> Self {
        Self::Generic(error.to_string())
    }
}

impl From<serde_json::Error> for GooseError {
    fn from(error: serde_json::Error) -> Self {
        Self::Generic(error.to_string())
    }
}

/// A text message passed to a provider.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderMessage {
    pub role: MessageRole,
    pub text: String,
}

/// Supported message roles for provider requests and streamed responses.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum MessageRole {
    User,
    Assistant,
}

impl ProviderMessage {
    fn to_goose_message(&self) -> Message {
        match self.role {
            MessageRole::User => Message::user().with_text(&self.text),
            MessageRole::Assistant => Message::assistant().with_text(&self.text),
        }
    }
}

/// Model selection and optional generation settings for a provider request.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderModelConfig {
    pub model_name: String,
    #[uniffi(default = None)]
    pub context_limit: Option<u64>,
    #[uniffi(default = None)]
    pub temperature: Option<f32>,
    #[uniffi(default = None)]
    pub max_tokens: Option<i32>,
    #[uniffi(default = false)]
    pub toolshim: bool,
    #[uniffi(default = None)]
    pub toolshim_model: Option<String>,
    /// Provider-specific request parameters as a JSON object string.
    #[uniffi(default = None)]
    pub request_params_json: Option<String>,
    #[uniffi(default = None)]
    pub reasoning: Option<bool>,
}

impl ProviderModelConfig {
    fn to_goose_model_config(&self) -> Result<ModelConfig, GooseError> {
        let mut config = ModelConfig::new(&self.model_name)
            .with_context_limit(self.context_limit.map(|limit| limit as usize))
            .with_temperature(self.temperature)
            .with_max_tokens(self.max_tokens)
            .with_toolshim(self.toolshim)
            .with_toolshim_model(self.toolshim_model.clone());

        if let Some(request_params_json) = &self.request_params_json {
            let request_params = serde_json::from_str(request_params_json)?;
            config = config.with_merged_request_params(request_params);
        }

        config.reasoning = self.reasoning;
        Ok(config)
    }
}

/// One item yielded by a provider stream.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ProviderStreamChunk {
    /// The concatenated text content in this message chunk, if one was emitted.
    pub text: Option<String>,
    /// Full Goose message JSON for callers that need non-text content such as tool requests.
    pub message_json: Option<String>,
    /// Provider usage JSON when the provider emits usage metadata.
    pub usage_json: Option<String>,
}

/// A declarative Goose provider constructed from provider JSON.
#[derive(uniffi::Object)]
pub struct DeclarativeProvider {
    provider: Box<dyn Provider>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[uniffi::export]
impl DeclarativeProvider {
    /// Construct a declarative provider using the process environment to resolve
    /// configured API key environment variables.
    #[uniffi::constructor]
    pub fn from_json(json: String) -> Result<Arc<Self>, GooseError> {
        let provider = goose_providers::declarative::from_json(&json, None, EnvKeyResolver {})?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| GooseError::Generic(error.to_string()))?;

        Ok(Arc::new(Self {
            provider,
            runtime: Arc::new(runtime),
        }))
    }

    pub fn name(&self) -> String {
        self.provider.get_name().to_string()
    }

    /// Start a streaming completion request. Tools are not yet exposed over the
    /// uniffi boundary, so this calls providers with an empty tool list.
    pub fn stream(
        &self,
        model: ProviderModelConfig,
        system: String,
        messages: Vec<ProviderMessage>,
    ) -> Result<Arc<DeclarativeProviderStream>, GooseError> {
        let model = model.to_goose_model_config()?;
        let messages = messages
            .iter()
            .map(ProviderMessage::to_goose_message)
            .collect::<Vec<_>>();
        let stream =
            self.runtime
                .block_on(self.provider.stream(&model, &system, &messages, &[]))?;

        Ok(Arc::new(DeclarativeProviderStream {
            stream: Mutex::new(stream),
            runtime: Arc::clone(&self.runtime),
        }))
    }
}

/// A blocking iterator over provider stream chunks.
#[derive(uniffi::Object)]
pub struct DeclarativeProviderStream {
    stream: Mutex<MessageStream>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[uniffi::export]
impl DeclarativeProviderStream {
    /// Return the next stream chunk, or `None` when the stream is exhausted.
    pub fn next(&self) -> Result<Option<ProviderStreamChunk>, GooseError> {
        let mut stream = self
            .stream
            .lock()
            .map_err(|_| GooseError::Generic("provider stream lock poisoned".to_string()))?;

        let Some((message, usage)) = self.runtime.block_on(stream.next()).transpose()? else {
            return Ok(None);
        };

        let text = message.as_ref().map(Message::as_concat_text);
        let message_json = message.as_ref().map(serde_json::to_string).transpose()?;
        let usage_json = usage.as_ref().map(serde_json::to_string).transpose()?;

        Ok(Some(ProviderStreamChunk {
            text,
            message_json,
            usage_json,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_config_rejects_invalid_request_params_json() {
        let config = ProviderModelConfig {
            model_name: "test".to_string(),
            context_limit: None,
            temperature: None,
            max_tokens: None,
            toolshim: false,
            toolshim_model: None,
            request_params_json: Some("not json".to_string()),
            reasoning: None,
        };

        assert!(config.to_goose_model_config().is_err());
    }

    #[test]
    fn provider_message_converts_user_text() {
        let message = ProviderMessage {
            role: MessageRole::User,
            text: "what is the capital of France?".to_string(),
        }
        .to_goose_message();

        assert_eq!(message.as_concat_text(), "what is the capital of France?");
    }
}
