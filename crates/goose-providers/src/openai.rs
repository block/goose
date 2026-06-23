use super::api_client::ApiClient;
use super::base::{ConfigKey, ModelInfo, Provider, ProviderMetadata};
use super::retry::ProviderRetry;
use crate::conversation::message::Message;
use crate::conversation::token_usage::ProviderUsage;
use crate::errors::ProviderError;
use crate::formats::openai::is_openai_responses_model;
use crate::formats::openai::{
    create_request_with_options, get_usage, response_to_message, OpenAiFormatOptions,
};
use crate::formats::openai_responses::{
    create_responses_request, get_responses_usage, responses_api_to_message, ResponsesApiResponse,
};
use crate::images::ImageFormat;
use crate::openai_compatible::{
    handle_response_openai_compat, handle_status, stream_openai_compat, stream_responses_compat,
};
use crate::request_log::{start_log, LoggerHandleExt};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::StatusCode;
use std::collections::HashMap;

use crate::base::{MessageStream, ProviderDescriptor};
use crate::model::ModelConfig;
use rmcp::model::Tool;

pub const OPEN_AI_PROVIDER_NAME: &str = "openai";
pub const OPEN_AI_DEFAULT_BASE_PATH: &str = "v1/chat/completions";
pub const OPEN_AI_VERSIONLESS_BASE_PATH: &str = "chat/completions";
const OPEN_AI_DEFAULT_RESPONSES_PATH: &str = "v1/responses";
const OPEN_AI_DEFAULT_MODELS_PATH: &str = "v1/models";
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
    ("gpt-5", 400_000),
    ("gpt-5-mini", 400_000),
    ("gpt-5-nano", 400_000),
    ("gpt-5-pro", 400_000),
    ("gpt-5-codex", 400_000),
    ("gpt-5.1", 400_000),
    ("gpt-5.1-codex", 400_000),
    ("gpt-5.2", 400_000),
    ("gpt-5.2-codex", 400_000),
    ("gpt-5.2-pro", 400_000),
    ("gpt-5.3-codex", 400_000),
    ("gpt-5.4", 1_050_000),
    ("gpt-5.4-mini", 400_000),
    ("gpt-5.4-nano", 400_000),
    ("gpt-5.4-pro", 1_050_000),
];

pub const OPEN_AI_DOC_URL: &str = "https://platform.openai.com/docs/models";

type OpenAiBaseUrlParts = (String, Vec<(String, String)>, bool);

/// Ensure a base URL has an explicit scheme.
///
/// Users frequently enter hosts like `localhost:1234` without a scheme. The
/// `url` crate parses such input as `scheme="localhost"`, `path="1234"`,
/// silently dropping both the host and the port. When no `://` is present we
/// prepend a sensible scheme (`http://` for local hosts, `https://`
/// otherwise) so the host and port survive parsing.
pub fn ensure_url_scheme(raw_url: &str) -> String {
    let trimmed = raw_url.trim();
    if trimmed.contains("://") {
        return trimmed.to_string();
    }

    let host_part = trimmed.split(['/', '?']).next().unwrap_or(trimmed);
    let bare_host = if let Some(rest) = host_part.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest)
    } else {
        host_part.split(':').next().unwrap_or(host_part)
    };
    let is_local = bare_host == "localhost"
        || bare_host == "127.0.0.1"
        || bare_host == "0.0.0.0"
        || bare_host == "::1";

    let scheme = if is_local { "http" } else { "https" };
    format!("{}://{}", scheme, trimmed)
}

