use crate::message::Message;
use crate::model::ModelConfig;
use crate::providers::base::Usage;
use anyhow::Result;
use mcp_core::tool::Tool;
use serde_json::Value;

use super::anthropic;

pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    match model_config.model_name.as_str() {
        "claude-3-5-sonnet-v2@20241022" | "claude-3-5-sonnet@20240620" => {
            create_anthropic_request(model_config, system, messages, tools)
        }
        _ => Err(anyhow::anyhow!("Vertex AI only supports Anthropic models")),
    }
}

pub fn create_anthropic_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let mut request = anthropic::create_request(model_config, system, messages, tools)?;

    // the Vertex AI for Claude API has small differences from the Anthropic API
    // ref: https://docs.anthropic.com/en/api/claude-on-vertex-ai
    request.as_object_mut().unwrap().remove("model");
    request.as_object_mut().unwrap().insert(
        "anthropic_version".to_string(),
        Value::String("vertex-2023-10-16".to_string()),
    );

    Ok(request)
}

pub fn response_to_message(response: Value) -> Result<Message> {
    anthropic::response_to_message(response)
}

pub fn get_usage(data: &Value) -> Result<Usage> {
    anthropic::get_usage(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_request() {
        let model_config = ModelConfig::new("claude-3-5-sonnet-v2@20241022".to_string());
        let system = "You are a helpful assistant.";
        let messages = vec![Message::user().with_text("Hello, how are you?")];
        let tools = vec![];

        let request = create_request(&model_config, &system, &messages, &tools).unwrap();

        assert!(request.get("anthropic_version").is_some());
        assert!(request.get("model").is_none());
    }
}
