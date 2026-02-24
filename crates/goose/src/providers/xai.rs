use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::model::ModelConfig;
use anyhow::Result;
use futures::future::BoxFuture;

const XAI_PROVIDER_NAME: &str = "xai";
pub const XAI_API_HOST: &str = "https://api.x.ai/v1";
pub const XAI_DEFAULT_MODEL: &str = "grok-code-fast-1";

pub const XAI_DOC_URL: &str = "https://docs.x.ai/docs/overview";

pub struct XaiProvider;

impl ProviderDef for XaiProvider {
    type Provider = OpenAiCompatibleProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::from_canonical(
            XAI_PROVIDER_NAME,
            "xAI",
            "Grok models from xAI, including reasoning and multimodal capabilities",
            XAI_DEFAULT_MODEL,
            vec![],
            XAI_DOC_URL,
            vec![
                ConfigKey::new("XAI_API_KEY", true, true, None, true),
                ConfigKey::new("XAI_HOST", false, false, Some(XAI_API_HOST), false),
            ],
        )
    }

    fn from_env(
        model: ModelConfig,
        _extensions: Vec<crate::config::ExtensionConfig>,
    ) -> BoxFuture<'static, Result<OpenAiCompatibleProvider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();
            let api_key: String = config.get_secret("XAI_API_KEY")?;
            let host: String = config
                .get_param("XAI_HOST")
                .unwrap_or_else(|_| XAI_API_HOST.to_string());

            let api_client = ApiClient::new(host, AuthMethod::BearerToken(api_key))?;

            Ok(OpenAiCompatibleProvider::new(
                XAI_PROVIDER_NAME.to_string(),
                api_client,
                model,
                String::new(),
            ))
        })
    }
}