pub fn parse_openai_base_url(raw_url: &str) -> Result<OpenAiBaseUrlParts> {
    let raw_url = ensure_url_scheme(raw_url);
    let raw_url = raw_url.as_str();
    let parsed = url::Url::parse(raw_url)
        .map_err(|e| anyhow::anyhow!("Invalid OPENAI_BASE_URL '{}': {}", raw_url, e))?;

    let authority = parsed[..url::Position::BeforePath].to_string();
    let query_params: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let path = parsed.path().trim_end_matches('/');
    if path.is_empty() || path == "/" {
        return Ok((authority, query_params, true));
    }

    if path == "/v1" {
        return Ok((authority, query_params, true));
    }
    if let Some(prefix) = path.strip_suffix("/v1") {
        return Ok((format!("{}{}", authority, prefix), query_params, true));
    }

    Ok((format!("{}{}", authority, path), query_params, false))
}

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
    name: String,
    custom_models: Option<Vec<String>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    preserve_thinking_context: bool,
}

/// Builder for [`OpenAiProvider`].
///
/// Exposes every field of the provider so that constructors living outside
/// `openai.rs` (e.g. in `openai_def.rs`) can assemble a provider without
/// needing direct access to the struct's private fields.
pub struct OpenAiProviderBuilder {
    api_client: ApiClient,
    base_path: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
    supports_streaming: bool,
    name: String,
    custom_models: Option<Vec<String>>,
    dynamic_models: Option<bool>,
    skip_canonical_filtering: bool,
    preserve_thinking_context: bool,
}

impl OpenAiProviderBuilder {
    pub fn new(api_client: ApiClient, model: ModelConfig) -> Self {
        Self {
            api_client,
            base_path: OPEN_AI_DEFAULT_BASE_PATH.to_string(),
            organization: None,
            project: None,
            model,
            custom_headers: None,
            supports_streaming: true,
            name: OPEN_AI_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            preserve_thinking_context: false,
        }
    }

    pub fn api_client(mut self, api_client: ApiClient) -> Self {
        self.api_client = api_client;
        self
    }

    pub fn base_path(mut self, base_path: impl Into<String>) -> Self {
        self.base_path = base_path.into();
        self
    }

    pub fn organization(mut self, organization: Option<String>) -> Self {
        self.organization = organization;
        self
    }

    pub fn project(mut self, project: Option<String>) -> Self {
        self.project = project;
        self
    }

    pub fn model(mut self, model: ModelConfig) -> Self {
        self.model = model;
        self
    }

    pub fn custom_headers(mut self, custom_headers: Option<HashMap<String, String>>) -> Self {
        self.custom_headers = custom_headers;
        self
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

    pub fn preserve_thinking_context(mut self, preserve_thinking_context: bool) -> Self {
        self.preserve_thinking_context = preserve_thinking_context;
        self
    }

    pub fn build(self) -> OpenAiProvider {
        OpenAiProvider {
            api_client: self.api_client,
            base_path: self.base_path,
            organization: self.organization,
            project: self.project,
            model: self.model,
            custom_headers: self.custom_headers,
            supports_streaming: self.supports_streaming,
            name: self.name,
            custom_models: self.custom_models,
            dynamic_models: self.dynamic_models,
            skip_canonical_filtering: self.skip_canonical_filtering,
            preserve_thinking_context: self.preserve_thinking_context,
        }
    }
}

impl OpenAiProvider {
    #[doc(hidden)]
    pub fn new(api_client: ApiClient, model: ModelConfig) -> Self {
        Self {
            api_client,
            base_path: OPEN_AI_DEFAULT_BASE_PATH.to_string(),
            organization: None,
            project: None,
            model,
            custom_headers: None,
            supports_streaming: true,
            name: OPEN_AI_PROVIDER_NAME.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            preserve_thinking_context: false,
        }
    }

    fn normalize_base_path(base_path: &str) -> String {
        if let Some(path) = base_path.strip_prefix('/') {
            format!("/{}", path.trim_end_matches('/'))
        } else {
            base_path.trim_end_matches('/').to_string()
        }
    }

    fn is_chat_completions_path(base_path: &str) -> bool {
        let normalized = Self::normalize_base_path(base_path).to_ascii_lowercase();
        normalized.contains("chat/completions")
    }

    fn is_responses_path(base_path: &str) -> bool {
        let normalized = Self::normalize_base_path(base_path).to_ascii_lowercase();
        normalized.ends_with("responses") || normalized.contains("/responses")
    }

