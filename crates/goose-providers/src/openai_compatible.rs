use crate::conversation::token_usage::{CostSource, ProviderUsage};
use crate::images::ImageFormat;
use anyhow::Error;
use async_stream::try_stream;
use bytes::Bytes;
use futures::Stream;
use futures::TryStreamExt;
use reqwest::Response;
#[cfg(test)]
use reqwest::StatusCode;
use serde_json::Value;
use std::pin::Pin;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::api_client::ApiClient;
use super::base::{stream_from_single_message, MessageStream, Provider};
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::errors::ProviderError;
use crate::formats::openai::{
    create_request, create_request_for_model_with_options, get_cost, get_usage,
    response_to_message, response_to_streaming_message, OpenAiFormatOptions,
};
use crate::formats::openai_responses::responses_api_to_streaming_message;
use crate::model::ModelConfig;
use crate::request_log::{start_log, LoggerHandleExt, RequestLogHandle};
use rmcp::model::Tool;

pub struct OpenAiCompatibleProvider {
    name: String,
    /// Client targeted at the base URL (e.g. `https://api.x.ai/v1`)
    api_client: ApiClient,
    /// Path prefix prepended to `chat/completions` (e.g. `"deployments/{name}/"` for Azure).
    completions_prefix: String,
    supports_streaming: bool,
}

impl OpenAiCompatibleProvider {
    pub fn new(name: String, api_client: ApiClient, completions_prefix: String) -> Self {
        Self {
            name,
            api_client,
            completions_prefix,
            supports_streaming: true,
        }
    }

    pub fn with_supports_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    #[allow(clippy::too_many_arguments)]
    fn build_request_for_model(
        &self,
        model_config: &ModelConfig,
        wire_model: &str,
        capability_model: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        for_streaming: bool,
    ) -> Result<Value, ProviderError> {
        create_request_for_model_with_options(
            model_config,
            wire_model,
            capability_model,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            for_streaming,
            OpenAiFormatOptions {
                preserve_thinking_context: true,
                ..Default::default()
            },
        )
        .map_err(|e| ProviderError::RequestFailed(format!("Failed to create request: {}", e)))
    }

    pub async fn stream_for_model(
        &self,
        model_config: &ModelConfig,
        wire_model: &str,
        capability_model: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = self.build_request_for_model(
            model_config,
            wire_model,
            capability_model,
            system,
            messages,
            tools,
            self.supports_streaming,
        )?;
        self.stream_payload(model_config, payload).await
    }

    async fn stream_payload(
        &self,
        model_config: &ModelConfig,
        payload: Value,
    ) -> Result<MessageStream, ProviderError> {
        let mut log = start_log(model_config, &payload)?;
        let path = format!("{}chat/completions", self.completions_prefix);
        // For streaming, pre-read the first body chunk inside the retry scope so a
        // 200 OK whose connection dies before any body bytes still retries.
        let (response, first_chunk) = self
            .with_retry(|| async {
                let mut response = handle_status(
                    self.api_client
                        .request(&path)
                        .model_headers(model_config)?
                        .streaming(self.supports_streaming)
                        .response_post(&payload)
                        .await?,
                )
                .await?;
                let first_chunk = if self.supports_streaming {
                    Some(first_body_chunk(&mut response).await?)
                } else {
                    None
                };
                Ok((response, first_chunk))
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;
        if self.supports_streaming {
            stream_openai_compat_with_prefix(response, first_chunk, log)
        } else {
            let json = response.json().await.map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to parse JSON: {}", e))
            })?;
            let message = response_to_message(&json).map_err(|e| {
                ProviderError::RequestFailed(format!("Failed to parse message: {}", e))
            })?;
            let usage_json = json.get("usage").unwrap_or(&Value::Null);
            let usage_data = get_usage(usage_json);
            let mut usage = ProviderUsage::new(model_config.model_name.clone(), usage_data);
            if let Some(cost) = get_cost(usage_json) {
                usage = usage.with_cost(cost, CostSource::ProviderReported);
            }
            log.write(
                &serde_json::to_value(&message).unwrap_or_default(),
                Some(&usage.usage),
            )?;
            Ok(stream_from_single_message(message, usage))
        }
    }

