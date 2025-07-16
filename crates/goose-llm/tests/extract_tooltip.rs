use anyhow::Result;
use dotenv::dotenv;
use goose_llm::extractors::generate_tooltip;
use goose_llm::message::{Message, MessageContent, ToolRequest};
use goose_llm::providers::errors::ProviderError;
use goose_llm::types::core::{Content, ToolCall};
use serde_json::json;

fn should_run_test() -> Result<(), String> {
    dotenv().ok();
    if std::env::var("DATABRICKS_HOST").is_err() {
        return Err("Missing DATABRICKS_HOST".to_string());
    }
    if std::env::var("DATABRICKS_TOKEN").is_err() {
        return Err("Missing DATABRICKS_TOKEN".to_string());
    }
    Ok(())
}

async fn _generate_tooltip(messages: &[Message]) -> Result<String, ProviderError> {
    let provider_name = "databricks";
    let provider_config = serde_json::json!({
        "host": std::env::var("DATABRICKS_HOST").expect("Missing DATABRICKS_HOST"),
        "token": std::env::var("DATABRICKS_TOKEN").expect("Missing DATABRICKS_TOKEN"),
    });

    generate_tooltip(provider_name, provider_config, messages, None).await
}

#[tokio::test]
async fn test_generate_tooltip_simple() {
    if should_run_test().is_err() {
        println!("Skipping...");
        return;
    }

    // Two plain-text messages
    let messages = vec![
        Message::user().with_text("Hello, how are you?"),
        Message::assistant().with_text("I'm fine, thanks! How can I help?"),
    ];

    let tooltip = _generate_tooltip(&messages)
        .await
        .expect("Failed to generate tooltip");
    println!("Generated tooltip: {:?}", tooltip);

    assert!(!tooltip.trim().is_empty(), "Tooltip must not be empty");
    assert!(
        tooltip.len() < 100,
        "Tooltip should be reasonably short (<100 chars)"
    );
}

#[tokio::test]
async fn test_generate_tooltip_with_tools() {
    if should_run_test().is_err() {
        println!("Skipping...");
        return;
    }

    // 1) Assistant message with a tool request
    let mut tool_req_msg = Message::assistant();
    let req = ToolRequest {
        id: "1".to_string(),
        tool_call: Ok(ToolCall::new("get_time", json!({"timezone": "UTC"}))).into(),
    };
    tool_req_msg.content.push(MessageContent::ToolReq(req));

    // 2) User message with the tool response
    let tool_resp_msg = Message::user().with_tool_response(
        "1",
        Ok(vec![Content::text("The current time is 12:00 UTC")]).into(),
    );

    let messages = vec![tool_req_msg, tool_resp_msg];

    let tooltip = _generate_tooltip(&messages)
        .await
        .expect("Failed to generate tooltip");
    println!("Generated tooltip (tools): {:?}", tooltip);

    assert!(!tooltip.trim().is_empty(), "Tooltip must not be empty");
    assert!(
        tooltip.len() < 100,
        "Tooltip should be reasonably short (<100 chars)"
    );
}