    fn is_responses_model(model_name: &str) -> bool {
        is_openai_responses_model(model_name)
    }

    fn should_use_responses_api(model_name: &str, base_path: &str) -> bool {
        let normalized_base_path = Self::normalize_base_path(base_path);
        // Only the standard "v1/chat/completions" is treated as a default
        // path that defers to model-based routing.  The versionless
        // "chat/completions" (derived from an OPENAI_BASE_URL without /v1)
        // is treated as custom because versionless gateways typically do not
        // support the Responses API.
        let has_custom_base_path = normalized_base_path != OPEN_AI_DEFAULT_BASE_PATH;

        if has_custom_base_path {
            if Self::is_responses_path(&normalized_base_path) {
                return true;
            }
            if Self::is_chat_completions_path(&normalized_base_path) {
                return false;
            }
        }

        Self::is_responses_model(model_name)
    }

    /// Providers known to reject `max_completion_tokens` and require
    /// the legacy `max_tokens` field instead.
    const PROVIDERS_NEEDING_MAX_TOKENS_REMAP: &[&str] = &[
        "cerebras",
        "custom_deepseek",
        "groq",
        "inception",
        "kimi",
        "lmstudio",
        "mistral",
        "moonshot",
        "nearai",
        "ovhcloud",
    ];

    const PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS: &[&str] = &["nearai"];

    fn sanitize_request_for_compat(&self, mut payload: serde_json::Value) -> serde_json::Value {
        if let Some(obj) = payload.as_object_mut() {
            if Self::PROVIDERS_NEEDING_MAX_TOKENS_REMAP.contains(&self.name.as_str()) {
                if let Some(value) = obj.remove("max_completion_tokens") {
                    obj.entry("max_tokens").or_insert(value);
                }
            }

            if Self::PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS.contains(&self.name.as_str()) {
                let model_name = obj.get("model").and_then(|model| model.as_str());
                if !model_name.is_some_and(Self::is_responses_model) {
                    obj.remove("reasoning_effort");
                }

                if let Some(messages) = obj.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    for message in messages {
                        if message
                            .get("role")
                            .and_then(|role| role.as_str())
                            .is_some_and(|role| role == "developer")
                        {
                            message["role"] = serde_json::Value::String("system".to_string());
                        }
                    }
                }
            }
        }

        payload
    }

    fn should_use_responses_api_for_provider(&self, model_name: &str) -> bool {
        if Self::PROVIDERS_NEEDING_STANDARD_CHAT_PARAMS.contains(&self.name.as_str()) {
            return false;
        }

        Self::should_use_responses_api(model_name, &self.base_path)
    }

    fn map_base_path(base_path: &str, target: &str, fallback: &str) -> String {
        let normalized = Self::normalize_base_path(base_path);
        if normalized.ends_with(target) || normalized.contains(&format!("/{target}")) {
            return normalized;
        }

        if Self::is_chat_completions_path(&normalized) {
            return normalized.replacen("chat/completions", target, 1);
        }

        if Self::is_responses_path(&normalized) {
            return normalized.replacen("responses", target, 1);
        }

        if normalized.starts_with('/') {
            format!("/{}", fallback.trim_start_matches('/'))
        } else {
            fallback.to_string()
        }
    }

    /// Fill the model's context limit from the API when it isn't already set.
    ///
    /// An existing value may be an explicit GOOSE_CONTEXT_LIMIT, an ACP/server
    /// per-session override, or a GOOSE_PREDEFINED_MODELS entry, none of which we
    /// should overwrite. llama.cpp and Ollama report the real allocated window via
    /// the non-standard meta.n_ctx field; reading it fixes auto-compaction for local
    /// servers that would otherwise fall back to DEFAULT_CONTEXT_LIMIT. The probe is
    /// bounded by a short timeout so a hung /v1/models can't stall provider
    /// construction (the shared ApiClient uses OPENAI_TIMEOUT, up to 600s).
    pub async fn probe_context_limit_if_unset(&mut self) {
        if self.model.context_limit.is_some() {
            return;
        }
        const N_CTX_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
        let model_name = self.model.model_name.clone();
        if let Ok(Some(n_ctx)) =
            tokio::time::timeout(N_CTX_PROBE_TIMEOUT, self.fetch_n_ctx_from_api(&model_name)).await
        {
            self.model.context_limit = Some(n_ctx);
        }
    }