    fn build_request(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
        for_streaming: bool,
    ) -> Result<Value, ProviderError> {
        create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            for_streaming,
        )
        .map_err(|e| ProviderError::RequestFailed(format!("Failed to create request: {}", e)))
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiCompatibleProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn refresh_credentials(&self) -> Result<(), ProviderError> {
        self.api_client
            .refresh_credentials()
            .await
            .map_err(|error| ProviderError::Authentication(error.to_string()))
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .response_get("models")
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;
        let json = handle_response_openai_compat(response).await?;

        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }

        let arr = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::RequestFailed("Missing 'data' array in models response".to_string())
        })?;
        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let payload = self.build_request(
            model_config,
            system,
            messages,
            tools,
            self.supports_streaming,
        )?;
        self.stream_payload(model_config, payload).await
    }
}

// Re-exported from the dedicated `http_status` module — these helpers are
// format-agnostic and used across all provider families.
pub use super::http_status::{
    handle_response, handle_status, map_http_error_to_provider_error, sanitize_url,
};

// Legacy alias kept for callers that haven't migrated their import path yet.
pub use super::http_status::handle_response as handle_response_openai_compat;

/// Reads the first chunk of a streaming response body.
///
/// A `200 OK` whose connection dies before delivering any body bytes only
/// fails here, while the caller is still inside its `with_retry` scope —
/// afterwards the failure would surface as a mid-stream item error where
/// retry is unreachable. A clean EOF is also an error: an empty body is
/// never a valid answer to a streaming request, and `Ok(None)` here would
/// otherwise complete the retry scope with a stream that yields nothing.
pub async fn first_body_chunk(response: &mut Response) -> Result<Bytes, ProviderError> {
    response
        .chunk()
        .await
        .map_err(ProviderError::stream_decode_error)?
        .ok_or_else(|| {
            ProviderError::stream_decode_error("stream ended before the first body chunk")
        })
}

fn body_stream_with_prefix(
    response: Response,
    first_chunk: Option<Bytes>,
) -> Pin<Box<dyn Stream<Item = std::io::Result<Bytes>> + Send>> {
    Box::pin(async_stream::stream! {
        if let Some(chunk) = first_chunk {
            yield Ok(chunk);
        }
        let body = response.bytes_stream().map_err(std::io::Error::other);
        tokio::pin!(body);
        while let Some(item) = body.next().await {
            yield item;
        }
    })
}

pub fn stream_openai_compat(
    response: Response,
    log: Option<Box<dyn RequestLogHandle>>,
) -> Result<MessageStream, ProviderError> {
    stream_openai_compat_with_prefix(response, None, log)
}

pub fn stream_openai_compat_with_prefix(
    response: Response,
    first_chunk: Option<Bytes>,
    mut log: Option<Box<dyn RequestLogHandle>>,
) -> Result<MessageStream, ProviderError> {
    let stream = body_stream_with_prefix(response, first_chunk);

    Ok(Box::pin(try_stream! {
        let stream_reader = StreamReader::new(stream);
        let framed = FramedRead::new(stream_reader, LinesCodec::new())
            .map_err(Error::from);

        let message_stream = response_to_streaming_message(framed);
        pin!(message_stream);
        while let Some(message) = message_stream.next().await {
            let (message, usage) = message.map_err(|e|
                e.downcast::<ProviderError>()
                    .unwrap_or_else(ProviderError::stream_decode_error)
            )?;
            log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
            yield (message, usage);
        }
    }))
}

