use std::sync::Arc;

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

pub fn create(name: &str, model: ModelConfig) -> Result<Arc<dyn Provider>> {
    // We use Arc instead of Box to be able to clone for multiple async tasks
    match name {
        "openai" => Ok(Arc::new(OpenAiProvider::from_env(model)?)),
        "anthropic" => Ok(Arc::new(AnthropicProvider::from_env(model)?)),
        "azure_openai" => Ok(Arc::new(AzureProvider::from_env(model)?)),
        "aws_bedrock" => Ok(Arc::new(BedrockProvider::from_env(model)?)),
        "databricks" => Ok(Arc::new(DatabricksProvider::from_env(model)?)),
        "groq" => Ok(Arc::new(GroqProvider::from_env(model)?)),
        "ollama" => Ok(Arc::new(OllamaProvider::from_env(model)?)),
        "openrouter" => Ok(Arc::new(OpenRouterProvider::from_env(model)?)),
        "gcp_vertex_ai" => Ok(Arc::new(GcpVertexAIProvider::from_env(model)?)),
        "google" => Ok(Arc::new(GoogleProvider::from_env(model)?)),
        _ => Err(anyhow::anyhow!("Unknown provider: {}", name)),
    }
}
