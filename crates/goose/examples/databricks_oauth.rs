use anyhow::Result;
use dotenv::dotenv;
use goose::{
    message::Message,
    providers::{base::Provider, databricks::DatabricksProvider},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();

    // Clear any token to force OAuth
    std::env::remove_var("DATABRICKS_TOKEN");

    // Create the provider
    let provider = DatabricksProvider::default();

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
