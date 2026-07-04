use crate::api_client::{AuthMethod, TlsConfig};
use crate::base::ProviderDescriptor;
use crate::declarative::{DeclarativeProviderConfig, KeyResolver};
use crate::errors::ProviderError;
use crate::request_log::{start_log, LoggerHandleExt};
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde_json::Value;
use std::io;
use tokio::pin;
use tokio_util::io::StreamReader;

use super::api_client::ApiClient;
use super::base::{ConfigKey, MessageStream, ModelInfo, Provider, ProviderMetadata};
use super::formats::anthropic::{
    create_request, response_to_streaming_message, AnthropicFormatOptions, ANTHROPIC_PROVIDER_NAME,
};
use super::openai_compatible::handle_status;
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const ANTHROPIC_DEFAULT_MODEL: &str = "claude-sonnet-4-5";
pub const ANTHROPIC_DEFAULT_FAST_MODEL: &str = "claude-haiku-4-5";
const ANTHROPIC_KNOWN_MODELS: &[&str] = &[
    // Claude 5 models
    "claude-fable-5",
    "claude-sonnet-5",
    "claude-opus-4-8",
    "claude-opus-4-7",
    // Claude 4.6 models
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    // Claude 4.5 models with aliases
    "claude-sonnet-4-5",
    "claude-sonnet-4-5-20250929",
    "claude-haiku-4-5",
    "claude-haiku-4-5-20251001",
    "claude-opus-4-5",
    "claude-opus-4-5-20251101",
    // Legacy Claude 4.0 models
    "claude-sonnet-4-0",
    "claude-sonnet-4-20250514",
    "claude-opus-4-0",
    "claude-opus-4-20250514",
];

const ANTHROPIC_DOC_URL: &str = "https://docs.anthropic.com/en/docs/about-claude/models";
pub const ANTHROPIC_API_VERSION: &str = "2023-06-01";

#[derive(serde::Serialize)]
pub struct AnthropicProvider {
    #[serde(skip)]
    api_client: ApiClient,
    supports_streaming: bool,
    name: String,
    custom_models: Option<Vec<String>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    #[serde(skip)]
    format_options: AnthropicFormatOptions,
}

/// Builder for [`AnthropicProvider`].
///
/// Exposes every field of the provider so that constructors living outside
/// `anthropic.rs` (e.g. in `anthropic_def.rs`, which lives in the `gosling`
/// crate) can assemble a provider without needing direct access to the
/// struct's private fields.
pub struct AnthropicProviderBuilder {
    api_client: ApiClient,
    supports_streaming: bool,
    name: String,
    custom_models: Option<Vec<String>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    format_options: AnthropicFormatOptions,
}

impl AnthropicProviderBuilder {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            supports_streaming: true,
            name: ANTHROPIC_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            format_options: AnthropicFormatOptions::default(),
        }
    }

    pub fn api_client(mut self, api_client: ApiClient) -> Self {
        self.api_client = api_client;
        self
    }

    pub fn map_api_client(mut self, f: impl FnOnce(ApiClient) -> ApiClient) -> Self {
        self.api_client = f(self.api_client);
        self
    }

    pub fn try_map_api_client(
        mut self,
        f: impl FnOnce(ApiClient) -> Result<ApiClient>,
    ) -> Result<Self> {
        self.api_client = f(self.api_client)?;
        Ok(self)
    }

    pub fn supports_streaming(mut self, supports_streaming: bool) -> Self {
        self.supports_streaming = supports_streaming;
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn custom_models(mut self, custom_models: Option<Vec<String>>) -> Self {
        self.custom_models = custom_models;
        self
    }

    pub fn dynamic_models(mut self, dynamic_models: Option<bool>) -> Self {
        self.dynamic_models = dynamic_models;
        self
    }

    pub fn skip_canonical_filtering(mut self, skip_canonical_filtering: bool) -> Self {
        self.skip_canonical_filtering = skip_canonical_filtering;
        self
    }

    pub fn format_options(mut self, format_options: AnthropicFormatOptions) -> Self {
        self.format_options = format_options;
        self
    }

    pub fn build(self) -> AnthropicProvider {
        AnthropicProvider {
            api_client: self.api_client,
            supports_streaming: self.supports_streaming,
            name: self.name,
            custom_models: self.custom_models,
            dynamic_models: self.dynamic_models,
            skip_canonical_filtering: self.skip_canonical_filtering,
            format_options: self.format_options,
        }
    }
}

