use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use anyhow::Result;
use futures::future::BoxFuture;

const AIMLAPI_PROVIDER_NAME: &str = "aimlapi";
pub const AIMLAPI_API_HOST: &str = "https://api.aimlapi.com/v1";
pub const AIMLAPI_DEFAULT_MODEL: &str = "openai/gpt-5-5";
pub const AIMLAPI_KNOWN_MODELS: &[&str] = &[
    "openai/gpt-5-5",
    "anthropic/claude-opus-5",
    "google/gemini-3-7-flash",
    "deepseek/deepseek-v4-pro-0813",
];
pub const AIMLAPI_DOC_URL: &str = "https://docs.aimlapi.com";

pub struct AimlapiProvider;

impl goose_providers::base::ProviderDescriptor for AimlapiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            AIMLAPI_PROVIDER_NAME,
            "AI/ML API",
            "One API key for 300+ chat, image, video, audio, and embedding models from many providers",
            AIMLAPI_DEFAULT_MODEL,
            AIMLAPI_KNOWN_MODELS.to_vec(),
            AIMLAPI_DOC_URL,
            vec![
                ConfigKey::new("AIMLAPI_API_KEY", true, true, None, true),
                ConfigKey::new("AIMLAPI_HOST", false, false, Some(AIMLAPI_API_HOST), false),
            ],
        )
    }
}

impl ProviderDef for AimlapiProvider {
    type Provider = OpenAiCompatibleProvider;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<OpenAiCompatibleProvider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();
            let api_key: String = config.get_secret("AIMLAPI_API_KEY")?;
            let host: String = config
                .get_param("AIMLAPI_HOST")
                .unwrap_or_else(|_| AIMLAPI_API_HOST.to_string());

            let api_client =
                ApiClient::new_with_tls(host, AuthMethod::BearerToken(api_key), tls_config)?
                    .with_request_builder(crate::session_context::session_id_request_builder());

            Ok(OpenAiCompatibleProvider::new(
                AIMLAPI_PROVIDER_NAME.to_string(),
                api_client,
                String::new(),
            ))
        })
    }
}
