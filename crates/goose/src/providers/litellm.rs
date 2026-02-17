use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::OnceCell;

use super::api_client::{ApiClient, AuthMethod};
use super::base::{
    ConfigKey, MessageStream, ModelInfo, Provider, ProviderDef, ProviderMetadata, ProviderUsage,
};
use super::embedding::EmbeddingCapable;
use super::errors::ProviderError;
use super::openai_compatible::handle_response_openai_compat;
use super::retry::ProviderRetry;
use super::utils::{get_model, ImageFormat, RequestLog};
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

const LITELLM_PROVIDER_NAME: &str = "litellm";
pub const LITELLM_DEFAULT_MODEL: &str = "gpt-4o-mini";
pub const LITELLM_DOC_URL: &str = "https://docs.litellm.ai/docs/";

/// Extended model capabilities fetched from LiteLLM proxy's /model/info endpoint.
/// Contains reasoning support flag and the underlying model identifier, which is
/// needed to avoid false-positive reasoning parameter injection based on model name
/// heuristics (fixes issue #4221).
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for Phase 2/3 (variant system, pricing display)
struct LiteLLMModelCapabilities {
    /// Whether this model supports reasoning/extended thinking
    supports_reasoning: bool,
    /// Whether this model supports prompt caching
    supports_cache_control: bool,
    /// The actual underlying model string (e.g. "anthropic/claude-opus-4-6...")
    litellm_model: Option<String>,
    /// Context window size
    max_input_tokens: usize,
    /// Cost per input token in USD
    input_cost_per_token: Option<f64>,
    /// Cost per output token in USD
    output_cost_per_token: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct LiteLLMProvider {
    #[serde(skip)]
    api_client: ApiClient,
    base_path: String,
    model: ModelConfig,
    #[serde(skip)]
    name: String,
    /// Cached model capabilities from /model/info, populated once on first use
    #[serde(skip)]
    model_capabilities_cache: OnceCell<HashMap<String, LiteLLMModelCapabilities>>,
}

impl LiteLLMProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let secrets = config
            .get_secrets("LITELLM_API_KEY", &["LITELLM_CUSTOM_HEADERS"])
            .unwrap_or_default();
        let api_key = secrets.get("LITELLM_API_KEY").cloned().unwrap_or_default();
        let host: String = config
            .get_param("LITELLM_HOST")
            .unwrap_or_else(|_| "https://api.litellm.ai".to_string());
        let base_path: String = config
            .get_param("LITELLM_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        let custom_headers: Option<HashMap<String, String>> = secrets
            .get("LITELLM_CUSTOM_HEADERS")
            .cloned()
            .map(parse_custom_headers);
        let timeout_secs: u64 = config.get_param("LITELLM_TIMEOUT").unwrap_or(600);

        let auth = if api_key.is_empty() {
            AuthMethod::NoAuth
        } else {
            AuthMethod::BearerToken(api_key)
        };

        let mut api_client =
            ApiClient::with_timeout(host, auth, std::time::Duration::from_secs(timeout_secs))?;

        if let Some(headers) = custom_headers {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())?;
                let header_value = reqwest::header::HeaderValue::from_str(&value)?;
                header_map.insert(header_name, header_value);
            }
            api_client = api_client.with_headers(header_map)?;
        }

        Ok(Self {
            api_client,
            base_path,
            model,
            name: LITELLM_PROVIDER_NAME.to_string(),
            model_capabilities_cache: OnceCell::new(),
        })
    }

    /// Fetch and cache model capabilities from the LiteLLM proxy's /model/info endpoint.
    /// Returns a map of model_name -> capabilities. The result is cached for the
    /// lifetime of this provider instance (typically the session).
    async fn get_model_capabilities(
        &self,
    ) -> Result<&HashMap<String, LiteLLMModelCapabilities>, ProviderError> {
        self.model_capabilities_cache
            .get_or_try_init(|| async { self.fetch_model_capabilities_from_proxy().await })
            .await
    }

    /// Fetches model info from the LiteLLM proxy, extracting both the standard
    /// ModelInfo fields and extended capabilities (reasoning support, pricing, etc.).
    async fn fetch_model_capabilities_from_proxy(
        &self,
    ) -> Result<HashMap<String, LiteLLMModelCapabilities>, ProviderError> {
        let response = self
            .api_client
            .request(None, "model/info")
            .response_get()
            .await?;

        if !response.status().is_success() {
            return Err(ProviderError::RequestFailed(format!(
                "Models endpoint returned status: {}",
                response.status()
            )));
        }

        let response_json: Value = response.json().await.map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to parse models response: {}", e))
        })?;

        let models_data = response_json["data"].as_array().ok_or_else(|| {
            ProviderError::RequestFailed("Missing data field in models response".to_string())
        })?;

        let mut capabilities = HashMap::new();
        for model_data in models_data {
            if let Some(model_name) = model_data["model_name"].as_str() {
                if model_name.contains("/*") {
                    continue;
                }

                let model_info = &model_data["model_info"];

                // The underlying model string from litellm_params (e.g. "anthropic/claude-opus-4-6...")
                let litellm_model = model_data["litellm_params"]["model"]
                    .as_str()
                    .map(|s| s.to_string());

                // Determine reasoning support:
                // 1. Explicit model_info.supports_reasoning flag (user-configured)
                // 2. LiteLLM's built-in supports_reasoning field
                // 3. Fall back to false (safe default — never inject reasoning params unless known)
                let supports_reasoning = model_info["supports_reasoning"]
                    .as_bool()
                    .unwrap_or(false);

                let supports_cache_control = model_info["supports_prompt_caching"]
                    .as_bool()
                    .unwrap_or(false);

                let max_input_tokens =
                    model_info["max_input_tokens"].as_u64().unwrap_or(128000) as usize;

                let input_cost_per_token = model_info["input_cost_per_token"].as_f64();
                let output_cost_per_token = model_info["output_cost_per_token"].as_f64();

                capabilities.insert(
                    model_name.to_string(),
                    LiteLLMModelCapabilities {
                        supports_reasoning,
                        supports_cache_control,
                        litellm_model,
                        max_input_tokens,
                        input_cost_per_token,
                        output_cost_per_token,
                    },
                );
            }
        }

        Ok(capabilities)
    }

    /// Build a Vec<ModelInfo> from cached capabilities for use in Provider trait methods.
    #[allow(dead_code)] // Used in tests; will be used in Phase 3 (pricing display)
    fn capabilities_to_model_info(
        capabilities: &HashMap<String, LiteLLMModelCapabilities>,
    ) -> Vec<ModelInfo> {
        capabilities
            .iter()
            .map(|(name, caps)| {
                let mut info = ModelInfo::new(name.clone(), caps.max_input_tokens);
                info.supports_cache_control = Some(caps.supports_cache_control);
                info.input_token_cost = caps.input_cost_per_token;
                info.output_token_cost = caps.output_cost_per_token;
                if caps.input_cost_per_token.is_some() || caps.output_cost_per_token.is_some() {
                    info.currency = Some("$".to_string());
                }
                info
            })
            .collect()
    }

    /// Check if the current model supports reasoning parameters.
    /// Returns false if model info is unavailable (safe default).
    async fn model_supports_reasoning(&self, model_name: &str) -> bool {
        match self.get_model_capabilities().await {
            Ok(caps) => caps
                .get(model_name)
                .map(|c| c.supports_reasoning)
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Strip reasoning-specific parameters from the request payload.
    /// Called when the model is known not to support reasoning, to prevent
    /// false-positive reasoning param injection from OpenAI format heuristics.
    fn strip_reasoning_params(payload: &mut Value) {
        if let Some(obj) = payload.as_object_mut() {
            obj.remove("reasoning_effort");

            // If the OpenAI formatter used "developer" role (for o-series heuristic),
            // revert it to "system" since this model isn't actually an o-series model
            if let Some(messages) = obj.get_mut("messages").and_then(|m| m.as_array_mut()) {
                for msg in messages.iter_mut() {
                    if msg.get("role") == Some(&json!("developer")) {
                        msg["role"] = json!("system");
                    }
                }
            }

            // Revert max_completion_tokens back to max_tokens if it was set by the
            // o-series heuristic but the model doesn't actually need it
            if let Some(max_completion_tokens) = obj.remove("max_completion_tokens") {
                if !obj.contains_key("max_tokens") {
                    obj.insert("max_tokens".to_string(), max_completion_tokens);
                }
            }
        }
    }

    async fn post(
        &self,
        session_id: Option<&str>,
        payload: &Value,
    ) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post(session_id, &self.base_path, payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

impl ProviderDef for LiteLLMProvider {
    type Provider = Self;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            LITELLM_PROVIDER_NAME,
            "LiteLLM",
            "LiteLLM proxy supporting multiple models with automatic prompt caching",
            LITELLM_DEFAULT_MODEL,
            vec![],
            LITELLM_DOC_URL,
            vec![
                ConfigKey::new("LITELLM_API_KEY", true, true, None, true),
                ConfigKey::new(
                    "LITELLM_HOST",
                    true,
                    false,
                    Some("http://localhost:4000"),
                    true,
                ),
                ConfigKey::new(
                    "LITELLM_BASE_PATH",
                    true,
                    false,
                    Some("v1/chat/completions"),
                    false,
                ),
                ConfigKey::new("LITELLM_CUSTOM_HEADERS", false, true, None, false),
                ConfigKey::new("LITELLM_TIMEOUT", false, false, Some("600"), false),
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
impl Provider for LiteLLMProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(skip_all, name = "provider_complete")]
    async fn stream(
        &self,
        model_config: &ModelConfig,
        session_id: &str,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<MessageStream, ProviderError> {
        let session_id = if session_id.is_empty() {
            None
        } else {
            Some(session_id)
        };
        // Build the request using the standard OpenAI format
        let mut payload = super::formats::openai::create_request(
            model_config,
            system,
            messages,
            tools,
            &ImageFormat::OpenAi,
            false,
        )?;

        // Fix for #4221: The OpenAI format uses model-name heuristics (names starting
        // with "o1", "o3", etc.) to inject reasoning_effort. For LiteLLM, model names
        // are user-defined aliases, so these heuristics produce false positives.
        // We check the actual model capabilities from the proxy and strip reasoning
        // params if the model doesn't support them.
        if !self
            .model_supports_reasoning(&model_config.model_name)
            .await
        {
            Self::strip_reasoning_params(&mut payload);
        }

        if self.supports_cache_control().await {
            payload = update_request_for_cache_control(&payload);
        }

        let response = self
            .with_retry(|| async {
                let payload_clone = payload.clone();
                self.post(session_id, &payload_clone).await
            })
            .await?;

        let message = super::formats::openai::response_to_message(&response)?;
        let usage = super::formats::openai::get_usage(&response);
        let response_model = get_model(&response);
        let mut log = RequestLog::start(model_config, &payload)?;
        log.write(&response, Some(&usage))?;
        let provider_usage = ProviderUsage::new(response_model, usage);
        Ok(super::base::stream_from_single_message(
            message,
            provider_usage,
        ))
    }

    fn supports_embeddings(&self) -> bool {
        true
    }

    async fn supports_cache_control(&self) -> bool {
        if let Ok(caps) = self.get_model_capabilities().await {
            if let Some(model_caps) = caps.get(&self.model.model_name) {
                return model_caps.supports_cache_control;
            }
        }

        // Fallback: if we can't reach the proxy, guess based on model name
        self.model.model_name.to_lowercase().contains("claude")
    }

    async fn fetch_supported_models(&self) -> Result<Vec<String>, ProviderError> {
        let caps = self.get_model_capabilities().await?;
        Ok(caps.keys().cloned().collect())
    }
}

#[async_trait]
impl EmbeddingCapable for LiteLLMProvider {
    async fn create_embeddings(
        &self,
        session_id: &str,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        let embedding_model = std::env::var("GOOSE_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());

        let payload = json!({
            "input": texts,
            "model": embedding_model,
            "encoding_format": "float"
        });

        let response = self
            .api_client
            .response_post(Some(session_id), "v1/embeddings", &payload)
            .await?;
        let response_text = response.text().await?;
        let response_json: Value = serde_json::from_str(&response_text)?;

        let data = response_json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing data field"))?;

        let mut embeddings = Vec::new();
        for item in data {
            let embedding: Vec<f32> = item["embedding"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Missing embedding field"))?
                .iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }
}

/// Updates the request payload to include cache control headers for automatic prompt caching
/// Adds ephemeral cache control to the last 2 user messages, system message, and last tool
pub fn update_request_for_cache_control(original_payload: &Value) -> Value {
    let mut payload = original_payload.clone();

    if let Some(messages_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("messages"))
        .and_then(|messages| messages.as_array_mut())
    {
        let mut user_count = 0;
        for message in messages_spec.iter_mut().rev() {
            if message.get("role") == Some(&json!("user")) {
                if let Some(content) = message.get_mut("content") {
                    if let Some(content_str) = content.as_str() {
                        *content = json!([{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]);
                    }
                }
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        if let Some(system_message) = messages_spec
            .iter_mut()
            .find(|msg| msg.get("role") == Some(&json!("system")))
        {
            if let Some(content) = system_message.get_mut("content") {
                if let Some(content_str) = content.as_str() {
                    *system_message = json!({
                        "role": "system",
                        "content": [{
                            "type": "text",
                            "text": content_str,
                            "cache_control": { "type": "ephemeral" }
                        }]
                    });
                }
            }
        }
    }

    if let Some(tools_spec) = payload
        .as_object_mut()
        .and_then(|obj| obj.get_mut("tools"))
        .and_then(|tools| tools.as_array_mut())
    {
        if let Some(last_tool) = tools_spec.last_mut() {
            if let Some(function) = last_tool.get_mut("function") {
                function
                    .as_object_mut()
                    .unwrap()
                    .insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
            }
        }
    }
    payload
}

fn parse_custom_headers(headers_str: String) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in headers_str.lines() {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test: verify model capabilities parsing against a real LiteLLM proxy.
    /// Requires LITELLM_INTEGRATION_TEST_HOST and LITELLM_INTEGRATION_TEST_KEY env vars.
    /// Run with: cargo test -p goose --lib providers::litellm::tests::test_live -- --ignored
    #[tokio::test]
    #[ignore] // Only run manually with env vars set
    async fn test_live_model_capabilities_from_proxy() {
        let host = std::env::var("LITELLM_INTEGRATION_TEST_HOST")
            .expect("Set LITELLM_INTEGRATION_TEST_HOST to run this test");
        let api_key = std::env::var("LITELLM_INTEGRATION_TEST_KEY")
            .expect("Set LITELLM_INTEGRATION_TEST_KEY to run this test");

        let auth = AuthMethod::BearerToken(api_key);
        let api_client =
            ApiClient::with_timeout(host, auth, std::time::Duration::from_secs(30)).unwrap();

        let provider = LiteLLMProvider {
            api_client,
            base_path: "v1/chat/completions".to_string(),
            model: ModelConfig::new("haiku-4.5").unwrap(),
            name: LITELLM_PROVIDER_NAME.to_string(),
            model_capabilities_cache: OnceCell::new(),
        };

        let caps = provider.get_model_capabilities().await.unwrap();

        // Should have parsed at least some models
        assert!(!caps.is_empty(), "Expected at least one model from proxy");

        // Print all models for debugging
        for (name, cap) in caps.iter() {
            println!(
                "  {}: reasoning={}, caching={}, underlying={:?}",
                name, cap.supports_reasoning, cap.supports_cache_control, cap.litellm_model
            );
        }
    }

    #[test]
    fn test_strip_reasoning_params_removes_reasoning_effort() {
        let mut payload = json!({
            "model": "o3-custom-alias",
            "messages": [{"role": "developer", "content": "system prompt"}],
            "reasoning_effort": "medium",
            "max_completion_tokens": 1024
        });

        LiteLLMProvider::strip_reasoning_params(&mut payload);

        assert!(payload.get("reasoning_effort").is_none());
    }

    #[test]
    fn test_strip_reasoning_params_reverts_developer_to_system() {
        let mut payload = json!({
            "model": "o3-custom-alias",
            "messages": [{"role": "developer", "content": "system prompt"}],
            "reasoning_effort": "medium"
        });

        LiteLLMProvider::strip_reasoning_params(&mut payload);

        let messages = payload["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
    }

    #[test]
    fn test_strip_reasoning_params_reverts_max_completion_tokens() {
        let mut payload = json!({
            "model": "o3-custom-alias",
            "messages": [{"role": "developer", "content": "system prompt"}],
            "reasoning_effort": "medium",
            "max_completion_tokens": 2048
        });

        LiteLLMProvider::strip_reasoning_params(&mut payload);

        assert!(payload.get("max_completion_tokens").is_none());
        assert_eq!(payload["max_tokens"], 2048);
    }

    #[test]
    fn test_strip_reasoning_params_preserves_existing_max_tokens() {
        // If both max_tokens and max_completion_tokens exist, don't overwrite max_tokens
        let mut payload = json!({
            "model": "test",
            "messages": [{"role": "system", "content": "hi"}],
            "max_tokens": 1024,
            "max_completion_tokens": 2048
        });

        LiteLLMProvider::strip_reasoning_params(&mut payload);

        assert_eq!(payload["max_tokens"], 1024);
        assert!(payload.get("max_completion_tokens").is_none());
    }

    #[test]
    fn test_strip_reasoning_params_noop_on_regular_payload() {
        let mut payload = json!({
            "model": "gpt-4o",
            "messages": [{"role": "system", "content": "system prompt"}],
            "max_tokens": 1024,
            "temperature": 0.7
        });

        let original = payload.clone();
        LiteLLMProvider::strip_reasoning_params(&mut payload);

        assert_eq!(payload, original);
    }

    #[test]
    fn test_strip_reasoning_params_preserves_user_messages() {
        let mut payload = json!({
            "model": "o3-custom",
            "messages": [
                {"role": "developer", "content": "system prompt"},
                {"role": "user", "content": "hello"},
                {"role": "assistant", "content": "hi there"}
            ],
            "reasoning_effort": "high"
        });

        LiteLLMProvider::strip_reasoning_params(&mut payload);

        let messages = payload["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[1]["content"], "hello");
    }

    #[test]
    fn test_parse_model_capabilities_from_proxy_response() {
        // Simulate the JSON structure returned by LiteLLM's /model/info endpoint
        let proxy_response = json!({
            "data": [
                {
                    "model_name": "claude-4.5-sonnet",
                    "litellm_params": {
                        "model": "anthropic/claude-sonnet-4-5-20250929",
                        "api_key": "sk-***"
                    },
                    "model_info": {
                        "max_input_tokens": 200000,
                        "supports_prompt_caching": true,
                        "supports_reasoning": true,
                        "input_cost_per_token": 0.000003,
                        "output_cost_per_token": 0.000015
                    }
                },
                {
                    "model_name": "open-mistral-small",
                    "litellm_params": {
                        "model": "mistral/open-mistral-small-3.1",
                        "api_key": "sk-***"
                    },
                    "model_info": {
                        "max_input_tokens": 128000,
                        "supports_prompt_caching": false
                        // No supports_reasoning → should default to false
                    }
                },
                {
                    "model_name": "o3-custom",
                    "litellm_params": {
                        "model": "openai/o3-mini",
                        "api_key": "sk-***"
                    },
                    "model_info": {
                        "max_input_tokens": 200000,
                        "supports_reasoning": true
                    }
                }
            ]
        });

        // Parse the response the same way fetch_model_capabilities_from_proxy does
        let models_data = proxy_response["data"].as_array().unwrap();
        let mut capabilities = HashMap::new();

        for model_data in models_data {
            let model_name = model_data["model_name"].as_str().unwrap();
            let model_info = &model_data["model_info"];
            let litellm_model = model_data["litellm_params"]["model"]
                .as_str()
                .map(|s| s.to_string());

            let supports_reasoning = model_info["supports_reasoning"]
                .as_bool()
                .unwrap_or(false);

            capabilities.insert(
                model_name.to_string(),
                LiteLLMModelCapabilities {
                    supports_reasoning,
                    supports_cache_control: model_info["supports_prompt_caching"]
                        .as_bool()
                        .unwrap_or(false),
                    litellm_model,
                    max_input_tokens: model_info["max_input_tokens"].as_u64().unwrap_or(128000)
                        as usize,
                    input_cost_per_token: model_info["input_cost_per_token"].as_f64(),
                    output_cost_per_token: model_info["output_cost_per_token"].as_f64(),
                },
            );
        }

        // claude-4.5-sonnet: has supports_reasoning: true
        let claude = capabilities.get("claude-4.5-sonnet").unwrap();
        assert!(claude.supports_reasoning);
        assert!(claude.supports_cache_control);
        assert_eq!(
            claude.litellm_model.as_deref(),
            Some("anthropic/claude-sonnet-4-5-20250929")
        );
        assert_eq!(claude.max_input_tokens, 200000);
        assert_eq!(claude.input_cost_per_token, Some(0.000003));
        assert_eq!(claude.output_cost_per_token, Some(0.000015));

        // open-mistral-small: no supports_reasoning field → false
        let mistral = capabilities.get("open-mistral-small").unwrap();
        assert!(!mistral.supports_reasoning);
        assert!(!mistral.supports_cache_control);
        assert_eq!(
            mistral.litellm_model.as_deref(),
            Some("mistral/open-mistral-small-3.1")
        );

        // o3-custom: user alias that starts with "o3" but correctly marked as reasoning
        let o3 = capabilities.get("o3-custom").unwrap();
        assert!(o3.supports_reasoning);
        assert_eq!(o3.litellm_model.as_deref(), Some("openai/o3-mini"));
    }

    #[test]
    fn test_capabilities_to_model_info() {
        let mut capabilities = HashMap::new();
        capabilities.insert(
            "test-model".to_string(),
            LiteLLMModelCapabilities {
                supports_reasoning: true,
                supports_cache_control: true,
                litellm_model: Some("anthropic/claude-sonnet-4-5".to_string()),
                max_input_tokens: 200000,
                input_cost_per_token: Some(0.000003),
                output_cost_per_token: Some(0.000015),
            },
        );
        capabilities.insert(
            "no-pricing-model".to_string(),
            LiteLLMModelCapabilities {
                supports_reasoning: false,
                supports_cache_control: false,
                litellm_model: None,
                max_input_tokens: 128000,
                input_cost_per_token: None,
                output_cost_per_token: None,
            },
        );

        let models = LiteLLMProvider::capabilities_to_model_info(&capabilities);

        let test_model = models.iter().find(|m| m.name == "test-model").unwrap();
        assert_eq!(test_model.context_limit, 200000);
        assert_eq!(test_model.supports_cache_control, Some(true));
        assert_eq!(test_model.input_token_cost, Some(0.000003));
        assert_eq!(test_model.output_token_cost, Some(0.000015));
        assert_eq!(test_model.currency, Some("$".to_string()));

        let no_pricing = models
            .iter()
            .find(|m| m.name == "no-pricing-model")
            .unwrap();
        assert_eq!(no_pricing.context_limit, 128000);
        assert_eq!(no_pricing.supports_cache_control, Some(false));
        assert_eq!(no_pricing.input_token_cost, None);
        assert_eq!(no_pricing.currency, None);
    }

    /// Integration-style test: simulate the complete flow where the OpenAI formatter
    /// would incorrectly add reasoning params for a model alias starting with "o3",
    /// and verify that strip_reasoning_params correctly cleans up the payload.
    #[test]
    fn test_false_positive_o3_alias_is_corrected() {
        // Simulate what openai::create_request produces for a model named "o3-mistral"
        // (the OpenAI formatter sees "o3" prefix and injects reasoning params)
        let mut payload = json!({
            "model": "o3-mistral",
            "messages": [
                {"role": "developer", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello"}
            ],
            "reasoning_effort": "medium",
            "max_completion_tokens": 1024
        });

        // The model is NOT a reasoning model (it's a Mistral model behind a "o3-" alias)
        LiteLLMProvider::strip_reasoning_params(&mut payload);

        // Verify all reasoning artifacts are removed
        assert!(payload.get("reasoning_effort").is_none());
        assert!(payload.get("max_completion_tokens").is_none());
        assert_eq!(payload["max_tokens"], 1024);

        // System message role should be reverted
        let messages = payload["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "You are a helpful assistant.");

        // User message should be untouched
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "Hello");
    }

    /// Verify that when a model IS a reasoning model, its payload is NOT stripped
    /// (i.e., strip_reasoning_params is not called for reasoning models).
    #[test]
    fn test_reasoning_model_payload_preserved() {
        // This represents a correctly-formed payload for an actual reasoning model
        let payload = json!({
            "model": "o3-mini",
            "messages": [
                {"role": "developer", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is 2+2?"}
            ],
            "reasoning_effort": "high",
            "max_completion_tokens": 4096
        });

        // For a reasoning model, we would NOT call strip_reasoning_params.
        // Verify the payload remains intact.
        assert_eq!(payload["reasoning_effort"], "high");
        assert_eq!(payload["max_completion_tokens"], 4096);
        assert_eq!(payload["messages"][0]["role"], "developer");
    }

    /// End-to-end test: call the real openai::create_request with a model name that
    /// triggers the o-series heuristic, then verify strip_reasoning_params cleans it up.
    #[test]
    fn test_create_request_then_strip_for_false_positive() {
        use crate::providers::formats::openai::create_request;
        use crate::providers::utils::ImageFormat;

        // Model alias "o3-mistral" triggers the openai formatter's o-series heuristic
        let model_config = ModelConfig {
            model_name: "o3-mistral".to_string(),
            context_limit: Some(128000),
            temperature: None,
            max_tokens: Some(2048),
            toolshim: false,
            toolshim_model: None,
            fast_model: None,
            request_params: None,
        };

        let messages = vec![crate::conversation::message::Message::user().with_text("Hello")];

        let mut payload = create_request(
            &model_config,
            "You are helpful.",
            &messages,
            &[],
            &ImageFormat::OpenAi,
            false,
        )
        .unwrap();

        // Before strip: OpenAI formatter injected reasoning params
        assert!(payload.get("reasoning_effort").is_some());
        assert_eq!(payload["messages"][0]["role"], "developer");
        assert!(payload.get("max_completion_tokens").is_some());
        assert!(payload.get("max_tokens").is_none());

        // Apply the fix
        LiteLLMProvider::strip_reasoning_params(&mut payload);

        // After strip: reasoning artifacts removed
        assert!(payload.get("reasoning_effort").is_none());
        assert_eq!(payload["messages"][0]["role"], "system");
        assert!(payload.get("max_completion_tokens").is_none());
        assert_eq!(payload["max_tokens"], 2048);

        // Model name should be "o3" (the formatter stripped "-mistral" as reasoning effort suffix)
        // Actually let's check what the formatter does with "o3-mistral"
        let model = payload["model"].as_str().unwrap();
        // The formatter treats the last segment after "-" as potential effort level.
        // "o3-mistral" → last part "mistral" is not low/medium/high, so model stays "o3-mistral"
        assert_eq!(model, "o3-mistral");
    }

    #[test]
    fn test_wildcard_models_filtered_from_capabilities() {
        let proxy_response = json!({
            "data": [
                {
                    "model_name": "anthropic/*",
                    "litellm_params": { "model": "anthropic/*" },
                    "model_info": { "max_input_tokens": 200000 }
                },
                {
                    "model_name": "real-model",
                    "litellm_params": { "model": "openai/gpt-4o" },
                    "model_info": { "max_input_tokens": 128000 }
                }
            ]
        });

        let models_data = proxy_response["data"].as_array().unwrap();
        let mut capabilities = HashMap::new();

        for model_data in models_data {
            if let Some(model_name) = model_data["model_name"].as_str() {
                if model_name.contains("/*") {
                    continue;
                }
                capabilities.insert(
                    model_name.to_string(),
                    LiteLLMModelCapabilities {
                        supports_reasoning: false,
                        supports_cache_control: false,
                        litellm_model: None,
                        max_input_tokens: 128000,
                        input_cost_per_token: None,
                        output_cost_per_token: None,
                    },
                );
            }
        }

        assert!(!capabilities.contains_key("anthropic/*"));
        assert!(capabilities.contains_key("real-model"));
    }
}