impl AnthropicProvider {
    async fn fetch_models_from_api(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.api_client.request("v1/models").response_get().await?;

        if response.status() == StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|p| {
                    p.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| "models endpoint not found".to_string());
            return Err(ProviderError::EndpointNotFound(msg));
        }

        // Check HTTP status first — non-success responses (auth errors,
        // rate limits, bad requests, etc.) propagate unchanged.
        let response = handle_status(response).await?;

        // Read the response body. Network errors (timeouts, connection resets)
        // must propagate — only JSON decode failures indicate a non-models
        // endpoint (e.g. an HTML error page from a misconfigured base URL).
        let body = response.text().await.map_err(|e| {
            ProviderError::NetworkError(format!("Failed to read response body: {}", e))
        })?;
        let json: Value = serde_json::from_str(&body).map_err(|e| {
            ProviderError::EndpointNotFound(format!("Response body is not valid JSON: {}", e))
        })?;

        // Check for the standard error shape first — this is always an
        // error regardless of whether `data` is also present.
        if let Some(err_obj) = json.get("error") {
            let msg = err_obj
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            let kind = err_obj.get("type").and_then(|v| v.as_str());
            return Err(ProviderError::from_models_error_payload(kind, msg));
        }

        // A 200 response whose body isn't a models payload (no `data` array)
        // is treated as `EndpointNotFound` so that `fetch_supported_models`
        // can fall back to the predefined list. Before giving up, check for
        // a top-level `message` field as an error indicator — some providers
        // return {"message": "invalid api key"} instead of the standard
        // {"error": {"message": "..."}} shape. Only treat it as an error
        // when `data` is missing, so that a valid payload like
        // {"data": [...], "message": "ok"} is not rejected.
        let arr = match json.get("data").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
                    let kind = json.get("type").and_then(|v| v.as_str());
                    return Err(ProviderError::from_models_error_payload(kind, msg));
                }
                return Err(ProviderError::EndpointNotFound(
                    "response is not a models payload (missing 'data' array)".into(),
                ));
            }
        };

        let mut models: Vec<String> = arr
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        models.sort();
        Ok(models)
    }
}

impl ProviderDescriptor for AnthropicProvider {
    fn metadata() -> ProviderMetadata {
        // Context limits come from the canonical registry: several current
        // models (opus-4-6 and later, fable-5) have 1M-token windows, so a
        // hardcoded 200k here misreports the context meter for them.
        let models: Vec<ModelInfo> = ANTHROPIC_KNOWN_MODELS
            .iter()
            .map(|&model_name| {
                let context_limit = ModelConfig::new(model_name)
                    .with_canonical_limits(ANTHROPIC_PROVIDER_NAME)
                    .context_limit();
                ModelInfo::new(model_name, context_limit)
            })
            .collect();

        ProviderMetadata::with_models(
            ANTHROPIC_PROVIDER_NAME,
            "Anthropic",
            "Claude and other models from Anthropic",
            ANTHROPIC_DEFAULT_MODEL,
            models,
            ANTHROPIC_DOC_URL,
            vec![
                ConfigKey::new("ANTHROPIC_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "ANTHROPIC_HOST",
                    true,
                    false,
                    Some("https://api.anthropic.com"),
                    false,
                ),
            ],
        )
        .with_fast_model(ANTHROPIC_DEFAULT_FAST_MODEL)
        .with_setup_steps(vec![
            "Go to https://platform.claude.com/settings/keys",
            "Click 'Create Key'",
            "Copy the key and paste it above",
        ])
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        if let Some(custom_models) = &self.custom_models {
            if self.dynamic_models == Some(false) {
                return Ok(custom_models.clone());
            }
            match self.fetch_models_from_api().await {
                Ok(models) => return Ok(models),
                Err(e) if e.is_endpoint_not_found() => {
                    tracing::debug!(
                        "Models endpoint not implemented for provider '{}' ({}), using predefined list",
                        self.name,
                        e
                    );
                    return Ok(custom_models.clone());
                }
                Err(e) => return Err(e),
            }
        }

        self.fetch_models_from_api().await
    }