    async fn fetch_models_from_api(&self) -> Result<Vec<String>, ProviderError> {
        let models_path =
            Self::map_base_path(&self.base_path, "models", OPEN_AI_DEFAULT_MODELS_PATH);
        let response = self
            .api_client
            .request(None, &models_path)
            .response_get()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::EndpointNotFound(body));
        }

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
        Ok(models)
    }

    /// llama.cpp and Ollama expose the actual allocated context window in the
    /// non-standard `meta.n_ctx` field of `/v1/models`. Returns `None` when absent
    /// (e.g. real OpenAI).
    async fn fetch_n_ctx_from_api(&self, model_name: &str) -> Option<usize> {
        let models_path =
            Self::map_base_path(&self.base_path, "models", OPEN_AI_DEFAULT_MODELS_PATH);
        let response = self
            .api_client
            .request(None, &models_path)
            .response_get()
            .await
            .ok()?;
        let json = handle_response_openai_compat(response).await.ok()?;
        parse_n_ctx_from_models(&json, model_name)
    }
}

/// Extract `meta.n_ctx` for `model_name` from a `/v1/models` response body.
fn parse_n_ctx_from_models(json: &serde_json::Value, model_name: &str) -> Option<usize> {
    let data = json.get("data")?.as_array()?;

    let n_ctx = |entry: &serde_json::Value| -> Option<usize> {
        entry
            .get("meta")?
            .get("n_ctx")?
            .as_u64()
            .map(|v| v as usize)
    };

    if let Some(entry) = data
        .iter()
        .find(|e| e.get("id").and_then(|v| v.as_str()) == Some(model_name))
    {
        return n_ctx(entry);
    }

    // For single-model servers without --alias, llama.cpp reports the loaded model
    // file path as id rather than the client's alias, so no entry matches above.
    // Fall back to the sole entry's n_ctx.
    match data.as_slice() {
        [only] => n_ctx(only),
        _ => None,
    }
}

