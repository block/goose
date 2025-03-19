use super::{
    base::{Provider, ProviderMetadata},
    databricks::DatabricksProvider,
    openai::OpenAiProvider,
};
use crate::model::ModelConfig;
use anyhow::Result;

pub fn providers() -> Vec<ProviderMetadata> {
    vec![DatabricksProvider::metadata(), OpenAiProvider::metadata()]
}

pub fn create(name: &str, model: ModelConfig) -> Result<Box<dyn Provider + Send + Sync>> {
    match name {
        "openai" => Ok(Box::new(OpenAiProvider::from_env(model)?)),
        "databricks" => Ok(Box::new(DatabricksProvider::from_env(model)?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}