    async fn stream(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let mut payload = create_request(
            ANTHROPIC_PROVIDER_NAME,
            model_config,
            system,
            messages,
            tools,
            self.format_options,
        )?;
        payload
            .as_object_mut()
            .unwrap()
            .insert("stream".to_string(), Value::Bool(true));

        let mut log = start_log(model_config, &payload)?;

        let response = self
            .with_retry(|| async {
                let request = self.api_client.request("v1/messages");
                let resp = request.response_post(&payload).await?;
                handle_status(resp).await
            })
            .await
            .inspect_err(|e| {
                let _ = log.error(e);
            })?;

        let stream = response.bytes_stream().map_err(io::Error::other);

        Ok(Box::pin(try_stream! {
            let stream_reader = StreamReader::new(stream);
            let framed = tokio_util::codec::FramedRead::new(stream_reader, tokio_util::codec::LinesCodec::new()).map_err(anyhow::Error::from);

            let message_stream = response_to_streaming_message(framed);
            pin!(message_stream);
            while let Some(message) = futures::StreamExt::next(&mut message_stream).await {
                let (message, usage) = message.map_err(ProviderError::from_stream_error)?;
                log.write(&message, usage.as_ref().map(|f| f.usage).as_ref())?;
                yield (message, usage);
            }
        }))
    }
}

fn format_options_for_provider(preserves_thinking: bool) -> AnthropicFormatOptions {
    AnthropicFormatOptions {
        preserve_unsigned_thinking: preserves_thinking,
        preserve_thinking_context: preserves_thinking,
        thinking_disabled: false,
    }
}

