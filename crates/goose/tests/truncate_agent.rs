// src/lib.rs or tests/truncate_agent_tests.rs

use anyhow::Result;
use futures::StreamExt;
use goose::agents::AgentFactory;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::base::Provider;
use goose::providers::{
    anthropic::AnthropicProvider, azure::AzureProvider, bedrock::BedrockProvider,
    databricks::DatabricksProvider, gcpvertexai::GcpVertexAIProvider, google::GoogleProvider,
    groq::GroqProvider, ollama::OllamaProvider, openai::OpenAiProvider,
    openrouter::OpenRouterProvider,
};

#[derive(Debug, PartialEq)]
enum ProviderType {
    Azure,
    OpenAi,
    Anthropic,
    Bedrock,
    Databricks,
    GcpVertexAI,
    Google,
    Groq,
    Ollama,
    OpenRouter,
}

impl ProviderType {
    fn required_env(&self) -> &'static [&'static str] {
        match self {
            ProviderType::Azure => &[
                "AZURE_OPENAI_API_KEY",
                "AZURE_OPENAI_ENDPOINT",
                "AZURE_OPENAI_DEPLOYMENT_NAME",
            ],
            ProviderType::OpenAi => &["OPENAI_API_KEY"],
            ProviderType::Anthropic => &["ANTHROPIC_API_KEY"],
            ProviderType::Bedrock => &["AWS_PROFILE"],
            ProviderType::Databricks => &["DATABRICKS_HOST"],
            ProviderType::Google => &["GOOGLE_API_KEY"],
            ProviderType::Groq => &["GROQ_API_KEY"],
            ProviderType::Ollama => &[],
            ProviderType::OpenRouter => &["OPENROUTER_API_KEY"],
            ProviderType::GcpVertexAI => &["GCP_PROJECT_ID", "GCP_LOCATION"],
        }
    }

    fn pre_check(&self) -> Result<()> {
        match self {
            ProviderType::Ollama => {
                // Check if the `ollama ls` CLI command works
                use std::process::Command;
                let output = Command::new("ollama").arg("ls").output();
                if let Ok(output) = output {
                    if output.status.success() {
                        return Ok(()); // CLI is running
                    }
                }
                println!("Skipping Ollama tests - `ollama ls` command not found or failed");
                Err(anyhow::anyhow!("Ollama CLI is not running"))
            }
            _ => Ok(()), // Other providers don't need special pre-checks
        }
    }

    fn create_provider(&self, model_config: ModelConfig) -> Result<Box<dyn Provider>> {
        Ok(match self {
            ProviderType::Azure => Box::new(AzureProvider::from_env(model_config)?),
            ProviderType::OpenAi => Box::new(OpenAiProvider::from_env(model_config)?),
            ProviderType::Anthropic => Box::new(AnthropicProvider::from_env(model_config)?),
            ProviderType::Bedrock => Box::new(BedrockProvider::from_env(model_config)?),
            ProviderType::Databricks => Box::new(DatabricksProvider::from_env(model_config)?),
            ProviderType::GcpVertexAI => Box::new(GcpVertexAIProvider::from_env(model_config)?),
            ProviderType::Google => Box::new(GoogleProvider::from_env(model_config)?),
            ProviderType::Groq => Box::new(GroqProvider::from_env(model_config)?),
            ProviderType::Ollama => Box::new(OllamaProvider::from_env(model_config)?),
            ProviderType::OpenRouter => Box::new(OpenRouterProvider::from_env(model_config)?),
        })
    }
}

pub fn check_required_env_vars(required_vars: &[&str]) -> Result<()> {
    let missing_vars: Vec<&str> = required_vars
        .iter()
        .filter(|&&var| std::env::var(var).is_err())
        .cloned()
        .collect();

    if !missing_vars.is_empty() {
        println!(
            "Skipping tests. Missing environment variables: {:?}",
            missing_vars
        );
        return Err(anyhow::anyhow!("Required environment variables not set"));
    }
    Ok(())
}

