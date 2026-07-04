use anyhow::Result;
use futures::future::BoxFuture;

use crate::{
    config::{Config, DeclarativeProviderConfig},
    providers::{base::ProviderDef, custom_provider_config::ConfigKeyResolver},
};
use gosling_providers::{
    anthropic::{self, AnthropicProvider, AnthropicProviderBuilder, ANTHROPIC_API_VERSION},
    api_client::{ApiClient, AuthMethod, TlsConfig},
    base::ProviderDescriptor,
};

pub struct AnthropicProviderDef;

impl ProviderDescriptor for AnthropicProviderDef {
    fn metadata() -> gosling_providers::base::ProviderMetadata {
        AnthropicProvider::metadata()
    }
}

impl ProviderDef for AnthropicProviderDef {
    type Provider = AnthropicProvider;

    fn from_env(
        _extensions: Vec<crate::config::ExtensionConfig>,
        tls_config: Option<crate::providers::api_client::TlsConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>> {
        Box::pin(from_env(tls_config))
    }
}

async fn from_env(
    tls_config: Option<crate::providers::api_client::TlsConfig>,
) -> Result<AnthropicProvider> {
    let config = crate::config::Config::global();
    let api_key: String = config.get_secret("ANTHROPIC_API_KEY")?;
    let host: String = config
        .get_param("ANTHROPIC_HOST")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_string());

    let auth = AuthMethod::ApiKey {
        header_name: "x-api-key".to_string(),
        key: api_key,
    };

    let api_client = ApiClient::new_with_tls(host, auth, tls_config)?
        .with_request_builder(crate::session_context::session_id_request_builder())
        .with_header("anthropic-version", ANTHROPIC_API_VERSION)?;

    Ok(AnthropicProviderBuilder::new(api_client).build())
}

pub fn from_custom_config(
    config: DeclarativeProviderConfig,
    tls_config: Option<TlsConfig>,
) -> Result<AnthropicProvider> {
    anthropic::from_declarative_config(config, tls_config, ConfigKeyResolver::new(Config::global()))
        .map(|builder| {
            builder
                .map_api_client(|api_client| {
                    api_client
                        .with_request_builder(crate::session_context::session_id_request_builder())
                })
                .build()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::declarative_providers::{DeclarativeProviderConfig, ProviderEngine};
    use gosling_providers::base::{ModelInfo, Provider};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_provider_with_server(
        server_uri: &str,
        custom_models: Option<Vec<String>>,
        dynamic_models: Option<bool>,
    ) -> AnthropicProvider {
        let auth = AuthMethod::ApiKey {
            header_name: "x-api-key".to_string(),
            key: "test-key".to_string(),
        };
        let api_client = ApiClient::new_with_tls(server_uri.to_string(), auth, None)
            .unwrap()
            .with_header("anthropic-version", ANTHROPIC_API_VERSION)
            .unwrap();
        AnthropicProviderBuilder::new(api_client)
            .name("custom_anthropic")
            .custom_models(custom_models)
            .dynamic_models(dynamic_models)
            .build()
    }

    fn base_declarative_config(
        models: Vec<ModelInfo>,
        dynamic_models: Option<bool>,
    ) -> DeclarativeProviderConfig {
        DeclarativeProviderConfig {
            name: "custom_anthropic".to_string(),
            engine: ProviderEngine::Anthropic,
            display_name: "Custom Anthropic".to_string(),
            description: None,
            api_key_env: String::new(),
            base_url: "http://localhost:1".to_string(),
            models,
            headers: None,
            timeout_seconds: None,
            supports_streaming: Some(true),
            requires_auth: false,
            catalog_provider_id: None,
            base_path: None,
            env_vars: None,
            dynamic_models,
            skip_canonical_filtering: false,
            model_doc_link: None,
            setup_steps: vec![],
            fast_model: None,
            preserves_thinking: false,
        }
    }

    #[tokio::test]
    async fn fetch_supported_models_static_only_skips_api() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let provider = make_provider_with_server(
            &server.uri(),
            Some(vec!["m1".to_string(), "m2".to_string()]),
            Some(false),
        );

        let models = provider.fetch_supported_models().await.unwrap();
        assert_eq!(models, vec!["m1".to_string(), "m2".to_string()]);
    }

    #[test]
    fn from_custom_config_rejects_static_only_without_models() {
        let config = base_declarative_config(vec![], Some(false));
        let err = from_custom_config(config, None)
            .err()
            .expect("expected construction error for dynamic_models: false with empty models");
        let msg = err.to_string();
        assert!(
            msg.contains("dynamic_models: false"),
            "error message should mention dynamic_models: false; got: {msg}"
        );
    }
}
