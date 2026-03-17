use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::model::ModelConfig;
use anyhow::Result;
use futures::future::BoxFuture;

const AVIAN_PROVIDER_NAME: &str = "avian";
pub const AVIAN_API_HOST: &str = "https://api.avian.io/v1";
pub const AVIAN_DEFAULT_MODEL: &str = "deepseek/deepseek-v3.2";
pub const AVIAN_KNOWN_MODELS: &[&str] = &[
    "deepseek/deepseek-v3.2",
    "moonshotai/kimi-k2.5",
    "z-ai/glm-5",
    "minimax/minimax-m2.5",
];
pub const AVIAN_DOC_URL: &str = "https://avian.io/docs";

pub struct AvianProvider;

impl ProviderDef for AvianProvider {
    type Provider = OpenAiCompatibleProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            AVIAN_PROVIDER_NAME,
            "Avian",
            "Cost-effective inference API with DeepSeek, Kimi, GLM, and MiniMax models",
            AVIAN_DEFAULT_MODEL,
            AVIAN_KNOWN_MODELS.to_vec(),
            AVIAN_DOC_URL,
            vec![
                ConfigKey::new("AVIAN_API_KEY", true, true, None, true),
                ConfigKey::new("AVIAN_HOST", false, false, Some(AVIAN_API_HOST), false),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<OpenAiCompatibleProvider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();
            let api_key: String = config.get_secret("AVIAN_API_KEY")?;
            let host: String = config
                .get_param("AVIAN_HOST")
                .unwrap_or_else(|_| AVIAN_API_HOST.to_string());

            let api_client = ApiClient::new(host, AuthMethod::BearerToken(api_key))?;

            Ok(OpenAiCompatibleProvider::new(
                AVIAN_PROVIDER_NAME.to_string(),
                api_client,
                model,
                String::new(),
            ))
        })
    }
}
