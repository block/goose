use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::Provider;
use crate::providers::databricks::DatabricksProvider;
use crate::providers::errors::ProviderError;
use crate::types::core::Role;
use anyhow::Result;
use serde_json::{json, Value};

/// Generates a short (≤4 words) description of the session using Databricks “goose-gpt-4-1”.
pub async fn generate_session_description(messages: &[Message]) -> Result<String, ProviderError> {
    // Build the instruction prompt
    let system_prompt =
        "Based on the user messages in the conversation so far, provide a concise description \
         of this session in four words or less."
            .to_string();

    // Collect up to the first 3 user messages (truncated to 300 chars each)
    let context: Vec<String> = messages
        .iter()
        .filter(|m| m.role == Role::User)
        .take(3)
        .map(|m| {
            let text = m.content.concat_text_str();
            if text.len() > 300 {
                text.chars().take(300).collect()
            } else {
                text
            }
        })
        .collect();

    if context.is_empty() {
        return Err(ProviderError::ExecutionError(
            "No user messages found to generate a description.".to_string(),
        ));
    }

    let user_msg_text = format!(
        "Here are the first few user messages:\n{}",
        context.join("\n")
    );

    // Instantiate DatabricksProvider with goose-gpt-4-1
    let model_cfg = ModelConfig::new("goose-gpt-4-1".to_string()).with_temperature(Some(0.0));
    let provider = DatabricksProvider::from_env(model_cfg)?;

    // Use `extract` with a simple string schema
    let schema = json!({
        "type": "object",
        "properties": {
            "description": { "type": "string" }
        },
        "required": ["description"],
        "additionalProperties": false
    });
    let user_msg = Message::user().with_text(&user_msg_text);
    let resp = provider
        .extract(&system_prompt, &[user_msg], &schema)
        .await?;

    let obj = resp
        .data
        .as_object()
        .ok_or_else(|| ProviderError::ResponseParseError("Expected object".into()))?;

    let description = obj
        .get("description")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProviderError::ResponseParseError("Missing or non-string description".into())
        })?
        .to_string();

    Ok(description)
}
