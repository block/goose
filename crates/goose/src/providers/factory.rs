use super::{
    anthropic::AnthropicProvider, base::Provider, databricks::DatabricksProvider,
    google::GoogleProvider, groq::GroqProvider, ollama::OllamaProvider, openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
};
use anyhow::Result;

pub fn get_provider(name: &str) -> Result<Box<dyn Provider + Send + Sync>> {
    match name {
        "openai" => Ok(Box::new(OpenAiProvider::from_env()?)),
        "anthropic" => Ok(Box::new(AnthropicProvider::from_env()?)),
        "databricks" => Ok(Box::new(DatabricksProvider::from_env()?)),
        "groq" => Ok(Box::new(GroqProvider::from_env()?)),
        "ollama" => Ok(Box::new(OllamaProvider::from_env()?)),
        "openrouter" => Ok(Box::new(OpenRouterProvider::from_env()?)),
        "google" => Ok(Box::new(GoogleProvider::from_env()?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}
