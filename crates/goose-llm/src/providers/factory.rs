use std::sync::Arc;

use anyhow::Result;

use super::{base::Provider, databricks::DatabricksProvider, openai::OpenAiProvider};
use crate::model::ModelConfig;

pub fn create(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    // We use Arc instead of Box to be able to clone for multiple async tasks
    match name {
        "openai" => Ok(Arc::new(OpenAiProvider::from_env(model)?)),
        "databricks" => Ok(Arc::new(DatabricksProvider::from_env(model)?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}
