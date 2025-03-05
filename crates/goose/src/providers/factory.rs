use super::{
    anthropic::AnthropicProvider,
    azure::AzureProvider,
    base::{Provider, ProviderMetadata},
    bedrock::BedrockProvider,
    databricks::DatabricksProvider,
    gcpvertexai::GcpVertexAIProvider,
    google::GoogleProvider,
    groq::GroqProvider,
    ollama::OllamaProvider,
    openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
};
use crate::model::ModelConfig;
use anyhow::Result;

pub fn providers() -> Vec<ProviderMetadata> {
    vec![
        AnthropicProvider::metadata(),
        AzureProvider::metadata(),
        BedrockProvider::metadata(),
        DatabricksProvider::metadata(),
        GcpVertexAIProvider::metadata(),
        GoogleProvider::metadata(),
        GroqProvider::metadata(),
        OllamaProvider::metadata(),
        OpenAiProvider::metadata(),
        OpenRouterProvider::metadata(),
    ]
}

pub fn create(name: &str, model: ModelConfig) -> Result<Box<dyn Provider + Send + Sync>> {
    match name {
        "openai" => Ok(Box::new(OpenAiProvider::from_env(model)?)),
        "anthropic" => Ok(Box::new(AnthropicProvider::from_env(model)?)),
        "azure_openai" => Ok(Box::new(AzureProvider::from_env(model)?)),
        "bedrock" => Ok(Box::new(BedrockProvider::from_env(model)?)),
        "databricks" => Ok(Box::new(DatabricksProvider::from_env(model)?)),
        "groq" => Ok(Box::new(GroqProvider::from_env(model)?)),
        "ollama" => Ok(Box::new(OllamaProvider::from_env(model)?)),
        "openrouter" => Ok(Box::new(OpenRouterProvider::from_env(model)?)),
        "gcp_vertex_ai" => Ok(Box::new(GcpVertexAIProvider::from_env(model)?)),
        "google" => Ok(Box::new(GoogleProvider::from_env(model)?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}
