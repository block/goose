pub mod base;
pub mod errors;
pub mod formats;

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
) -> Result<Box<dyn Provider>, errors::Error> {
    Err(errors::Error::UnsupportedProvider(format!(
        "Provider '{}' is not supported in this build",
        provider_name
    )))
}