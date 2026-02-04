use super::api_client::{ApiClient, AuthMethod};
use super::base::{ConfigKey, ProviderDef, ProviderMetadata};
use super::openai_compatible::OpenAiCompatibleProvider;
use crate::model::ModelConfig;
use anyhow::Result;
use futures::future::BoxFuture;
use goose_decentralized_models::{discover_model_filtered, ModelFilter};
use std::time::Duration;

const DECENTRALIZED_PROVIDER_NAME: &str = "decentralized";
const DECENTRALIZED_DEFAULT_MODEL: &str = "discovered";
const DECENTRALIZED_DOC_URL: &str = "https://github.com/block/goose";
const DECENTRALIZED_TIMEOUT: u64 = 600;

pub struct DecentralizedProvider;

impl ProviderDef for DecentralizedProvider {
    type Provider = OpenAiCompatibleProvider;

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::new(
            DECENTRALIZED_PROVIDER_NAME,
            "Decentralized",
            "Peer-to-peer LLM models discovered via Nostr relays",
            DECENTRALIZED_DEFAULT_MODEL,
            vec![DECENTRALIZED_DEFAULT_MODEL],
            DECENTRALIZED_DOC_URL,
            vec![
                ConfigKey::new("DECENTRALIZED_GEO", false, false, None),
                ConfigKey::new("DECENTRALIZED_MAX_COST", false, false, None),
                ConfigKey::new("DECENTRALIZED_MIN_CONTEXT", false, false, None),
                ConfigKey::new("DECENTRALIZED_RELAYS", false, false, None),
                ConfigKey::new(
                    "DECENTRALIZED_TIMEOUT",
                    false,
                    false,
                    Some(&DECENTRALIZED_TIMEOUT.to_string()),
                ),
            ],
        )
        .with_unlisted_models()
    }

    fn from_env(model: ModelConfig) -> BoxFuture<'static, Result<OpenAiCompatibleProvider>> {
        Box::pin(async move {
            let config = crate::config::Config::global();

            // Build filter from environment variables
            let mut filter = ModelFilter::new();

            // Use the model name if it's not the default placeholder
            if model.model_name != DECENTRALIZED_DEFAULT_MODEL {
                filter = filter.model(&model.model_name);
            }

            if let Ok(geo) = config.get_param::<String>("DECENTRALIZED_GEO") {
                filter = filter.geo(&geo);
            }

            if let Ok(max_cost) = config.get_param::<f64>("DECENTRALIZED_MAX_COST") {
                filter = filter.max_cost(max_cost);
            }

            if let Ok(min_context) = config.get_param::<u32>("DECENTRALIZED_MIN_CONTEXT") {
                filter = filter.min_context(min_context);
            }

            // Get custom relays if specified
            let relays: Option<Vec<String>> = config
                .get_param::<String>("DECENTRALIZED_RELAYS")
                .ok()
                .map(|s| s.split(',').map(|r| r.trim().to_string()).collect());

            let timeout = Duration::from_secs(
                config
                    .get_param("DECENTRALIZED_TIMEOUT")
                    .unwrap_or(DECENTRALIZED_TIMEOUT),
            );

            // Discover a model
            tracing::info!("Discovering decentralized model with filter: {:?}", filter);
            let discovered = discover_model_filtered(relays, &filter)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No decentralized model found matching filter"))?;

            tracing::info!(
                "Discovered model '{}' at {} from publisher {}",
                discovered.model_name,
                discovered.endpoint,
                discovered.publisher_npub
            );

            // Create API client for the discovered endpoint
            let api_client =
                ApiClient::with_timeout(discovered.endpoint.clone(), AuthMethod::NoAuth, timeout)?;

            // Create model config with the discovered model name and context size
            let mut new_model = ModelConfig::new(&discovered.model_name)?
                .with_temperature(model.temperature)
                .with_max_tokens(model.max_tokens)
                .with_toolshim(model.toolshim)
                .with_toolshim_model(model.toolshim_model);
            if let Some(ctx) = discovered.context_size {
                new_model = new_model.with_context_limit(Some(ctx as usize));
            }

            Ok(OpenAiCompatibleProvider::new(
                DECENTRALIZED_PROVIDER_NAME.to_string(),
                api_client,
                new_model,
            ))
        })
    }
}
