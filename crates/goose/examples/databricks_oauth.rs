use anyhow::Result;
use dotenvy::dotenv;
use goose::conversation::message::Message;
use goose::providers::create_with_named_model;
use goose::providers::databricks::DATABRICKS_DEFAULT_MODEL;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Clear any token to force OAuth
    std::env::remove_var("DATABRICKS_TOKEN");

    // Create the provider
    let provider = create_with_named_model("databricks", DATABRICKS_DEFAULT_MODEL).await?;

    // Create a simple message
    let message = Message::user().with_text("Tell me a short joke about programming.");

    // Get a response
    let (response, usage) = provider
        .complete(
            "",  // Empty session ID for example
            "You are a helpful assistant.",
            &[message],
            &[],
        )
        .await?;

    println!("\nResponse from AI:");
    println!("---------------");
    println!("{:?}", response);

    println!("\nToken Usage:");
    println!("------------");
    println!("Input tokens: {:?}", usage.usage.input_tokens);
    println!("Output tokens: {:?}", usage.usage.output_tokens);
    println!("Total tokens: {:?}", usage.usage.total_tokens);

    Ok(())
}
