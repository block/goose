pub mod base;
pub mod errors;
pub mod formats;
pub mod mock;

#[cfg(feature = "http")]
pub mod databricks;
#[cfg(feature = "http")]
pub mod openai;
#[cfg(feature = "http")]
mod factory;
#[cfg(feature = "http")]
pub mod utils;

pub use base::{Provider, ProviderCompleteResponse, ProviderExtractResponse, Usage};
#[cfg(feature = "http")]
pub use factory::create;

#[cfg(not(feature = "http"))]
pub fn create(
    provider_name: &str,
    provider_config: serde_json::Value,
    model_config: crate::model::ModelConfig,
) -> Result<std::sync::Arc<dyn Provider>, errors::Error> {
    use std::sync::Arc;
    
    match provider_name {
        "mock" => {
            let config: mock::MockProviderConfig = serde_json::from_value(provider_config)
                .map_err(|e| errors::Error::ProviderError(errors::ProviderError::ExecutionError(
                    format!("Failed to parse mock provider config: {}", e)
                )))?;
            
            mock::MockProvider::from_config(config, model_config)
                .map(|provider| Arc::new(provider) as Arc<dyn Provider>)
                .map_err(errors::Error::ProviderError)
        },
        _ => Err(errors::Error::UnsupportedProvider(format!(
            "Provider '{}' is not supported in WASM mode. Use 'mock' provider instead.",
            provider_name
        ))),
    }
}