impl ProviderDescriptor for OpenAiProvider {
    fn metadata() -> ProviderMetadata {
        let models = OPEN_AI_KNOWN_MODELS
            .iter()
            .map(|(name, limit)| ModelInfo::new(*name, *limit))
            .collect();
        ProviderMetadata::with_models(
            OPEN_AI_PROVIDER_NAME,
            "OpenAI",
            "GPT-4 and other OpenAI models, including OpenAI compatible ones",
            OPEN_AI_DEFAULT_MODEL,
            models,
            OPEN_AI_DOC_URL,
            vec![
                ConfigKey::new("OPENAI_API_KEY", false, true, None, true),
                ConfigKey::new("OPENAI_BASE_URL", false, false, None, false),
                ConfigKey::new(
                    "OPENAI_HOST",
                    true,
                    false,
                    Some("https://api.openai.com"),
                    false,
                ),
                ConfigKey::new(
                    "OPENAI_BASE_PATH",
                    true,
                    false,
                    Some("v1/chat/completions"),
                    false,
                ),
                ConfigKey::new("OPENAI_ORGANIZATION", false, false, None, false),
                ConfigKey::new("OPENAI_PROJECT", false, false, None, false),
                ConfigKey::new("OPENAI_CUSTOM_HEADERS", false, true, None, false),
                ConfigKey::new("OPENAI_TIMEOUT", false, false, Some("600"), false),
            ],
        )
        .with_setup_steps(vec![
            "Go to https://platform.openai.com and sign up or log in",
            "Navigate to API Keys in the left sidebar",
            "Click 'Create new secret key'",
            "Copy the key and paste it above",
        ])
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn skip_canonical_filtering(&self) -> bool {
        self.skip_canonical_filtering
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
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
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        if self.should_use_responses_api_for_provider(&model_config.model_name) {
            let mut payload = create_responses_request(model_config, system, messages, tools)?;
            payload["stream"] = serde_json::Value::Bool(self.supports_streaming);

            let mut log = start_log(model_config, &payload)?;

            let response = self
                .with_retry(|| async {
                    let payload_clone = payload.clone();
                    let resp = self
                        .api_client
                        .response_post(
                            Some(session_id),
                            &Self::map_base_path(
                                &self.base_path,
                                "responses",
                                OPEN_AI_DEFAULT_RESPONSES_PATH,
                            ),
                            &payload_clone,
                        )
                        .await?;
                    handle_status(resp).await
                })
                .await
                .inspect_err(|e| {
                    let _ = log.error(e);
                })?;

            if self.supports_streaming {
                stream_responses_compat(response, log)
            } else {
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    ProviderError::RequestFailed(format!("Failed to parse JSON: {}", e))
                })?;

                let responses_api_response: ResponsesApiResponse =
                    serde_json::from_value(json.clone()).map_err(|e| {
                        ProviderError::ExecutionError(format!(
                            "Failed to parse responses API response: {}",
                            e
                        ))
                    })?;

                let message = responses_api_to_message(&responses_api_response)?;
                let usage_data = get_responses_usage(&responses_api_response);
                let usage = ProviderUsage::new(model_config.model_name.clone(), usage_data);

                log.write(
                    &serde_json::to_value(&message).unwrap_or_default(),
                    Some(&usage_data),
                )?;

                Ok(super::base::stream_from_single_message(message, usage))
            }
        } else {
            let payload = create_request_with_options(
                model_config,
                system,
                messages,
                tools,
                &ImageFormat::OpenAi,
                self.supports_streaming,
                OpenAiFormatOptions {
                    preserve_thinking_context: self.preserve_thinking_context,
                },
            )?;
            let payload = self.sanitize_request_for_compat(payload);
            let mut log = start_log(model_config, &payload)?;

            let response = self
                .with_retry(|| async {
                    let resp = self
                        .api_client
                        .response_post(Some(session_id), &self.base_path, &payload)
                        .await?;
                    handle_status(resp).await
                })
                .await
                .inspect_err(|e| {
                    let _ = log.error(e);
                })?;

            if self.supports_streaming {
                stream_openai_compat(response, log)
            } else {
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    ProviderError::RequestFailed(format!("Failed to parse JSON: {}", e))
                })?;

                let message = response_to_message(&json).map_err(|e| {
                    ProviderError::RequestFailed(format!("Failed to parse message: {}", e))
                })?;

                let usage_data = get_usage(json.get("usage").unwrap_or(&serde_json::Value::Null));
                let usage = ProviderUsage::new(model_config.model_name.clone(), usage_data);

                log.write(
                    &serde_json::to_value(&message).unwrap_or_default(),
                    Some(&usage_data),
                )?;

                Ok(super::base::stream_from_single_message(message, usage))
            }
        }
    }
}

