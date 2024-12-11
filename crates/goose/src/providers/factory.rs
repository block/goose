use super::{
    anthropic::AnthropicProvider, base::Provider, configs::ProviderConfig,
    databricks::DatabricksProvider, ollama::OllamaProvider, openai::OpenAiProvider,
};
use anyhow::Result;
use strum_macros::EnumIter;

#[derive(EnumIter, Debug)]
pub enum ProviderType {
    OpenAi,
    Databricks,
    Ollama,
    Anthropic,
}

pub fn get_provider(config: ProviderConfig) -> Result<Box<dyn Provider + Send + Sync>> {
    match config {
        ProviderConfig::OpenAi(openai_config) => Ok(Box::new(OpenAiProvider::new(openai_config)?)),
        ProviderConfig::Databricks(databricks_config) => {
            Ok(Box::new(DatabricksProvider::new(databricks_config)?))
        }
        ProviderConfig::Ollama(ollama_config) => Ok(Box::new(OllamaProvider::new(ollama_config)?)),
        ProviderConfig::Anthropic(anthropic_config) => {
            Ok(Box::new(AnthropicProvider::new(anthropic_config)?))
        }
    }
}
