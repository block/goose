use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata, ProviderUsage, Usage};
use super::embedding::{EmbeddingCapable, EmbeddingRequest, EmbeddingResponse};
use super::errors::ProviderError;
use super::formats::openai::{create_request, get_usage, response_to_message};
use super::utils::{
    emit_debug_trace, get_model, handle_response_openai_compat, handle_status_openai_compat,
    ImageFormat,
};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::conversation::message::Message;

use crate::model::ModelConfig;
use crate::providers::base::MessageStream;
use crate::providers::formats::openai::response_to_streaming_message;
use rmcp::model::Tool;

pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";
pub const OPEN_AI_DEFAULT_FAST_MODEL: &str = "gpt-4o-mini";
pub const OPEN_AI_KNOWN_MODELS: &[(&str, usize)] = &[
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-4.1", 128_000),
    ("gpt-4.1-mini", 128_000),
    ("o1", 200_000),
    ("o3", 200_000),
    ("gpt-3.5-turbo", 16_385),
    ("gpt-4-turbo", 128_000),
    ("o4-mini", 128_000),
];

pub const OPEN_AI_DOC_URL: &str = "https://platform.openai.com/docs/models";

#[derive(Debug, serde::Serialize)]
pub struct OpenAiProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
    supports_streaming: bool,
}