pub fn parse_custom_headers(s: String) -> HashMap<String, String> {
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
    use super::*;
    use crate::api_client::AuthMethod;
    use serde_json::json;

    fn make_provider(name: &str) -> OpenAiProvider {
        OpenAiProvider {
            api_client: ApiClient::new_with_tls(
                "http://localhost".to_string(),
                AuthMethod::NoAuth,
                None,
            )
            .unwrap(),
            base_path: "v1/chat/completions".to_string(),
            organization: None,
            project: None,
            model: ModelConfig::new_or_fail("test-model"),
            custom_headers: None,
            supports_streaming: true,
            name: name.to_string(),
            custom_models: None,
            dynamic_models: None,
            skip_canonical_filtering: false,
            preserve_thinking_context: false,
        }
    }

    #[test]
    fn sanitize_remaps_max_completion_tokens_for_compat_provider() {
        let provider = make_provider("mistral");
        let payload = json!({
            "model": "mistral-medium-latest",
            "messages": [],
            "max_completion_tokens": 16384
        });

        let result = provider.sanitize_request_for_compat(payload);
        let obj = result.as_object().unwrap();

        assert!(!obj.contains_key("max_completion_tokens"));
        assert_eq!(obj.get("max_tokens").unwrap(), &json!(16384));
    }

    #[test]
    fn sanitize_preserves_existing_max_tokens_for_compat_provider() {
        let provider = make_provider("mistral");
        let payload = json!({
            "model": "mistral-medium-latest",
            "messages": [],
            "max_tokens": 4096,
            "max_completion_tokens": 16384
        });

        let result = provider.sanitize_request_for_compat(payload);
        let obj = result.as_object().unwrap();

        assert!(!obj.contains_key("max_completion_tokens"));
        assert_eq!(obj.get("max_tokens").unwrap(), &json!(4096));
    }

    #[test]
    fn sanitize_noop_for_native_openai_provider() {
        let provider = make_provider("openai");
        let payload = json!({
            "model": "o3",
            "messages": [],
            "max_completion_tokens": 16384
        });

        let result = provider.sanitize_request_for_compat(payload);
        let obj = result.as_object().unwrap();

        assert!(obj.contains_key("max_completion_tokens"));
        assert!(!obj.contains_key("max_tokens"));
    }

    #[test]
    fn sanitize_noop_for_unknown_provider() {
        let provider = make_provider("some_future_provider");
        let payload = json!({
            "model": "future-model",
            "messages": [],
            "max_completion_tokens": 16384
        });

        let result = provider.sanitize_request_for_compat(payload);
        let obj = result.as_object().unwrap();

        assert!(obj.contains_key("max_completion_tokens"));
        assert!(!obj.contains_key("max_tokens"));
    }

    #[test]
    fn sanitize_no_token_params() {
        let provider = make_provider("groq");
        let payload = json!({
            "model": "llama-3.3-70b-versatile",
            "messages": []
        });

        let result = provider.sanitize_request_for_compat(payload.clone());
        assert_eq!(result, payload);
    }

    #[test]
    fn sanitize_nearai_reasoning_chat_params() {
        let provider = make_provider("nearai");
        let payload = json!({
            "model": "Qwen/Qwen3.6-35B-A3B-FP8",
            "messages": [
                {
                    "role": "developer",
                    "content": "system instructions"
                },
                {
                    "role": "user",
                    "content": "hello"
                }
            ],
            "reasoning_effort": "medium",
            "max_completion_tokens": 16384
        });

        let result = provider.sanitize_request_for_compat(payload);
        let obj = result.as_object().unwrap();

        assert!(!obj.contains_key("reasoning_effort"));
        assert!(!obj.contains_key("max_completion_tokens"));
        assert_eq!(obj.get("max_tokens").unwrap(), &json!(16384));
        assert_eq!(obj["messages"][0]["role"], "system");
        assert_eq!(obj["messages"][1]["role"], "user");
    }

    #[test]
    fn sanitize_nearai_preserves_openai_reasoning_effort() {
        let provider = make_provider("nearai");
        let payload = json!({
            "model": "openai/gpt-5",
            "messages": [],
            "reasoning_effort": "medium",
            "max_completion_tokens": 16384
        });

        let result = provider.sanitize_request_for_compat(payload);
        let obj = result.as_object().unwrap();

        assert_eq!(obj.get("reasoning_effort"), Some(&json!("medium")));
        assert!(!obj.contains_key("max_completion_tokens"));
        assert_eq!(obj.get("max_tokens").unwrap(), &json!(16384));
    }

    #[test]
    fn nearai_uses_chat_completions_for_openai_reasoning_models() {
        let provider = make_provider("nearai");

        assert!(!provider.should_use_responses_api_for_provider("openai/gpt-5"));
        assert!(!provider.should_use_responses_api_for_provider("openai/o3"));
    }

    #[test]
    fn responses_api_routing_uses_model_family_unless_path_forces_chat() {
        for (model_name, base_path, expected) in [
            ("gpt-5.4", "v1/chat/completions", true),
            ("gpt-5.4-xhigh", "v1/chat/completions", true),
            ("gpt-5.2-pro-2025-12-11", "v1/chat/completions", true),
            ("gpt-4o", "v1/chat/completions", false),
            ("gpt-5.2-codex", "openai/v1/chat/completions", false),
        ] {
            assert_eq!(
                OpenAiProvider::should_use_responses_api(model_name, base_path),
                expected,
                "unexpected routing for {model_name} via {base_path}"
            );
        }
    }

    #[test]
    fn custom_chat_path_maps_to_responses_path() {
        let responses_path = OpenAiProvider::map_base_path(
            "openai/v1/chat/completions",
            "responses",
            "v1/responses",
        );
        assert_eq!(responses_path, "openai/v1/responses");
    }

    #[test]
    fn responses_path_maps_to_models_path() {
        let models_path =
            OpenAiProvider::map_base_path("openai/v1/responses", "models", "v1/models");
        assert_eq!(models_path, "openai/v1/models");
    }

    #[test]
    fn unknown_path_falls_back_to_default_models_path() {
        let models_path = OpenAiProvider::map_base_path("custom/path", "models", "v1/models");
        assert_eq!(models_path, "v1/models");
    }

    #[test]
    fn absolute_chat_path_maps_to_absolute_responses_path() {
        let responses_path =
            OpenAiProvider::map_base_path("/v1/chat/completions", "responses", "v1/responses");
        assert_eq!(responses_path, "/v1/responses");
    }

    #[test]
    fn unknown_absolute_path_falls_back_to_absolute_models_path() {
        let models_path = OpenAiProvider::map_base_path("/custom/path", "models", "v1/models");
        assert_eq!(models_path, "/v1/models");
    }
    #[test]
    fn versionless_base_path_opts_out_of_responses_for_codex_models() {
        assert!(!OpenAiProvider::should_use_responses_api(
            "gpt-5-codex",
            "chat/completions"
        ));
    }

    #[test]
    fn ensure_url_scheme_adds_http_for_local_hosts() {
        assert_eq!(ensure_url_scheme("localhost:1234"), "http://localhost:1234");
        assert_eq!(
            ensure_url_scheme("127.0.0.1:8080/v1"),
            "http://127.0.0.1:8080/v1"
        );
        assert_eq!(ensure_url_scheme("0.0.0.0:3000"), "http://0.0.0.0:3000");
        assert_eq!(ensure_url_scheme("[::1]:1234"), "http://[::1]:1234");
    }

    #[test]
    fn ensure_url_scheme_adds_https_for_remote_hosts() {
        assert_eq!(
            ensure_url_scheme("api.example.com:8443/v1"),
            "https://api.example.com:8443/v1"
        );
        assert_eq!(ensure_url_scheme("example.com"), "https://example.com");
    }

    #[test]
    fn ensure_url_scheme_preserves_existing_scheme() {
        assert_eq!(
            ensure_url_scheme("http://localhost:1234"),
            "http://localhost:1234"
        );
        assert_eq!(
            ensure_url_scheme("https://api.openai.com/v1"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn parse_n_ctx_falls_back_to_sole_entry_when_id_differs() {
        let body = json!({
            "data": [
                { "id": "/models/qwen3.gguf", "meta": { "n_ctx": 32768 } }
            ]
        });
        assert_eq!(parse_n_ctx_from_models(&body, "qwen3"), Some(32768));
    }

    #[test]
    fn parse_n_ctx_no_fallback_with_multiple_unmatched_entries() {
        let body = json!({
            "data": [
                { "id": "model-a", "meta": { "n_ctx": 4096 } },
                { "id": "model-b", "meta": { "n_ctx": 8192 } }
            ]
        });
        assert_eq!(parse_n_ctx_from_models(&body, "model-c"), None);
    }
}
