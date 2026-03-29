use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, MessageStream, Provider, ProviderDef, ProviderMetadata};
use super::errors::ProviderError;
use super::openai_compatible::handle_status_openai_compat;
use super::retry::{ProviderRetry, RetryConfig};
use super::utils::{ImageFormat, RequestLog};
use crate::config::declarative_providers::DeclarativeProviderConfig;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use crate::providers::formats::ollama::{create_request, response_to_streaming_message_ollama};
use anyhow::{Error, Result};
use async_stream::try_stream;
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::TryStreamExt;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Response,
};
use rmcp::model::Tool;
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};
use tokio::pin;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_util::io::StreamReader;
use url::Url;

const OLLAMA_PROVIDER_NAME: &str = "ollama";
pub const OLLAMA_HOST: &str = "localhost";
pub const OLLAMA_API_KEY: &str = "OLLAMA_API_KEY";
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

#[derive(serde::Serialize)]
pub struct OllamaProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
    supports_streaming: bool,
    name: String,
}
fn resolve_ollama_num_ctx(model_config: &ModelConfig) -> Option<usize> {
    let config = crate::config::Config::global();
    let input_limit = match config.get_param::<usize>("GOOSE_INPUT_LIMIT") {
        Ok(limit) if limit > 0 => Some(limit),
        Ok(_) => None,
        Err(crate::config::ConfigError::NotFound(_)) => None,
        Err(e) => {
            tracing::warn!("Invalid GOOSE_INPUT_LIMIT value: {}", e);
            None
        }
    };

    input_limit.or(model_config.context_limit)
}

fn apply_ollama_options(payload: &mut Value, model_config: &ModelConfig) {
    if let Some(obj) = payload.as_object_mut() {
        // Ollama does not support stream_options; remove it to prevent hangs.
        obj.remove("stream_options");

        // Convert max_completion_tokens / max_tokens to Ollama's options.num_predict.
        // Reasoning models emit max_completion_tokens; non-reasoning models emit max_tokens.
        let max_tokens = obj
            .remove("max_completion_tokens")
            .or_else(|| obj.remove("max_tokens"));
        if let Some(max_tokens) = max_tokens {
            let options = obj.entry("options").or_insert_with(|| json!({}));
            if let Some(options_obj) = options.as_object_mut() {
                options_obj.entry("num_predict").or_insert(max_tokens);
            }
        }

        // Apply num_ctx from context limit settings.
        if let Some(limit) = resolve_ollama_num_ctx(model_config) {
            let options = obj.entry("options").or_insert_with(|| json!({}));
            if let Some(options_obj) = options.as_object_mut() {
                options_obj.insert("num_ctx".to_string(), json!(limit));
            }
        }
    }
}

impl OllamaProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let host: String = config
            .get_param("OLLAMA_HOST")
            .unwrap_or_else(|_| OLLAMA_HOST.to_string());
        let timeout =
            Duration::from_secs(config.get_param("OLLAMA_TIMEOUT").unwrap_or(OLLAMA_TIMEOUT));
        let base_url = normalized_ollama_base_url(&host)?;
        let api_key = optional_ollama_api_key(config, OLLAMA_API_KEY, &base_url);
        let api_client = build_api_client(base_url, timeout, api_key, None)?;

        Ok(Self {
            api_client,
            model,
            supports_streaming: true,
            name: OLLAMA_PROVIDER_NAME.to_string(),
        })
    }

    pub fn from_custom_config(
        model: ModelConfig,
        config: DeclarativeProviderConfig,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(OLLAMA_TIMEOUT));
        let base_url = normalized_ollama_base_url(&config.base_url)?;
        let api_key = optional_ollama_api_key(
            crate::config::Config::global(),
            &config.api_key_env,
            &base_url,
        );
        let api_client = build_api_client(base_url, timeout, api_key, config.headers.as_ref())?;

        let supports_streaming = config.supports_streaming.unwrap_or(true);

        if !supports_streaming {
            return Err(anyhow::anyhow!(
                "Ollama provider does not support non-streaming mode. All Ollama models support streaming. \
                Please remove 'supports_streaming: false' from your provider configuration."
            ));
        }

        Ok(Self {
            api_client,
            model,
            supports_streaming,
            name: config.name.clone(),
        })
    }
}