pub fn stream_responses_compat(
    response: Response,
    log: Option<Box<dyn RequestLogHandle>>,
) -> Result<MessageStream, ProviderError> {
    stream_responses_compat_with_prefix(response, None, log)
}

pub fn stream_responses_compat_with_prefix(
    response: Response,
    first_chunk: Option<Bytes>,
    mut log: Option<Box<dyn RequestLogHandle>>,
) -> Result<MessageStream, ProviderError> {
    let stream = body_stream_with_prefix(response, first_chunk);

    Ok(Box::pin(try_stream! {
        let stream_reader = StreamReader::new(stream);
        let framed = FramedRead::new(stream_reader, LinesCodec::new())
            .map_err(Error::from);

        let message_stream = responses_api_to_streaming_message(framed);
        pin!(message_stream);
        while let Some(message) = message_stream.next().await {
            let (message, usage) = message.map_err(|e|
                e.downcast::<ProviderError>()
                    .unwrap_or_else(ProviderError::stream_decode_error)
            )?;
            log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
            yield (message, usage);
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ModelConfig;
    use serde_json::json;
    use test_case::test_case;

    #[test_case(
        StatusCode::PAYMENT_REQUIRED,
        Some(json!({"error": {"message": "Insufficient credits to complete this request"}})),
        "CreditsExhausted"
        ; "402 with payload"
    )]
    #[test_case(
        StatusCode::PAYMENT_REQUIRED,
        None,
        "CreditsExhausted"
        ; "402 without payload"
    )]
    #[test_case(
        StatusCode::TOO_MANY_REQUESTS,
        Some(json!({"error": {"message": "Rate limit exceeded"}})),
        "RateLimitExceeded"
        ; "429 rate limit"
    )]
    #[test_case(
        StatusCode::UNAUTHORIZED,
        None,
        "Authentication"
        ; "401 unauthorized"
    )]
    #[test_case(
        StatusCode::BAD_REQUEST,
        Some(json!({"error": {"message": "This request exceeds the maximum context length"}})),
        "ContextLengthExceeded"
        ; "400 context length"
    )]
    #[test_case(
        StatusCode::INTERNAL_SERVER_ERROR,
        None,
        "ServerError"
        ; "500 server error"
    )]
    #[test_case(
        StatusCode::NOT_FOUND,
        None,
        "RequestFailed"
        ; "404 not found"
    )]
    #[test_case(
        StatusCode::NOT_FOUND,
        Some(json!({"error": {"message": "model not available"}})),
        "RequestFailed"
        ; "404 with error payload"
    )]
    fn http_status_maps_to_expected_error(
        status: StatusCode,
        payload: Option<Value>,
        expected_variant: &str,
    ) {
        let err = map_http_error_to_provider_error(status, payload, "http://test/endpoint");
        let actual = err.telemetry_type();
        let expected_telemetry = match expected_variant {
            "CreditsExhausted" => "credits_exhausted",
            "RateLimitExceeded" => "rate_limit",
            "Authentication" => "auth",
            "ContextLengthExceeded" => "context_length",
            "ServerError" => "server",
            "RequestFailed" => "request",
            other => panic!("Unknown variant: {other}"),
        };
        assert_eq!(
            actual, expected_telemetry,
            "Expected {expected_variant}, got error: {err:?}"
        );
    }

    #[test]
    fn build_request_respects_non_streaming_mode() {
        let provider = OpenAiCompatibleProvider::new(
            "test".to_string(),
            ApiClient::new_with_tls(
                "http://localhost".to_string(),
                super::super::api_client::AuthMethod::NoAuth,
                None,
            )
            .unwrap(),
            String::new(),
        )
        .with_supports_streaming(false);

        let model = ModelConfig::new("test-model");
        let payload = provider
            .build_request(&model, "", &[], &[], provider.supports_streaming)
            .unwrap();

        assert_eq!(payload.get("stream"), None);
        assert_eq!(payload.get("stream_options"), None);
    }
}
