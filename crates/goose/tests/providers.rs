use anyhow::Result;
use dotenv::dotenv;
use goose::message::{Message, MessageContent};
use goose::providers::configs::OllamaProviderConfig;
use goose::providers::ollama::{OLLAMA_HOST, OLLAMA_MODEL};
use mcp_core::tool::Tool;

use goose::providers::{
    base::Provider,
    configs::{
        DatabricksAuth, DatabricksProviderConfig, ModelConfig, OpenAiProviderConfig, ProviderConfig,
    },
    factory::get_provider,
};

/// Generic test harness for any Provider implementation
struct ProviderTester {
    provider: Box<dyn Provider + Send + Sync>,
}

impl ProviderTester {
    fn new(config: ProviderConfig) -> Result<Self> {
        Ok(Self {
            provider: get_provider(config)?,
        })
    }

    async fn test_basic_response(&self) -> Result<()> {
        let message = Message::user().with_text("Just say hello!");

        let (response, _) = self
            .provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await?;

        // For a basic response, we expect a single text response
        assert_eq!(
            response.content.len(),
            1,
            "Expected single content item in response"
        );

        // Verify we got a text response
        assert!(
            matches!(response.content[0], MessageContent::Text(_)),
            "Expected text response"
        );

        Ok(())
    }

    async fn test_tool_usage(&self) -> Result<()> {
        let weather_tool = Tool::new(
            "get_weather",
            "Get the weather for a location",
            serde_json::json!({
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    }
                }
            }),
        );

        let message = Message::user().with_text("What's the weather like in San Francisco?");

        let (response, _) = self
            .provider
            .complete(
                "You are a helpful weather assistant.",
                &[message],
                &[weather_tool],
            )
            .await?;

        // Verify we got a tool request
        assert!(
            response
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::ToolRequest(_))),
            "Expected tool request in response"
        );

        Ok(())
    }

    /// Run all provider tests
    async fn run_test_suite(&self) -> Result<()> {
        self.test_basic_response().await?;
        self.test_tool_usage().await?;
        Ok(())
    }
}

fn load_env() {
    if let Ok(path) = dotenv() {
        println!("Loaded environment from {:?}", path);
    }
}

#[tokio::test]
async fn test_openai_provider() -> Result<()> {
    load_env();

    // Skip if credentials aren't available
    if std::env::var("OPENAI_API_KEY").is_err() || std::env::var("OPENAI_MODEL").is_err() {
        println!("Skipping OpenAI tests - credentials not configured");
        return Ok(());
    }

    let config = ProviderConfig::OpenAi(OpenAiProviderConfig {
        host: "https://api.openai.com".to_string(),
        api_key: std::env::var("OPENAI_API_KEY")?,
        model: ModelConfig::new(std::env::var("OPENAI_MODEL")?),
    });

    let tester = ProviderTester::new(config)?;
    tester.run_test_suite().await?;

    Ok(())
}

#[tokio::test]
async fn test_databricks_provider() -> Result<()> {
    load_env();

    // Skip if credentials aren't available
    if std::env::var("DATABRICKS_HOST").is_err()
        || std::env::var("DATABRICKS_TOKEN").is_err()
        || std::env::var("DATABRICKS_MODEL").is_err()
    {
        println!("Skipping Databricks tests - credentials not configured");
        return Ok(());
    }

    let config = ProviderConfig::Databricks(DatabricksProviderConfig {
        host: std::env::var("DATABRICKS_HOST")?,
        model: ModelConfig::new(std::env::var("DATABRICKS_MODEL")?),
        auth: DatabricksAuth::Token(std::env::var("DATABRICKS_TOKEN")?),
        image_format: goose::providers::utils::ImageFormat::Anthropic,
    });

    let tester = ProviderTester::new(config)?;
    tester.run_test_suite().await?;

    Ok(())
}

#[tokio::test]
async fn test_databricks_provider_oauth() -> Result<()> {
    load_env();

    // Skip if credentials aren't available
    if std::env::var("DATABRICKS_HOST").is_err() || std::env::var("DATABRICKS_MODEL").is_err() {
        println!("Skipping Databricks OAuth tests - credentials not configured");
        return Ok(());
    }

    let config = ProviderConfig::Databricks(DatabricksProviderConfig {
        host: std::env::var("DATABRICKS_HOST")?,
        model: ModelConfig::new(std::env::var("DATABRICKS_MODEL")?),
        auth: DatabricksAuth::oauth(std::env::var("DATABRICKS_HOST")?),
        image_format: goose::providers::utils::ImageFormat::Anthropic,
    });

    let tester = ProviderTester::new(config)?;
    tester.run_test_suite().await?;

    Ok(())
}

// Integration tests that run against a real Ollama server
#[tokio::test]
async fn test_ollama_provider() -> Result<()> {
    load_env();

    // Skip if model isin't set (specified in ci)
    if std::env::var("OLLAMA_MODEL").is_err() {
        println!("Skipping Ollama tests - model not configured");
        return Ok(());
    }

    let config = ProviderConfig::Ollama(OllamaProviderConfig {
        host: std::env::var("OLLAMA_HOST").unwrap_or_else(|_| String::from(OLLAMA_HOST)),
        model: ModelConfig::new(
            std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| String::from(OLLAMA_MODEL)),
        ),
    });

    let tester = ProviderTester::new(config)?;
    tester.run_test_suite().await?;

    Ok(())
}
