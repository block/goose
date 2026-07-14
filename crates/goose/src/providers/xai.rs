use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use anyhow::Result;
use futures::future::BoxFuture;

const XAI_PROVIDER_NAME: &str = "xai";
pub const XAI_API_HOST: &str = "https://api.x.ai/v1";
pub const XAI_DEFAULT_MODEL: &str = "grok-code-fast-1";
pub const XAI_KNOWN_MODELS: &[&str] = &[
    "grok-code-fast-1",
    "grok-4-0709",
    "grok-4-latest",
    "grok-4-fast-reasoning",
    "grok-4-fast-reasoning-latest",
    "grok-4-fast-non-reasoning",
    "grok-4-fast-non-reasoning-latest",
    "grok-4.5",
    "grok-3",
    "grok-3-fast",
    "grok-3-mini",
    "grok-3-mini-fast",
    "grok-2-vision-1212",
    "grok-2-image-1212",
    "grok-3-latest",
    "grok-3-fast-latest",
    "grok-3-mini-latest",
    "grok-3-mini-fast-latest",
    "grok-2-vision",
    "grok-2-vision-latest",
    "grok-2-image",
    "grok-2-image-latest",
    "grok-2",
    "grok-2-latest",
];

pub const XAI_DOC_URL: &str = "https://docs.x.ai/docs/overview";

pub struct XaiProvider;

/// Known-model metadata for both the API-key and OAuth xAI providers.
/// All models resolve context limits and reasoning flags via the canonical
/// registry (`x-ai/grok-*`).
pub fn xai_known_model_info() -> Vec<goose_providers::base::ModelInfo> {
    use goose_providers::base::model_info_for_provider_model;

    XAI_KNOWN_MODELS
        .iter()
        .map(|&model_name| model_info_for_provider_model(XAI_PROVIDER_NAME, model_name))
        .collect()
}

impl goose_providers::base::ProviderDescriptor for XaiProvider {
    fn metadata() -> ProviderMetadata {
        ProviderMetadata::with_models(
            XAI_PROVIDER_NAME,
            "xAI",
            "Grok models from xAI, including reasoning and multimodal capabilities",
            XAI_DEFAULT_MODEL,
            xai_known_model_info(),
            XAI_DOC_URL,
            vec![
                ConfigKey::new("XAI_API_KEY", true, true, None, true),
                ConfigKey::new("XAI_HOST", false, false, Some(XAI_API_HOST), false),
            ],
        )
    }
}

impl ProviderDef for XaiProvider {
    type Provider = OpenAiCompatibleProvider;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<OpenAiCompatibleProvider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();
            let api_key: String = config.get_secret("XAI_API_KEY")?;
            let host: String = config
                .get_param("XAI_HOST")
                .unwrap_or_else(|_| XAI_API_HOST.to_string());

            let api_client =
                ApiClient::new_with_tls(host, AuthMethod::BearerToken(api_key), tls_config)?
                    .with_request_builder(crate::session_context::session_id_request_builder());

            Ok(OpenAiCompatibleProvider::new(
                XAI_PROVIDER_NAME.to_string(),
                api_client,
                String::new(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_providers::base::ProviderDescriptor;

    #[test]
    fn grok_4_5_known_model_resolves_via_canonical_registry() {
        let info = xai_known_model_info()
            .into_iter()
            .find(|m| m.name == "grok-4.5")
            .expect("grok-4.5 present in known models");
        assert_eq!(info.context_limit, 500_000);
        assert!(info.reasoning);
    }

    #[test]
    fn metadata_includes_grok_4_5_from_canonical_registry() {
        let metadata = XaiProvider::metadata();
        let info = metadata
            .known_models
            .iter()
            .find(|m| m.name == "grok-4.5")
            .expect("grok-4.5 present in provider metadata");
        assert_eq!(info.context_limit, 500_000);
        assert!(info.reasoning);
    }
}