impl OpenAiProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(OPEN_AI_DEFAULT_FAST_MODEL.to_string());

        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENAI_API_KEY")?;
        let host: String = config
            .get_param("OPENAI_HOST")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        let base_path: String = config
            .get_param("OPENAI_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let organization: Option<String> = config.get_param("OPENAI_ORGANIZATION").ok();
        let project: Option<String> = config.get_param("OPENAI_PROJECT").ok();
        let custom_headers: Option<HashMap<String, String>> = config
            .get_secret("OPENAI_CUSTOM_HEADERS")
            .or_else(|_| config.get_param("OPENAI_CUSTOM_HEADERS"))
            .ok()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config.get_param("OPENAI_TIMEOUT").unwrap_or(600);

        let auth = AuthMethod::BearerToken(api_key);
        let mut api_client =
            ApiClient::with_timeout(host, auth, std::time::Duration::from_secs(timeout_secs))?;

        if let Some(org) = &organization {
            api_client = api_client.with_header("OpenAI-Organization", org)?;
        }

        if let Some(project) = &project {
            api_client = api_client.with_header("OpenAI-Project", project)?;
        }

        if let Some(headers) = &custom_headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        Ok(Self {
            api_client,
            base_path,
            organization,
            project,
            model,
            custom_headers,
            supports_streaming: true,
        })
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let global_config = crate::config::Config::global();
        let api_key: String = global_config
            .get_secret(&config.api_key_env)
            .map_err(|_e| anyhow::anyhow!("Missing API key: {}", config.api_key_env))?;

        let url = url::Url::parse(&config.base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base URL '{}': {}", config.base_url, e))?;

        let host = if let Some(port) = url.port() {
            format!(
                "{}://{}:{}",
                url.scheme(),
                url.host_str().unwrap_or(""),
                port
            )
        } else {
            format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""))
        };
        let base_path = url.path().trim_start_matches('/').to_string();
        let base_path = if base_path.is_empty() {
            "v1/chat/completions".to_string()
        } else {
            base_path
        };

        let timeout_secs = config.timeout_seconds.unwrap_or(600);
        let auth = AuthMethod::BearerToken(api_key);
        let mut api_client =
            ApiClient::with_timeout(host, auth, std::time::Duration::from_secs(timeout_secs))?;

        // Add custom headers if present
        if let Some(headers) = &config.headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        Ok(Self {
            api_client,
            base_path,
            organization: None,
            project: None,
            model,
            custom_headers: config.headers,
            supports_streaming: config.supports_streaming.unwrap_or(true),
        })
    }

    async fn post(&self, payload: &Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(&self.base_path, payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn metadata() -> ProviderMetadata {
        let models = OPEN_AI_KNOWN_MODELS
            .iter()
            .map(|(name, limit)| ModelInfo::new(*name, *limit))
            .collect();
        ProviderMetadata::with_models(
            "openai",
            "OpenAI",
            "GPT-4 and other OpenAI models, including OpenAI compatible ones",
            OPEN_AI_DEFAULT_MODEL,
            models,
            OPEN_AI_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_API_KEY", true, true, None),
                ConfigKey::new("OPENAI_HOST", true, false, Some("https://api.openai.com")),
                ConfigKey::new("OPENAI_BASE_PATH", true, false, Some("v1/chat/completions")),
                ConfigKey::new("OPENAI_ORGANIZATION", false, false, None),
                ConfigKey::new("OPENAI_PROJECT", false, false, None),
                ConfigKey::new("OPENAI_CUSTOM_HEADERS", false, true, None),
                ConfigKey::new("OPENAI_TIMEOUT", false, false, Some("600")),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        // Create span following OpenTelemetry GenAI semantic conventions
        // Span name format: "{operation} {model}" per https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/
        let span = tracing::info_span!(
            "chat",
            otel.name = format!("chat {}", model_config.model_name),
            gen_ai.request.model = %model_config.model_name,
            gen_ai.system = "openai",
            gen_ai.operation.name = "chat"
        );

        // Diagnostic: Check if we're creating a span without a parent context
        let current_span = tracing::Span::current();
        if current_span.id().is_none() {
            tracing::warn!(
                "LLM call without parent span context - this span will be unrooted!\nCall site: {}\nBacktrace:\n{:?}",
                std::panic::Location::caller(),
                std::backtrace::Backtrace::force_capture()
            );
        }

        let _enter = span.enter();

        let payload = create_request(model_config, system, messages, tools, &ImageFormat::OpenAi)?;
        let json_response = self.post(&payload).await?;

        let message = response_to_message(&json_response)?;
        let usage = json_response
            .get("usage")
            .map(get_usage)
            .unwrap_or_else(|| {
                tracing::debug!("Failed to get usage data");
                Usage::default()
            });
        let model = get_model(&json_response);
        emit_debug_trace(&self.model, &payload, &json_response, &usage);
        Ok((message, ProviderUsage::new(model, usage)))
    }

    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let models_path = self.base_path.replace("v1/chat/completions", "v1/models");
        let response = self.api_client.response_get(&models_path).await?;
        let json = handle_response_openai_compat(response).await?;
        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(ProviderError::Authentication(msg.to_string()));
        }

        let data = json.get("data").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::UsageError("Missing data field in JSON response".into())
        })?;
        let mut models: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(Some(models))
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        EmbeddingCapable::create_embeddings(self, texts)
            .await
            .map_err(|e| ProviderError::ExecutionError(e.to_string()))
    }

    fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    async fn stream(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        // Create span following OpenTelemetry GenAI semantic conventions
        // Span name format: "{operation} {model}" per https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/
        let span = tracing::info_span!(
            "chat",
            otel.name = format!("chat {}", self.model.model_name),
            gen_ai.request.model = %self.model.model_name,
            gen_ai.system = "openai",
            gen_ai.operation.name = "chat"
        );

        // Diagnostic: Check if we're creating a span without a parent context
        let current_span = tracing::Span::current();
        if current_span.id().is_none() {
            tracing::warn!(
                "LLM call without parent span context - this span will be unrooted!\nCall site: {}\nBacktrace:\n{:?}",
                std::panic::Location::caller(),
                std::backtrace::Backtrace::force_capture()
            );
        }

        let mut payload =
            create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;
        payload["stream"] = serde_json::Value::Bool(true);
        payload["stream_options"] = json!({
            "include_usage": true,
        });

        // Enter the span BEFORE making the HTTP request so trace context propagates correctly
        let response = {
            let _enter = span.enter();
            let response = self
                .api_client
                .response_post(&self.base_path, &payload)
                .await?;
            handle_status_openai_compat(response).await?
            // Span exits here after HTTP request completes
        };

        let stream = response.bytes_stream().map_err(io::Error::other);

        let model_config = self.model.clone();

        // Capture span to keep it alive during stream consumption
        // Enter the span for each item yielded from the stream
        Ok(Box::pin(try_stream! {
            let _span_guard = span.enter();

            let stream_reader = StreamReader::new(stream);
            let framed = FramedRead::new(stream_reader, LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = message_stream.next().await {
                let (message, usage) = message.map_err(|e| ProviderError::RequestFailed(format!("Stream decode error: {}", e)))?;
                emit_debug_trace(&model_config, &payload, &message, &usage.as_ref().map(|f| f.usage).unwrap_or_default());
                yield (message, usage);
            }
            // Span ends when _span_guard is dropped after stream is fully consumed
        }))
    }
}

