use anyhow::Result;
use dotenv::dotenv;
use goose::{
    message::Message,
    providers::{
        configs::{DatabricksProviderConfig, ProviderConfig},
        factory::get_provider,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Get required environment variables
    let host =
        std::env::var("DATABRICKS_HOST").expect("DATABRICKS_HOST environment variable is required");
    let model = std::env::var("DATABRICKS_MODEL")
        .expect("DATABRICKS_MODEL environment variable is required");

    // Create the Databricks provider configuration with OAuth
    let config = ProviderConfig::Databricks(DatabricksProviderConfig::with_oauth(host, model));

    // Create the provider
    let provider = get_provider(config)?;

    // Create a simple message
    let message = Message::user().with_text("Tell me a short joke about programming.");

    // Get a response
    let (response, usage) = provider
        .complete("You are a helpful assistant.", &[message], &[])
        .await?;

    // Print the response and usage statistics
    println!("\nResponse from AI:");
    println!("---------------");
    for content in response.content {
        dbg!(content);
    }
    println!("\nToken Usage:");
    println!("------------");
    println!("Input tokens: {:?}", usage.usage.input_tokens);
    println!("Output tokens: {:?}", usage.usage.output_tokens);
    println!("Total tokens: {:?}", usage.usage.total_tokens);

    Ok(())
}
