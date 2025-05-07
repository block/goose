use anyhow::Result;
use dotenv::dotenv;
use goose_llm::extractors::generate_tooltip;
use goose_llm::message::{Message, MessageContent, ToolRequest};
use goose_llm::providers::errors::ProviderError;
use goose_llm::types::core::{Content, ToolCall};
use serde_json::json;

#[tokio::test]
async fn test_generate_tooltip_simple() -> Result<(), ProviderError> {
    // Skip if no Databricks creds
    dotenv().ok();
    if std::env::var("DATABRICKS_HOST").is_err() || std::env::var("DATABRICKS_TOKEN").is_err() {
        println!("Skipping simple tooltip test – Databricks creds not set");
        return Ok(());
    }

    // Two plain-text messages
    let messages = vec![
        Message::user().with_text("Hello, how are you?"),
        Message::assistant().with_text("I'm fine, thanks! How can I help?"),
    ];

    let tooltip = generate_tooltip(&messages).await?;
    println!("Generated tooltip: {:?}", tooltip);

    assert!(!tooltip.trim().is_empty(), "Tooltip must not be empty");
    assert!(
        tooltip.len() < 100,
        "Tooltip should be reasonably short (<100 chars)"
    );
    Ok(())
}

#[tokio::test]
async fn test_generate_tooltip_with_tools() -> Result<(), ProviderError> {
    // Skip if no Databricks creds
    dotenv().ok();
    if std::env::var("DATABRICKS_HOST").is_err() || std::env::var("DATABRICKS_TOKEN").is_err() {
        println!("Skipping tool‐based tooltip test – Databricks creds not set");
        return Ok(());
    }

    // 1) Assistant message with a tool request
    let mut tool_req_msg = Message::assistant();
    let req = ToolRequest {
        id: "1".to_string(),
        tool_call: Ok(ToolCall::new("get_time", json!({"timezone": "UTC"}))),
    };
    tool_req_msg.content.push(MessageContent::ToolRequest(req));

    // 2) User message with the tool response
    let tool_resp_msg = Message::user().with_tool_response(
        "1",
        Ok(vec![Content::text("The current time is 12:00 UTC")]),
    );

    let messages = vec![tool_req_msg, tool_resp_msg];

    let tooltip = generate_tooltip(&messages).await?;
    println!("Generated tooltip (tools): {:?}", tooltip);

    assert!(!tooltip.trim().is_empty(), "Tooltip must not be empty");
    assert!(
        tooltip.len() < 100,
        "Tooltip should be reasonably short (<100 chars)"
    );
    Ok(())
}
