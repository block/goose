use anyhow::Result;
use dotenvy::dotenv;
use goose::conversation::message::Message;
use goose::providers::{
    base::{Provider, Usage},
    databricks::DatabricksProvider,
};
use tokio_stream::StreamExt;

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
    let mut stream = provider
        .stream("You are a helpful assistant.", &[message], &[])
        .await?;

    println!("\nResponse from AI:");
    println!("---------------");
    let mut usage = Usage::default();
    while let Some(Ok((msg, usage_part))) = stream.next().await {
        dbg!(msg);
        usage_part.map(|u| {
            usage += u.usage;
        });
    }
    println!("\nToken Usage:");
    println!("------------");
    println!("Input tokens: {:?}", usage.input_tokens);
    println!("Output tokens: {:?}", usage.output_tokens);
    println!("Total tokens: {:?}", usage.total_tokens);

    Ok(())
}