impl ProviderDef for OllamaProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            OLLAMA_PROVIDER_NAME,
            "Ollama",
            "Local and hosted open source models",
            OLLAMA_DEFAULT_MODEL,
            OLLAMA_KNOWN_MODELS.to_vec(),
            OLLAMA_DOC_URL,
            vec![
                ConfigKey::new("OLLAMA_HOST", true, false, Some(OLLAMA_HOST), true),
                ConfigKey::new(OLLAMA_API_KEY, false, true, None, true),
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

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(Self::from_env(model))
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
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
        session_id: &str,
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
        apply_ollama_options(&mut payload, model_config);
        let mut log = RequestLog::start(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let resp = self
                    .api_client
                    .response_post(Some(session_id), "v1/chat/completions", &payload)
                    .await?;
                handle_status_openai_compat(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;
        stream_ollama(response, log)
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self
            .api_client
            .request(None, "api/tags")
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

/// Per-chunk timeout for Ollama streaming responses.
/// If no new raw SSE data arrives within this duration, the connection is considered dead.
const OLLAMA_CHUNK_TIMEOUT_SECS: u64 = 30;

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
                         Try a smaller model or reduce the number of tools.",
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
fn stream_ollama(response: Response, mut log: RequestLog) -> Result<MessageStream, ProviderError> {
    let stream = response.bytes_stream().map_err(std::io::Error::other);

    Ok(Box::pin(try_stream! {
        let stream_reader = StreamReader::new(stream);
        let framed = FramedRead::new(stream_reader, LinesCodec::new())
            .map_err(Error::from);

        let timed_lines = with_line_timeout(framed, OLLAMA_CHUNK_TIMEOUT_SECS);
        let message_stream = response_to_streaming_message_ollama(timed_lines);
        pin!(message_stream);

        while let Some(message) = message_stream.next().await {
            let (message, usage) = message.map_err(|e|
                ProviderError::RequestFailed(format!("Stream decode error: {}", e))
            )?;
            log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
            yield (message, usage);
        }
    }))
}

pub(crate) fn optional_ollama_api_key(
    config: &crate::config::Config,
    key_name: &str,
    base_url: &Url,
) -> Option<String> {
    if !should_lookup_api_key(key_name, base_url) {
        return None;
    }

    config
        .get_secret::<String>(key_name)
        .ok()
        .and_then(normalize_api_key)
}

pub(crate) fn normalized_ollama_base_url(host: &str) -> Result<Url> {
    let base = if host.starts_with("http://") || host.starts_with("https://") {
        host.to_string()
    } else {
        format!("http://{}", host)
    };

    let mut base_url = Url::parse(&base).map_err(|e| anyhow::anyhow!("Invalid base URL: {e}"))?;
    strip_known_api_suffix(&mut base_url);

    if base_url.port().is_none() && base_url.scheme() == "http" && is_local_host(&base_url) {
        base_url
            .set_port(Some(OLLAMA_DEFAULT_PORT))
            .map_err(|_| anyhow::anyhow!("Failed to set default port"))?;
    }

    Ok(base_url)
}

fn build_api_client(
    base_url: Url,
    timeout: Duration,
    api_key: Option<String>,
    headers: Option<&HashMap<String, String>>,
) -> Result<ApiClient> {
    let auth = api_key
        .map(AuthMethod::BearerToken)
        .unwrap_or(AuthMethod::NoAuth);
    let mut api_client = ApiClient::with_timeout(base_url.to_string(), auth, timeout)?;

    if let Some(headers) = headers {
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())?;
            let header_value = HeaderValue::from_str(value)?;
            header_map.insert(header_name, header_value);
        }
        api_client = api_client.with_headers(header_map)?;
    }

    Ok(api_client)
}

fn normalize_api_key(api_key: String) -> Option<String> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("notrequired") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn should_lookup_api_key(key_name: &str, base_url: &Url) -> bool {
    let trimmed = key_name.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("notrequired") {
        return false;
    }

    std::env::var(trimmed).is_ok() || !is_local_host(base_url)
}

fn strip_known_api_suffix(base_url: &mut Url) {
    let path = base_url.path().trim_end_matches('/').to_string();
    for suffix in ["/v1", "/api"] {
        if let Some(prefix) = path.strip_suffix(suffix) {
            let normalized = if prefix.is_empty() { "/" } else { prefix };
            base_url.set_path(normalized);
            return;
        }
    }
}

fn is_local_host(base_url: &Url) -> bool {
    matches!(
        base_url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::base::ModelInfo;
    use serial_test::serial;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_apply_ollama_options_uses_input_limit() {
        let _guard = env_lock::lock_env([("GOOSE_INPUT_LIMIT", Some("8192"))]);
        let model_config = ModelConfig::new("qwen3")
            .unwrap()
            .with_context_limit(Some(16_000));
        let mut payload = json!({});
        apply_ollama_options(&mut payload, &model_config);
        assert_eq!(payload["options"]["num_ctx"], 8192);
    }

    #[test]
    fn test_apply_ollama_options_falls_back_to_context_limit() {
        let _guard = env_lock::lock_env([("GOOSE_INPUT_LIMIT", None::<&str>)]);
        let model_config = ModelConfig::new("qwen3")
            .unwrap()
            .with_context_limit(Some(12_000));
        let mut payload = json!({});
        apply_ollama_options(&mut payload, &model_config);
        assert_eq!(payload["options"]["num_ctx"], 12_000);
    }

    #[test]
    fn test_apply_ollama_options_skips_when_no_limit() {
        let _guard = env_lock::lock_env([("GOOSE_INPUT_LIMIT", None::<&str>)]);
        let mut model_config = ModelConfig::new("qwen3").unwrap();
        model_config.context_limit = None;
        let mut payload = json!({});
        apply_ollama_options(&mut payload, &model_config);
        assert!(payload.get("options").is_none());
    }

    #[test]
    fn test_raw_create_request_contains_unsupported_ollama_fields() {
        use crate::providers::formats::ollama::create_request;
        use crate::providers::utils::ImageFormat;

        let model_config = ModelConfig::new("llama3.1")
            .unwrap()
            .with_max_tokens(Some(4096));
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
            "create_request should produce stream_options (unsupported by Ollama)"
        );
        assert!(
            payload.get("max_tokens").is_some(),
            "create_request should produce max_tokens (unsupported by Ollama)"
        );
    }

    #[test]
    fn test_apply_ollama_options_strips_unsupported_fields() {
        use crate::providers::formats::ollama::create_request;
        use crate::providers::utils::ImageFormat;

        let _guard = env_lock::lock_env([("GOOSE_INPUT_LIMIT", None::<&str>)]);
        let model_config = ModelConfig::new("llama3.1")
            .unwrap()
            .with_max_tokens(Some(4096));
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

        apply_ollama_options(&mut payload, &model_config);

        assert!(
            payload.get("stream_options").is_none(),
            "stream_options should be removed for Ollama"
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

    #[tokio::test]
    async fn test_stream_ollama_timeout_on_stall() {
        use std::convert::Infallible;

        let (tx, rx) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, Infallible>>(1);
        tx.send(Ok(bytes::Bytes::from(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"},\"index\":0}],\
             \"model\":\"test\",\"object\":\"chat.completion.chunk\",\"created\":0}\n",
        )))
        .await
        .unwrap();
        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let body = reqwest::Body::wrap_stream(stream);
        let response = http::Response::builder().status(200).body(body).unwrap();
        let response: reqwest::Response = response.into();

        let log = RequestLog::start(
            &ModelConfig::new("test").unwrap(),
            &json!({"model": "test"}),
        )
        .unwrap();

        let mut msg_stream = stream_ollama(response, log).unwrap();

        let result =
            tokio::time::timeout(Duration::from_secs(OLLAMA_CHUNK_TIMEOUT_SECS + 5), async {
                let mut last_err = None;
                while let Some(item) = msg_stream.next().await {
                    if let Err(e) = item {
                        last_err = Some(e);
                        break;
                    }
                }
                last_err
            })
            .await;

        match result {
            Ok(Some(err)) => {
                let err_msg = err.to_string();
                assert!(
                    err_msg.contains("stream stalled"),
                    "Expected stall timeout error, got: {}",
                    err_msg
                );
            }
            Ok(None) => panic!("Expected timeout error but stream completed normally"),
            Err(_) => panic!("Outer timeout elapsed -- per-chunk timeout did not fire"),
        }

        drop(tx);
    }

    #[test]
    fn test_ollama_retry_config_is_transient_only() {
        let config = RetryConfig::new(
            OLLAMA_MAX_RETRIES,
            OLLAMA_INITIAL_RETRY_INTERVAL_MS,
            OLLAMA_BACKOFF_MULTIPLIER,
            OLLAMA_MAX_RETRY_INTERVAL_MS,
        )
        .transient_only();

        assert!(config.transient_only);

        use super::super::errors::ProviderError;
        use super::super::retry::should_retry;

        assert!(!should_retry(
            &ProviderError::RequestFailed("Resource not found (404)".into()),
            &config
        ));
        assert!(!should_retry(
            &ProviderError::RequestFailed("Bad request (400)".into()),
            &config
        ));
        assert!(should_retry(
            &ProviderError::ServerError("500 model loading".into()),
            &config
        ));
        assert!(should_retry(
            &ProviderError::NetworkError("connection refused".into()),
            &config
        ));
    }

    fn test_provider(base_url: String, api_key_env: &str) -> OllamaProvider {
        let provider_config = DeclarativeProviderConfig {
            name: "custom_ollama_cloud".to_string(),
            engine: crate::config::declarative_providers::ProviderEngine::Ollama,
            display_name: "Ollama Cloud".to_string(),
            description: None,
            api_key_env: api_key_env.to_string(),
            base_url,
            models: vec![ModelInfo::new("qwen3", 128_000)],
            headers: None,
            timeout_seconds: Some(5),
            supports_streaming: Some(true),
            requires_auth: true,
            catalog_provider_id: None,
            base_path: None,
            env_vars: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
        };
        OllamaProvider::from_custom_config(ModelConfig::new_or_fail("qwen3"), provider_config)
            .expect("provider should build")
    }

    #[tokio::test]
    #[serial]
    async fn custom_config_uses_bearer_auth_for_remote_chat_requests() {
        let server = MockServer::start().await;
        let _guard = env_lock::lock_env([("TEST_OLLAMA_API_KEY", Some("ollama-cloud-key"))]);

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer ollama-cloud-key"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;

        let provider = test_provider(server.uri(), "TEST_OLLAMA_API_KEY");
        let response = provider
            .api_client
            .response_post(
                Some("test-session"),
                "v1/chat/completions",
                &json!({"model": "qwen3", "messages": []}),
            )
            .await;

        assert!(response.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn from_env_uses_bearer_auth_for_remote_chat_requests() {
        let server = MockServer::start().await;
        let server_uri = server.uri();
        let _guard = env_lock::lock_env([
            ("OLLAMA_HOST", Some(server_uri.as_str())),
            ("OLLAMA_API_KEY", Some("ollama-cloud-key")),
        ]);

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer ollama-cloud-key"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .mount(&server)
            .await;

        let provider = OllamaProvider::from_env(ModelConfig::new_or_fail("qwen3"))
            .await
            .expect("provider should build");
        let response = provider
            .api_client
            .response_post(
                Some("test-session"),
                "v1/chat/completions",
                &json!({"model": "qwen3", "messages": []}),
            )
            .await;

        assert!(response.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn custom_config_uses_bearer_auth_for_model_listing() {
        let server = MockServer::start().await;
        let _guard = env_lock::lock_env([("TEST_OLLAMA_API_KEY", Some("ollama-cloud-key"))]);

        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .and(header("authorization", "Bearer ollama-cloud-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "models": [{"name": "qwen3"}, {"name": "gpt-oss:120b"}]
            })))
            .mount(&server)
            .await;

        let provider = test_provider(server.uri(), "TEST_OLLAMA_API_KEY");
        let models = provider.fetch_supported_models().await.unwrap();

        assert_eq!(
            models,
            vec!["gpt-oss:120b".to_string(), "qwen3".to_string()]
        );
    }

    #[test]
    fn normalizes_v1_and_api_suffixes_to_server_root() {
        let v1_url = normalized_ollama_base_url("https://ollama.com/v1").unwrap();
        let api_url = normalized_ollama_base_url("https://ollama.com/api").unwrap();

        assert_eq!(v1_url.as_str(), "https://ollama.com/");
        assert_eq!(api_url.as_str(), "https://ollama.com/");
    }

    #[test]
    fn ignores_placeholder_api_key_names() {
        let remote_url = normalized_ollama_base_url("https://ollama.com").unwrap();

        assert!(!should_lookup_api_key("", &remote_url));
        assert!(!should_lookup_api_key("   ", &remote_url));
        assert!(!should_lookup_api_key("notrequired", &remote_url));
        assert!(!should_lookup_api_key("NotRequired", &remote_url));
        assert!(should_lookup_api_key("OLLAMA_API_KEY", &remote_url));
    }

    #[test]
    #[serial]
    fn skips_optional_api_key_lookup_for_local_hosts_without_env() {
        let base_url = normalized_ollama_base_url("localhost").unwrap();
        let _guard = env_lock::lock_env([("OLLAMA_API_KEY", None::<&str>)]);

        assert!(!should_lookup_api_key("OLLAMA_API_KEY", &base_url));
    }

    #[test]
    #[serial]
    fn allows_optional_api_key_lookup_for_local_hosts_when_env_is_set() {
        let base_url = normalized_ollama_base_url("localhost").unwrap();
        let _guard = env_lock::lock_env([("OLLAMA_API_KEY", Some("ollama-cloud-key"))]);

        assert!(should_lookup_api_key("OLLAMA_API_KEY", &base_url));
    }
}