fn parse_custom_headers(s: String) -> HashMap<String, String> {
    s.split(',')
        .filter_map(|header| {
            let mut parts = header.splitn(2, '=');
            let key = parts.next().map(|s| s.trim().to_string())?;
            let value = parts.next().map(|s| s.trim().to_string())?;
            Some((key, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::{
        export::trace::{SpanData, SpanExporter},
        trace::{RandomIdGenerator, Sampler, TracerProvider},
        Resource,
    };
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    /// Custom span exporter that captures spans for testing
    #[derive(Clone, Debug)]
    struct TestSpanExporter {
        spans: Arc<Mutex<Vec<SpanData>>>,
    }

    impl TestSpanExporter {
        fn new(spans: Arc<Mutex<Vec<SpanData>>>) -> Self {
            Self { spans }
        }
    }

    impl SpanExporter for TestSpanExporter {
        fn export(
            &mut self,
            batch: Vec<SpanData>,
        ) -> futures::future::BoxFuture<'static, opentelemetry_sdk::export::trace::ExportResult>
        {
            let spans = self.spans.clone();
            Box::pin(async move {
                spans.lock().unwrap().extend(batch);
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn test_stream_span_lifecycle() {
        // This test verifies that streaming operations keep the span active
        // during the entire stream consumption, not just during the HTTP request.
        // This prevents the bug where client spans finish before server spans.

        use async_stream::stream;
        use tokio_stream::StreamExt;

        // Set up OpenTelemetry with test exporter
        let exported_spans = Arc::new(Mutex::new(Vec::new()));
        let test_exporter = TestSpanExporter::new(exported_spans.clone());

        let tracer_provider = TracerProvider::builder()
            .with_simple_exporter(test_exporter)
            .with_resource(Resource::new(vec![opentelemetry::KeyValue::new(
                "service.name",
                "test",
            )]))
            .with_id_generator(RandomIdGenerator::default())
            .with_sampler(Sampler::AlwaysOn)
            .build();

        let _ = opentelemetry::global::set_tracer_provider(tracer_provider.clone());

        let tracer = tracer_provider.tracer("test");
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry_layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        // Create a parent span to simulate agent.reply
        let parent_span = tracing::info_span!("parent");
        let _parent_enter = parent_span.enter();

        // Simulate a streaming operation with artificial delay
        let span = tracing::info_span!("simulated_stream");

        let mock_stream = Box::pin(stream! {
            let _span_guard = span.enter();

            // Simulate stream items with delays
            for i in 0..3 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                yield i;
            }
            // Span should stay active until here
        });

        // Record when we start consuming the stream
        let start = std::time::Instant::now();

        // Consume the stream (this simulates what agent.rs does)
        let mut count = 0;
        tokio::pin!(mock_stream);
        while let Some(_) = mock_stream.next().await {
            count += 1;
        }

        let duration = start.elapsed();

        drop(_parent_enter);
        drop(parent_span);

        // Force flush
        tracer_provider.force_flush();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify spans were exported
        let spans = exported_spans.lock().unwrap();
        assert_eq!(count, 3, "Should have consumed 3 stream items");

        // Find the simulated_stream span
        let stream_span = spans
            .iter()
            .find(|s| s.name == "simulated_stream")
            .expect("Should have simulated_stream span");

        // Calculate actual span duration
        let span_duration = stream_span
            .end_time
            .duration_since(stream_span.start_time)
            .expect("End time should be after start time");

        // Key assertion: Span duration should be >= stream consumption time
        // This proves the span stayed active during stream consumption
        assert!(
            span_duration.as_millis() >= 150, // 3 items × 50ms each
            "Span duration ({:?}) should be at least 150ms (stream consumption time), got {:?}ms",
            span_duration,
            span_duration.as_millis()
        );

        // Also verify span duration is roughly equal to actual stream consumption
        // (with some tolerance for test timing variability)
        assert!(
            span_duration.as_millis() <= duration.as_millis() + 50,
            "Span duration should not exceed actual consumption time by more than 50ms"
        );

        println!(
            "✓ Span lifecycle correct: span duration {:?} ≈ stream consumption {:?}",
            span_duration, duration
        );
    }
}

#[async_trait]
impl EmbeddingCapable for OpenAiProvider {
    async fn create_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let embedding_model = std::env::var("GOOSE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        // Create span following OpenTelemetry GenAI semantic conventions
        // Span name format: "{operation} {model}" per https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/
        let span = tracing::info_span!(
            "embedding",
            otel.name = format!("embedding {}", embedding_model),
            gen_ai.request.model = %embedding_model,
            gen_ai.system = "openai",
            gen_ai.operation.name = "embedding"
        );

        let _enter = span.enter();

        let request = EmbeddingRequest {
            input: texts,
            model: embedding_model,
        };

        let response = self
            .api_client
            .api_post("v1/embeddings", &serde_json::to_value(request)?)
            .await?;

        if response.status != StatusCode::OK {
            let error_text = response
                .payload
                .as_ref()
                .and_then(|p| p.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow::anyhow!("Embedding API error: {}", error_text));
        }

        let embedding_response: EmbeddingResponse = serde_json::from_value(
            response
                .payload
                .ok_or_else(|| anyhow::anyhow!("Empty response body"))?,
        )?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }
}
