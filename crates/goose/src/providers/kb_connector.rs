//! KB Connector Providers - Routes requests through the embedded proxy server.
//!
//! This module provides two providers for KB금융그룹 AI services:
//!
//! 1. **KbFabrixProvider** (`kb_fabrix`): Uses packed headers (client_id<|>token<|>user_id)
//!    for authentication with the Fabrix Connector system.
//!
//! 2. **KbOpenAiProxyProvider** (`kb_openai_proxy`): Standard API KEY + ENDPOINT,
//!    but still uses the proxy for tool injection (for LLMs without native tool support).
//!
//! Both providers use the embedded goose-connector-proxy to handle:
//! - Tool calling via prompt injection (for LLMs without native tool support)
//! - Response parsing for `<tool_call>` XML tags
//! - Format conversion between OpenAI and custom LLM formats

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use super::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage};
use super::errors::ProviderError;
use super::openai::OpenAiProvider;
use crate::conversation::message::Message;
use crate::model::ModelConfig;
use rmcp::model::Tool;

/// Default model for KB Connector
pub const KB_CONNECTOR_DEFAULT_MODEL: &str = "gpt-oss";

/// Known models available through KB Connector
const KB_CONNECTOR_KNOWN_MODELS: &[&str] = &["gpt-oss", "llama-70b", "llama-405b"];

const KB_CONNECTOR_DOC_URL: &str = "https://kb-ai.kbfg.com/docs";

// =============================================================================
// KB Fabrix Provider
// =============================================================================

/// KB Fabrix Provider - Uses packed headers (client_id<|>token<|>user_id)
#[derive(serde::Serialize)]
pub struct KbFabrixProvider {
    #[serde(skip)]
    inner_provider: Arc<OpenAiProvider>,
    model: ModelConfig,
    name: String,
}

impl KbFabrixProvider {
    /// Create a KbFabrixProvider from environment/config variables.
    ///
    /// Required config keys:
    /// - KB_FABRIX_CLIENT_ID (secret)
    /// - KB_FABRIX_TOKEN (secret)
    /// - KB_FABRIX_USER_ID
    /// - KB_FABRIX_LLM_URL
    ///
    /// Optional:
    /// - KB_FABRIX_LLM_ID (default: gpt-oss)
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        let client_id: String = config.get_secret("KB_FABRIX_CLIENT_ID")?;
        let token: String = config.get_secret("KB_FABRIX_TOKEN")?;
        let user_id: String = config.get_param("KB_FABRIX_USER_ID")?;
        let llm_url: String = config.get_param("KB_FABRIX_LLM_URL")?;
        let llm_id: String = config
            .get_param("KB_FABRIX_LLM_ID")
            .unwrap_or_else(|_| KB_CONNECTOR_DEFAULT_MODEL.to_string());

        // Pack headers in the format: client_id<|>token<|>user_id
        let packed_key = format!("{}<|>{}<|>{}", client_id, token, user_id);

        // Set CONNECTOR_* environment variables for the proxy
        // SAFETY: Setting env vars in single-threaded initialization context
        unsafe {
            std::env::set_var("CONNECTOR_API_KEY", &packed_key);
            std::env::set_var("CONNECTOR_LLM_URL", &llm_url);
            std::env::set_var("CONNECTOR_LLM_ID", &llm_id);
            std::env::set_var("CONNECTOR_MODE", "fabrix"); // Fabrix custom format
            // Force non-streaming since this provider doesn't support streaming
            std::env::set_var("CONNECTOR_FORCE_NON_STREAM", "true");
        }

        tracing::info!(
            "KB Fabrix Provider configured: url={}, llm_id={}, mode=fabrix",
            llm_url,
            llm_id
        );

        // Start the embedded proxy if not already running
        goose_connector_proxy::maybe_start_proxy().await?;

        // Create inner OpenAI provider that will talk to the proxy
        let inner_provider = Arc::new(OpenAiProvider::from_env(model.clone()).await?);

        Ok(Self {
            inner_provider,
            model,
            name: Self::metadata().name,
        })
    }
}

#[async_trait]
impl Provider for KbFabrixProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            "kb_fabrix",
            "KB Fabrix Connector",
            "KB금융그룹 Fabrix Connector - Packed Headers 인증 방식 (client_id/token/user_id)",
            KB_CONNECTOR_DEFAULT_MODEL,
            KB_CONNECTOR_KNOWN_MODELS.to_vec(),
            KB_CONNECTOR_DOC_URL,
            vec![
                ConfigKey::new("KB_FABRIX_CLIENT_ID", true, true, None),
                ConfigKey::new("KB_FABRIX_TOKEN", true, true, None),
                ConfigKey::new("KB_FABRIX_USER_ID", true, false, None),
                ConfigKey::new("KB_FABRIX_LLM_URL", true, false, None),
                // LLM_ID는 필수 - v1/models 엔드포인트가 없으므로 직접 입력 받음
                ConfigKey::new("KB_FABRIX_LLM_ID", true, false, Some(KB_CONNECTOR_DEFAULT_MODEL)),
            ],
        )
        .with_unlisted_models()
        .with_skip_model_fetch() // v1/models 엔드포인트 없음
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn complete_with_model(
        &self,
        session_id: &str,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        self.inner_provider
            .complete_with_model(session_id, model_config, system, messages, tools)
            .await
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    fn supports_streaming(&self) -> bool {
        false
    }
}

// =============================================================================
// KB OpenAI Proxy Provider
// =============================================================================

