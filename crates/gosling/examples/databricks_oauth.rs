use anyhow::Result;
use dotenvy::dotenv;
use gosling::conversation::message::Message;
use gosling::providers::create_with_named_model;
use gosling::providers::databricks::DATABRICKS_DEFAULT_MODEL;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    std::env::remove_var("DATABRICKS_TOKEN");

    let provider = create_with_named_model("databricks", Vec::new()).await?;

    let message = Message::user().with_text("Tell me a short joke about programming.");

    let model_config = gosling::model_config::model_config_from_user_config(
        "databricks",
        DATABRICKS_DEFAULT_MODEL,
    )?;
    let (response, usage) = provider
        .complete(
            &model_config,
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
