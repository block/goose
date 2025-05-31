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
    let model_name = "claude-3-5-haiku";
    let model_config = ModelConfig::new(model_name.to_string());
    let system_preamble = "";
    let messages = vec![Message::user().with_text("what your favorite ice cream")];

    for i in 0..100 {
        let completion_response: CompletionResponse = completion(CompletionRequest::new(
            provider.to_string(),
            provider_config.clone(),
            model_config.clone(),
            system_preamble.to_string(),
            messages.clone(),
            vec![].clone(),
        ))
        .await?;

        let serialized = serde_json::to_string_pretty(&completion_response)?;

        // Check for "type" & "text" in the response
        let concat_text = &completion_response.message.content.concat_text_str();
        if concat_text.contains("type") && concat_text.contains("text") {
            println!("Suspected JSON response: {}", concat_text);
            println!(
                "\n{} Completion: {}\n",
                i,
                serialized
            );
        }

        if i % 10 == 0 {
            println!(
                "\n{} [log] Completion: {}\n",
                i,
                serialized
            );
        }
    }

    Ok(())
}
