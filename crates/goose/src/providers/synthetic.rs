use super::api_client::{ApiClient, AuthMethod};
use super::errors::ProviderError;
use super::retry::ProviderRetry;
use super::utils::{get_model, handle_response_openai_compat};
use crate::conversation::message::Message;
use crate::impl_provider_default;
use crate::model::ModelConfig;
use crate::providers::base::{ConfigKey, Provider, ProviderMetadata, ProviderUsage, Usage};
use crate::providers::formats::openai::{create_request, get_usage, response_to_message};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::Value;

pub const SYNTHETIC_API_HOST: &str = "https://api.synthetic.new";
pub const SYNTHETIC_DEFAULT_MODEL: &str = "hf:zai-org/GLM-4.5";
pub const SYNTHETIC_KNOWN_MODELS: &[&str] = &[
    // This will be populated dynamically from the API, but provide some defaults
    "hf:zai-org/GLM-4.5",
    "hf:moonshotai/Kimi-K2-Instruct-0905",
    "hf:deepseek-ai/DeepSeek-V3-0324",
    "hf:deepseek-ai/DeepSeek-R1-0528",
];

// TODO(billy@syntheticlab.co): Update this when developer API docs are launched.
pub const SYNTHETIC_DOC_URL: &str = "https://synthetic.new";

#[derive(serde::Serialize)]
pub struct SyntheticProvider {
    #[serde(skip)]
    api_client: ApiClient,
    model: ModelConfig,
}

impl_provider_default!(SyntheticProvider);

impl SyntheticProvider {
    pub fn from_env(model: ModelConfig) -> Result<Self> {
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("SYNTHETIC_API_KEY")?;
        let host: String = config
            .get_param("SYNTHETIC_HOST")
            .unwrap_or_else(|_| SYNTHETIC_API_HOST.to_string());

        let auth = AuthMethod::BearerToken(api_key);
        let api_client = ApiClient::new(host, auth)?;

        Ok(Self { api_client, model })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let response = self
            .api_client
            .response_post("openai/v1/chat/completions", &payload)
            .await?;
        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for SyntheticProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            "synthetic",
            "Synthetic",
            "Unified provider for popular open weight models.",
            SYNTHETIC_DEFAULT_MODEL,
            SYNTHETIC_KNOWN_MODELS.to_vec(),
            SYNTHETIC_DOC_URL,
            vec![
                ConfigKey::new("SYNTHETIC_API_KEY", true, true, None),
                ConfigKey::new("SYNTHETIC_HOST", false, false, Some(SYNTHETIC_API_HOST)),
            ],
        )
    }

    fn get_model_config(&self) -> ModelConfig {
        self.model.clone()
    }

    #[tracing::instrument(
        skip(self, model_config, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    async fn complete_with_model(
        &self,
        model_config: &ModelConfig,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<(Message, ProviderUsage), ProviderError> {
        let payload = create_request(
            model_config,
            system,
            messages,
            tools,
            &super::utils::ImageFormat::OpenAi,
        )?;

        let response = self.with_retry(|| self.post(payload.clone())).await?;

        let message = response_to_message(&response)?;
        let usage = response.get("usage").map(get_usage).unwrap_or_else(|| {
            tracing::debug!("Failed to get usage data");
            Usage::default()
        });
        let response_model = get_model(&response);
        super::utils::emit_debug_trace(model_config, &payload, &response, &usage);
        Ok((message, ProviderUsage::new(response_model, usage)))
    }

    /// Fetch supported models from Synthetic API
    async fn fetch_supported_models(&self) -> Result<Option<Vec<String>>, ProviderError> {
        let response = self
            .api_client
            .request("openai/v1/models")
            .header("Content-Type", "application/json")?
            .response_get()
            .await?;
        let response = handle_response_openai_compat(response).await?;

        let data = response
            .get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                ProviderError::UsageError("Missing or invalid `data` field in response".into())
            })?;

        let mut model_names: Vec<String> = data
            .iter()
            .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(String::from))
            .collect();
        model_names.sort();
        Ok(Some(model_names))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_structure() {
        let metadata = SyntheticProvider::metadata();

        assert_eq!(metadata.default_model, "hf:zai-org/GLM-4.5");
        assert!(!metadata.known_models.is_empty());

        assert_eq!(metadata.config_keys.len(), 2);
        assert_eq!(metadata.config_keys[0].name, "SYNTHETIC_API_KEY");
        assert_eq!(metadata.config_keys[1].name, "SYNTHETIC_HOST");
    }
}