pub fn from_declarative_config(
    config: DeclarativeProviderConfig,
    tls_config: Option<TlsConfig>,
    key_resolver: impl KeyResolver,
) -> Result<AnthropicProviderBuilder> {
    let custom_models = if !config.models.is_empty() {
        Some(
            config
                .models
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<String>>(),
        )
    } else {
        None
    };

    if config.dynamic_models == Some(false) && custom_models.is_none() {
        return Err(anyhow::anyhow!(
            "Provider '{}' has dynamic_models: false but no static models listed; \
             at least one entry in `models` is required.",
            config.name
        ));
    }

    let api_key = if config.api_key_env.is_empty() {
        None
    } else {
        match key_resolver.resolve_key(config.api_key_env.as_str()) {
            Ok(key) => Some(key),
            Err(err) => {
                if config.requires_auth {
                    anyhow::bail!("missing required key {}: {}", config.api_key_env, err);
                }
                None
            }
        }
    };

    let auth = match api_key {
        Some(key) if !key.is_empty() => AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key,
        },
        _ => AuthMethod::NoAuth,
    };

    let format_options = format_options_for_provider(config.preserves_thinking);

    let mut api_client = ApiClient::new_with_tls(config.base_url, auth, tls_config)?;

    if let Some(headers) = &config.headers {
        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert(
            reqwest::header::HeaderName::from_static("anthropic-version"),
            reqwest::header::HeaderValue::from_static(ANTHROPIC_API_VERSION),
        );
        for (key, value) in headers {
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
            let header_value = reqwest::header::HeaderValue::from_str(value)?;
            header_map.insert(header_name, header_value);
        }
        api_client = api_client.with_headers(header_map)?;
    } else {
        api_client = api_client.with_header("anthropic-version", ANTHROPIC_API_VERSION)?;
    }

    let supports_streaming = config.supports_streaming.unwrap_or(true);

    if !supports_streaming {
        return Err(anyhow::anyhow!(
            "Anthropic provider does not support non-streaming mode. All Claude models support streaming. \
            Please remove 'supports_streaming: false' from your provider configuration."
        ));
    }

    Ok(AnthropicProviderBuilder::new(api_client)
        .supports_streaming(supports_streaming)
        .name(config.name.clone())
        .custom_models(custom_models)
        .dynamic_models(config.dynamic_models)
        .skip_canonical_filtering(config.skip_canonical_filtering)
        .format_options(format_options))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_client::AuthMethod;
    use serde_json::json;

    fn make_provider_with_custom_models(
        host: &str,
        custom_models: Vec<String>,
    ) -> AnthropicProvider {
        AnthropicProvider {
            api_client: ApiClient::new_with_tls(host.to_string(), AuthMethod::NoAuth, None)
                .unwrap(),
            supports_streaming: true,
            name: "test-provider".to_string(),
            custom_models: Some(custom_models),
            dynamic_models: Some(true),
            skip_canonical_filtering: false,
            format_options: AnthropicFormatOptions::default(),
        }
    }

    #[tokio::test]
    async fn fetch_models_treats_invalid_json_as_endpoint_not_found() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // Simulate a misconfigured base URL that returns HTML instead of JSON.
        // The body is read successfully (text), but `serde_json::from_str`
        // fails — a decode error, classified as `EndpointNotFound`.
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("<html>not a models endpoint</html>"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        let err = provider.fetch_models_from_api().await.unwrap_err();
        assert!(
            err.is_endpoint_not_found(),
            "expected EndpointNotFound, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_models_treats_missing_data_field_as_endpoint_not_found() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // JSON response without a `data` array — not a models payload.
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
            .expect(1)
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        let err = provider.fetch_models_from_api().await.unwrap_err();
        assert!(
            err.is_endpoint_not_found(),
            "expected EndpointNotFound, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_supported_models_falls_back_on_invalid_payload() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html>error page</html>"))
            .mount(&server)
            .await;

        let predefined = vec![
            "claude-sonnet-4-5".to_string(),
            "claude-haiku-4-5".to_string(),
        ];
        let provider = make_provider_with_custom_models(&server.uri(), predefined.clone());

        // The invalid payload triggers EndpointNotFound, which the existing
        // fetch_supported_models fallback catches and returns the static list.
        let models = provider
            .fetch_supported_models()
            .await
            .expect("should fall back to predefined list on invalid payload");
        assert_eq!(models, predefined);
    }

    #[tokio::test]
    async fn fetch_supported_models_propagates_auth_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "type": "error",
                "error": {
                    "type": "authentication_error",
                    "message": "invalid api key"
                }
            })))
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        // Auth errors must propagate — they indicate a real misconfiguration
        // that the user needs to know about.
        let err = provider.fetch_supported_models().await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Authentication(_)),
            "expected Authentication error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_supported_models_propagates_auth_error_from_200_payload() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // Some Anthropic-compatible endpoints return 200 with an error payload
        // instead of a proper 401 status. The error must still propagate.
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "error",
                "error": {
                    "type": "authentication_error",
                    "message": "invalid api key"
                }
            })))
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        let err = provider.fetch_supported_models().await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Authentication(_)),
            "expected Authentication error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_supported_models_preserves_server_error_payload() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "error",
                "error": {
                    "type": "overloaded_error",
                    "message": "server overloaded"
                }
            })))
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        let err = provider.fetch_supported_models().await.unwrap_err();
        assert!(
            matches!(err, ProviderError::ServerError(_)),
            "expected ServerError, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_supported_models_propagates_top_level_message_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // Some providers return errors with a top-level `message` field
        // instead of the standard `{"error": {"message": "..."}}` shape.
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "message": "invalid api key"
            })))
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        let err = provider.fetch_supported_models().await.unwrap_err();
        assert!(
            matches!(err, ProviderError::Authentication(_)),
            "expected Authentication error, got: {:?}",
            err
        );
    }

    #[tokio::test]
    async fn fetch_supported_models_ignores_top_level_message_when_data_present() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // A valid models payload with an extra informational `message` field
        // must NOT be treated as an error. The `data` array takes precedence.
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{"id": "model-a"}, {"id": "model-b"}],
                "message": "ok"
            })))
            .mount(&server)
            .await;

        let provider =
            make_provider_with_custom_models(&server.uri(), vec!["static-model".to_string()]);

        let models = provider
            .fetch_supported_models()
            .await
            .expect("valid payload with extra message field should succeed");
        assert_eq!(models, vec!["model-a".to_string(), "model-b".to_string()]);
    }
}