async fn run_truncate_test(
    provider_type: ProviderType,
    model: &str,
    context_window: usize,
) -> Result<()> {
    let model_config = ModelConfig::new(model.to_string())
        .with_context_limit(Some(context_window))
        .with_temperature(Some(0.0));
    let provider = provider_type.create_provider(model_config)?;

    let agent = AgentFactory::create("truncate", provider).unwrap();
    let repeat_count = context_window + 10_000;
    let large_message_content = "hello ".repeat(repeat_count);
    let messages = vec![
        Message::user().with_text("hi there. what is 2 + 2?"),
        Message::assistant().with_text("hey! I think it's 4."),
        Message::user().with_text(&large_message_content),
        Message::assistant().with_text("heyy!!"),
        Message::user().with_text("what's the meaning of life?"),
        Message::assistant().with_text("the meaning of life is 42"),
        Message::user().with_text(
            "did I ask you what's 2+2 in this message history? just respond with 'yes' or 'no'",
        ),
    ];

    let reply_stream = agent.reply(&messages, None).await?;
    tokio::pin!(reply_stream);

    let mut responses = Vec::new();
    while let Some(response_result) = reply_stream.next().await {
        match response_result {
            Ok(response) => responses.push(response),
            Err(e) => {
                println!("Error: {:?}", e);
                return Err(e);
            }
        }
    }

    println!("Responses: {responses:?}\n");
    assert_eq!(responses.len(), 1);

    // Ollama and OpenRouter truncate by default even when the context window is exceeded
    // We don't have control over the truncation behavior in these providers
    if provider_type == ProviderType::Ollama || provider_type == ProviderType::OpenRouter {
        println!("WARNING: Skipping test for {:?} because it truncates by default when the context window is exceeded", provider_type);
        return Ok(());
    }

    assert_eq!(responses[0].content.len(), 1);

    let response_text = responses[0].content[0].as_text().unwrap();
    assert!(response_text.to_lowercase().contains("no"));
    assert!(!response_text.to_lowercase().contains("yes"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestConfig {
        provider_type: ProviderType,
        model: &'static str,
        context_window: usize,
    }

    async fn run_test_with_config(config: TestConfig) -> Result<()> {
        println!("Starting test for {config:?}");

        // Check for required environment variables
        if check_required_env_vars(config.provider_type.required_env()).is_err() {
            return Ok(()); // Skip test if env vars are missing
        }

        // Run provider-specific pre-checks
        if config.provider_type.pre_check().is_err() {
            return Ok(()); // Skip test if pre-check fails
        }

        // Run the truncate test
        run_truncate_test(config.provider_type, config.model, config.context_window).await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_openai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::OpenAi,
            model: "o3-mini-low",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_azure() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Azure,
            model: "gpt-4o-mini",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_anthropic() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Anthropic,
            model: "claude-3-5-haiku-latest",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_bedrock() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Bedrock,
            model: "anthropic.claude-3-5-sonnet-20241022-v2:0",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_databricks() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Databricks,
            model: "databricks-meta-llama-3-3-70b-instruct",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_databricks_bedrock() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Databricks,
            model: "claude-3-5-sonnet-2",
            context_window: 200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_databricks_openai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Databricks,
            model: "gpt-4o-mini",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_google() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Google,
            model: "gemini-2.0-flash-exp",
            context_window: 1_200_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_groq() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Groq,
            model: "gemma2-9b-it",
            context_window: 9_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_openrouter() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::OpenRouter,
            model: "deepseek/deepseek-r1",
            context_window: 130_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_ollama() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::Ollama,
            model: "llama3.2",
            context_window: 128_000,
        })
        .await
    }

    #[tokio::test]
    async fn test_truncate_agent_with_gcpvertexai() -> Result<()> {
        run_test_with_config(TestConfig {
            provider_type: ProviderType::GcpVertexAI,
            model: "claude-3-5-sonnet-v2@20241022",
            context_window: 200_000,
        })
        .await
    }
}