/// KB OpenAI Proxy Provider - Standard API KEY but uses proxy for tool injection
///
/// This is for OpenAI-compatible LLMs that don't support native tool calling.
/// The proxy handles tool injection via prompts and parses `<tool_call>` responses.
#[derive(serde::Serialize)]
pub struct KbOpenAiProxyProvider {
    #[serde(skip)]
    inner_provider: Arc<OpenAiProvider>,
    model: ModelConfig,
    name: String,
}

impl KbOpenAiProxyProvider {
    /// Create a KbOpenAiProxyProvider from environment/config variables.
    ///
    /// Required config keys:
    /// - KB_OPENAI_PROXY_API_KEY (secret)
    /// - KB_OPENAI_PROXY_ENDPOINT
    ///
    /// Model name is passed via ModelConfig (from "Enter a model" prompt)
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();

        let api_key: String = config.get_secret("KB_OPENAI_PROXY_API_KEY")?;
        let endpoint: String = config.get_param("KB_OPENAI_PROXY_ENDPOINT")?;
        // 모델명은 ModelConfig에서 가져옴 (configure 시 "Enter a model" 프롬프트에서 입력받은 값)
        let model_name = model.model_name.clone();

        // For OpenAI proxy mode, we use the API key directly (no packed format)
        // SAFETY: Setting env vars in single-threaded initialization context
        unsafe {
            std::env::set_var("CONNECTOR_API_KEY", &api_key);
            std::env::set_var("CONNECTOR_LLM_URL", &endpoint);
            std::env::set_var("CONNECTOR_LLM_ID", &model_name);
            std::env::set_var("CONNECTOR_MODE", "openai"); // OpenAI compatible format
            // Force non-streaming since this provider doesn't support streaming
            std::env::set_var("CONNECTOR_FORCE_NON_STREAM", "true");
        }

        tracing::info!(
            "KB OpenAI Proxy Provider configured: endpoint={}, model={}, mode=openai",
            endpoint,
            model_name
        );

        // Start the embedded proxy if not already running
        goose_connector_proxy::maybe_start_proxy().await?;

        // Create inner OpenAI provider that will talk to the proxy
        let inner_provider = Arc::new(OpenAiProvider::from_env(model.clone()).await?);

        Ok(Self {
            inner_provider,
            model,
            name: Self::metadata().name,
        })
    }
}

#[async_trait]
impl Provider for KbOpenAiProxyProvider {
    fn metadata() -> ProviderMetadata
    where
        Self: Sized,
    {
        ProviderMetadata::new(
            "kb_openai_proxy",
            "KB OpenAI Proxy (No Native Tool)",
            "KB금융그룹 OpenAI 호환 프록시 - Native Tool calling이 불가능한 LLM용 (프록시가 Tool Injection 처리)",
            KB_CONNECTOR_DEFAULT_MODEL,
            KB_CONNECTOR_KNOWN_MODELS.to_vec(),
            KB_CONNECTOR_DOC_URL,
            vec![
                ConfigKey::new("KB_OPENAI_PROXY_API_KEY", true, true, None),
                ConfigKey::new("KB_OPENAI_PROXY_ENDPOINT", true, false, None),
                // 일반 model name 사용 - "Enter a model" 프롬프트에서 입력받음
            ],
        )
        .with_unlisted_models()
        .with_skip_model_fetch() // v1/models 엔드포인트 없음 (하지만 모델 프롬프트는 표시)
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    async fn complete_with_model(
        &self,
        session_id: &str,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        self.inner_provider
            .complete_with_model(session_id, model_config, system, messages, tools)
            .await
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    fn supports_streaming(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kb_fabrix_metadata() {
        let meta = KbFabrixProvider::metadata();
        assert_eq!(meta.name, "kb_fabrix");
        assert_eq!(meta.default_model, KB_CONNECTOR_DEFAULT_MODEL);
        assert!(meta.allows_unlisted_models);
        // Should have 5 config keys: CLIENT_ID, TOKEN, USER_ID, LLM_URL, LLM_ID
        assert_eq!(meta.config_keys.len(), 5);
        // First 4 should be required
        assert!(meta.config_keys[0].required); // CLIENT_ID
        assert!(meta.config_keys[1].required); // TOKEN
        assert!(meta.config_keys[2].required); // USER_ID
        assert!(meta.config_keys[3].required); // LLM_URL
        assert!(!meta.config_keys[4].required); // LLM_ID (optional)
    }

    #[test]
    fn test_kb_openai_proxy_metadata() {
        let meta = KbOpenAiProxyProvider::metadata();
        assert_eq!(meta.name, "kb_openai_proxy");
        assert_eq!(meta.default_model, KB_CONNECTOR_DEFAULT_MODEL);
        assert!(meta.allows_unlisted_models);
        // Should have 3 config keys: API_KEY, ENDPOINT, LLM_ID
        assert_eq!(meta.config_keys.len(), 3);
        // First 2 should be required
        assert!(meta.config_keys[0].required); // API_KEY
        assert!(meta.config_keys[1].required); // ENDPOINT
        assert!(!meta.config_keys[2].required); // LLM_ID (optional)
    }

    #[test]
    fn test_fabrix_packed_format() {
        let client_id = "my_client";
        let token = "my_token";
        let user_id = "user123";
        let packed = format!("{}<|>{}<|>{}", client_id, token, user_id);
        assert_eq!(packed, "my_client<|>my_token<|>user123");
    }
}
