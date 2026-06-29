use super::api_client::ApiClient;
use super::base::{ConfigKey, MessageStream, Provider, ProviderMetadata};
use super::openai_compatible::handle_status;
use super::retry::{ProviderRetry, RetryConfig};
use crate::base::ProviderDescriptor;
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::formats::ollama::{create_request, response_to_streaming_message_ollama};
use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::request_log::{start_log, LoggerHandleExt, RequestLogHandle};
use anyhow::{Error, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::Response;
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

pub const OLLAMA_PROVIDER_NAME: &str = "ollama";
pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_TIMEOUT: u64 = 600;
pub const OLLAMA_DEFAULT_PORT: u16 = 11434;
pub const OLLAMA_DEFAULT_MODEL: &str = "qwen3";
pub const OLLAMA_KNOWN_MODELS: &[&str] = &[
    OLLAMA_DEFAULT_MODEL,
    "qwen3-vl",
    "qwen3-coder:30b",
    "qwen3-coder:480b-cloud",
];
pub const OLLAMA_DOC_URL: &str = "https://ollama.com/library";

// Ollama-specific retry config: large models can take 30-120s to load into memory,
// during which Ollama returns 500 errors. Use more retries with gradual backoff
// to wait for the model to become ready.
const OLLAMA_MAX_RETRIES: usize = 10;
const OLLAMA_INITIAL_RETRY_INTERVAL_MS: u64 = 2000;
const OLLAMA_BACKOFF_MULTIPLIER: f64 = 1.5;
const OLLAMA_MAX_RETRY_INTERVAL_MS: u64 = 15_000;

/// Provider settings resolved from `config::Config` at construction time.
///
/// All values that the Ollama provider reads out of the global config are
/// resolved once, up front, and carried here. This keeps the streaming hot
/// path free of config lookups and makes the provider's config dependencies
/// explicit at the construction boundary.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OllamaOptions {
    /// Explicit context window override from `GOOSE_INPUT_LIMIT`.
    /// `None` when unset, zero, or invalid; the model's context limit is then
    /// used as the fallback.
    pub input_limit: Option<usize>,
    /// Whether to keep `stream_options` in the request (`OLLAMA_STREAM_USAGE`,
    /// default `true`).
    pub stream_usage: bool,
    /// Per-chunk stream timeout in seconds, resolved from
    /// `OLLAMA_STREAM_TIMEOUT` > `GOOSE_STREAM_TIMEOUT` > `OLLAMA_TIMEOUT` >
    /// default (120s).
    pub chunk_timeout_secs: u64,
}

impl Default for OllamaOptions {
    fn default() -> Self {
        Self {
            input_limit: None,
            stream_usage: true,
            chunk_timeout_secs: OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS,
        }
    }
}

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    api_client: ApiClient,
    name: String,
    skip_canonical_filtering: bool,
    options: OllamaOptions,
}

impl OllamaProvider {
    pub fn new(
        api_client: ApiClient,
        name: String,
        skip_canonical_filtering: bool,
        options: OllamaOptions,
    ) -> Self {
        Self {
            api_client,
            name,
            skip_canonical_filtering,
            options,
        }
    }
}

fn resolve_ollama_num_ctx(options: &OllamaOptions, model_config: &ModelConfig) -> Option<usize> {
    options.input_limit.or(model_config.context_limit)
}

fn apply_ollama_options(payload: &mut Value, options: &OllamaOptions, model_config: &ModelConfig) {
    if let Some(obj) = payload.as_object_mut() {
        // Gate stream_options behind OLLAMA_STREAM_USAGE (default: true).
        // Older Ollama builds that don't support stream_options may stall before
        // emitting any SSE data, blocking until the client timeout (600s).
        // with_line_timeout() only protects after the first line arrives, so
        // users on older builds should set OLLAMA_STREAM_USAGE=false.
        if !options.stream_usage {
            obj.remove("stream_options");
        }

        // Convert max_completion_tokens / max_tokens to Ollama's options.num_predict.
        // Reasoning models emit max_completion_tokens; non-reasoning models emit max_tokens.
        let max_tokens = obj
            .remove("max_completion_tokens")
            .or_else(|| obj.remove("max_tokens"));
        if let Some(max_tokens) = max_tokens {
            let options_value = obj.entry("options").or_insert_with(|| json!({}));
            if let Some(options_obj) = options_value.as_object_mut() {
                options_obj.entry("num_predict").or_insert(max_tokens);
            }
        }

        // Apply num_ctx from context limit settings.
        if let Some(limit) = resolve_ollama_num_ctx(options, model_config) {
            let options_value = obj.entry("options").or_insert_with(|| json!({}));
            if let Some(options_obj) = options_value.as_object_mut() {
                options_obj.insert("num_ctx".to_string(), json!(limit));
            }
        }
    }
}

