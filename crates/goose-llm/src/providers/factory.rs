use std::sync::Arc;

use anyhow::Result;

use super::{
    base::Provider,
    databricks::{DatabricksProvider, DatabricksProviderConfig},
    openai::{OpenAiProvider, OpenAiProviderConfig},
};
use crate::model::ModelConfig;

pub fn create(
    name: &str,
    provider_config: serde_json::Value,
    model: ModelConfig,
) -> Result<Arc<dyn Provider>> {
    // We use Arc instead of Box to be able to clone for multiple async tasks
    match name {
        "openai" => {
            let config: OpenAiProviderConfig = serde_json::from_value(provider_config)?;
            Ok(Arc::new(OpenAiProvider::from_config(config, model)?))
        }
        "databricks" => {
            let config: DatabricksProviderConfig = serde_json::from_value(provider_config)?;
            Ok(Arc::new(DatabricksProvider::from_config(config, model)?))
        }
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}