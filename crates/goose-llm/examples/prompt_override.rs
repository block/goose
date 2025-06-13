use std::vec;

use anyhow::Result;
use goose_llm::{
    completion,
    types::completion::{CompletionRequest, CompletionResponse},
    Message, ModelConfig,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = "databricks";
    let provider_config = json!({
        "host": std::env::var("DATABRICKS_HOST").expect("Missing DATABRICKS_HOST"),
        "token": std::env::var("DATABRICKS_TOKEN").expect("Missing DATABRICKS_TOKEN"),
    });
    // let model_name = "goose-gpt-4-1"; // parallel tool calls
    let model_name = "claude-3-5-haiku";
    let model_config = ModelConfig::new(model_name.to_string());

    let system_prompt_override = "You are a helpful assistant. Talk in the style of pirates.";

    for text in ["How was your day?"] {
        println!("\n---------------\n");
        println!("User Input: {text}");
        let messages = vec![
            Message::user().with_text("Hi there!"),
            Message::assistant().with_text("How can I help?"),
            Message::user().with_text(text),
        ];
        let completion_response: CompletionResponse = completion(CompletionRequest::new(
            provider.to_string(),
            provider_config.clone(),
            model_config.clone(),
            None,
            Some(system_prompt_override.to_string()),
            messages.clone(),
            vec![],
        ))
        .await?;
        // Print the response
        println!("\nCompletion Response:");
        println!("{}", serde_json::to_string_pretty(&completion_response)?);
    }

    Ok(())
}