impl ProviderDescriptor for OllamaProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OLLAMA_PROVIDER_NAME,
            "Ollama",
            "Local open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.to_vec(),
            OLLAMA_DOC_URL,
            vec![
                ConfigKey::new("OLLAMA_HOST", true, false, Some(OLLAMA_HOST), true),
                ConfigKey::new(
                    "OLLAMA_TIMEOUT",
                    false,
                    false,
                    Some(&(OLLAMA_TIMEOUT.to_string())),
                    false,
                ),
            ],
        )
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    fn retry_config(&self) -> RetryConfig {
        RetryConfig::new(
            OLLAMA_MAX_RETRIES,
            OLLAMA_INITIAL_RETRY_INTERVAL_MS,
            OLLAMA_BACKOFF_MULTIPLIER,
            OLLAMA_MAX_RETRY_INTERVAL_MS,
        )
        .transient_only()
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            true,
        )?;
        apply_ollama_options(&mut payload, &self.options, model_config);
        let mut log = start_log(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post("v1/chat/completions", &payload)
                    .await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;
        stream_ollama(response, self.options.chunk_timeout_secs, log)
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request("api/tags")
            .response_get()
            .await
            .map_err(|e| ProviderError::RequestFailed(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProviderError::RequestFailed(format!(
                "Failed to fetch models: HTTP {}",
                response.status()
            )));
        }

        let json_response = response.json::<Value>().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse response: {}", e))
        })?;

        let models = json_response
            .get("models")
            .and_then(|m| m.as_array())
            .ok_or_else(|| {
                ProviderError::RequestFailed("No models array in response".to_string())
            })?;

        let mut model_names: Vec<String> = models
            .iter()
            .filter_map(|model| model.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect();

        model_names.sort();

        Ok(model_names)
    }
}

/// Default per-chunk timeout for Ollama streaming responses (seconds).
/// Configurable via OLLAMA_STREAM_TIMEOUT, GOOSE_STREAM_TIMEOUT, or falls back
/// to OLLAMA_TIMEOUT. Set high to accommodate slower models (CPU inference,
/// large parameter counts, complex reasoning).
pub const OLLAMA_DEFAULT_CHUNK_TIMEOUT_SECS: u64 = 120;

/// Wraps a line stream with a per-item timeout at the raw SSE level.
/// This detects dead connections without false-positive stalls during long
/// tool-call generations where response_to_streaming_message_ollama buffers.
fn with_line_timeout(
    stream: impl futures::Stream<Item = anyhow::Result<String>> + Unpin + Send + 'static,
    timeout_secs: u64,
) -> std::pin::Pin<Box<dyn futures::Stream<Item = anyhow::Result<String>> + Send>> {
    let timeout = Duration::from_secs(timeout_secs);
    Box::pin(try_stream! {
        let mut stream = stream;

        // Allow time-to-first-token to be governed by the request timeout.
        // Only enforce per-chunk timeout after first SSE line arrives.
        match stream.next().await {
            Some(first_item) => yield first_item?,
            None => return,
        }
        loop {
            match tokio::time::timeout(timeout, stream.next()).await {
                Ok(Some(item)) => yield item?,
                Ok(None) => break,
                Err(_) => {
                    Err::<(), anyhow::Error>(anyhow::anyhow!(
                        "Ollama stream stalled: no data received for {}s. \
                         This may indicate the model is overwhelmed by the request payload. \
                         Try a smaller model, reduce the number of tools, or increase the \
                         timeout via OLLAMA_STREAM_TIMEOUT, GOOSE_STREAM_TIMEOUT, or \
                         OLLAMA_TIMEOUT in your config.",
                        timeout_secs
                    ))?;
                }
            }
        }
    })
}

