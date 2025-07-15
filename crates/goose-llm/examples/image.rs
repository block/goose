use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use goose_llm::{
    completion,
    message::MessageContent,
    types::completion::{CompletionRequest, CompletionResponse},
    Message, ModelConfig,
};
use serde_json::json;
use std::{fs, vec};

#[tokio::main]
async fn main() -> Result<()> {
    let provider = "databricks";
    let provider_config = json!({
        "host": std::env::var("DATABRICKS_HOST").expect("Missing DATABRICKS_HOST"),
        "token": std::env::var("DATABRICKS_TOKEN").expect("Missing DATABRICKS_TOKEN"),
    });
    let model_name = "goose-claude-4-sonnet"; // "gpt-4o";
    let model_config = ModelConfig::new(model_name.to_string());

    let system_preamble = "You are a helpful assistant.";

    // Read and encode test image
    let image_data = fs::read("examples/test_assets/test_image.png")?;
    let base64_image = BASE64.encode(image_data);

    let user_msg = Message::user()
        .with_text("What do you see in this image?")
        .with_content(MessageContent::image(base64_image, "image/png"));

    let messages = vec![user_msg];

    let completion_response: CompletionResponse = completion(
        CompletionRequest::new(
            provider.to_string(),
            provider_config.clone(),
            model_config.clone(),
            Some(system_preamble.to_string()),
            None,
            messages,
            vec![],
        )
        .with_request_id("test-image-1".to_string()),
    )
    .await?;

    // Print the response
    println!("\nCompletion Response:");
    println!("{}", serde_json::to_string_pretty(&completion_response)?);

    Ok(())
}
