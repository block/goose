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