/// Ollama-specific streaming handler with XML tool call fallback.
/// Uses the Ollama format module which buffers text when XML tool calls are detected,
/// preventing duplicate content from being emitted to the UI.
/// Timeout is applied at the raw SSE line level via with_line_timeout so that
/// buffering inside response_to_streaming_message_ollama does not cause false stalls.
fn stream_ollama(
    response: Response,
    chunk_timeout: u64,
    mut log: Option<Box<dyn RequestLogHandle>>,
) -> Result<MessageStream, ProviderError> {
    let stream = response.bytes_stream().map_err(std::io::Error::other);

    Ok(Box::pin(try_stream! {
        let stream_reader = StreamReader::new(stream);
        let framed = FramedRead::new(stream_reader, LinesCodec::new())
            .map_err(Error::from);

        let timed_lines = with_line_timeout(framed, chunk_timeout);
        let message_stream = response_to_streaming_message_ollama(timed_lines);
        pin!(message_stream);

        while let Some(message) = message_stream.next().await {
            let (message, usage) = message.map_err(ProviderError::from_stream_error)?;
            log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
            yield (message, usage);
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_ollama_options_uses_input_limit() {
        let options = OllamaOptions {
            input_limit: Some(8192),
            ..Default::default()
        };
        let model_config = ModelConfig::new("qwen3").with_context_limit(Some(16_000));
        let mut payload = json!({});
        apply_ollama_options(&mut payload, &options, &model_config);
        assert_eq!(payload["options"]["num_ctx"], 8192);
    }

    #[test]
    fn test_apply_ollama_options_falls_back_to_context_limit() {
        let options = OllamaOptions::default();
        let model_config = ModelConfig::new("qwen3").with_context_limit(Some(12_000));
        let mut payload = json!({});
        apply_ollama_options(&mut payload, &options, &model_config);
        assert_eq!(payload["options"]["num_ctx"], 12_000);
    }

    #[test]
    fn test_apply_ollama_options_skips_when_no_limit() {
        let options = OllamaOptions::default();
        let mut model_config = ModelConfig::new("qwen3");
        model_config.context_limit = None;
        let mut payload = json!({});
        apply_ollama_options(&mut payload, &options, &model_config);
        assert!(payload.get("options").is_none());
    }

    #[test]
    fn test_raw_create_request_contains_unsupported_ollama_fields() {
        use crate::formats::ollama::create_request;

        let model_config = ModelConfig::new("llama3.1").with_max_tokens(Some(4096));
        let messages = vec![crate::conversation::message::Message::user().with_text("hi")];

        let payload = create_request(
            &model_config,
            "You are a helpful assistant.",
            &messages,
            &[],
            &ImageFormat::OpenAi,
            true,
        )
        .unwrap();

        assert!(
            payload.get("stream_options").is_some(),
            "create_request should produce stream_options for usage tracking"
        );
        assert!(
            payload.get("max_tokens").is_some(),
            "create_request should produce max_tokens (unsupported by Ollama)"
        );
    }

    #[test]
    fn test_apply_ollama_options_preserves_stream_options_by_default() {
        use crate::formats::ollama::create_request;

        let options = OllamaOptions::default();
        let model_config = ModelConfig::new("llama3.1").with_max_tokens(Some(4096));
        let messages = vec![crate::conversation::message::Message::user().with_text("hi")];

        let mut payload = create_request(
            &model_config,
            "You are a helpful assistant.",
            &messages,
            &[],
            &ImageFormat::OpenAi,
            true,
        )
        .unwrap();

        apply_ollama_options(&mut payload, &options, &model_config);

        assert!(
            payload.get("stream_options").is_some(),
            "stream_options should be preserved by default for usage tracking"
        );
        assert!(
            payload.get("max_tokens").is_none(),
            "max_tokens should be removed for Ollama"
        );
        assert!(
            payload.get("max_completion_tokens").is_none(),
            "max_completion_tokens should be removed for Ollama"
        );
        assert_eq!(
            payload["options"]["num_predict"], 4096,
            "max_tokens should be moved to options.num_predict"
        );
        assert_eq!(payload["stream"], true, "stream field should be preserved");
    }

    #[test]
    fn test_apply_ollama_options_strips_stream_options_when_disabled() {
        use crate::formats::ollama::create_request;

        let options = OllamaOptions {
            input_limit: None,
            stream_usage: false,
            chunk_timeout_secs: 120,
        };
        let model_config = ModelConfig::new("llama3.1").with_max_tokens(Some(4096));
        let messages = vec![crate::conversation::message::Message::user().with_text("hi")];

        let mut payload = create_request(
            &model_config,
            "You are a helpful assistant.",
            &messages,
            &[],
            &ImageFormat::OpenAi,
            true,
        )
        .unwrap();

        apply_ollama_options(&mut payload, &options, &model_config);

        assert!(
            payload.get("stream_options").is_none(),
            "stream_options should be removed when OLLAMA_STREAM_USAGE=false"
        );
    }
}
