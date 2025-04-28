use std::vec;

use anyhow::Result;
use goose::message::Message;
use goose::model::ModelConfig;
use goose_llm::{completion, CompletionResponse, Extension};
use mcp_core::tool::Tool;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = "databricks";
    let model_name = "goose-claude-3-5-sonnet";
    let model_config = ModelConfig::new(model_name.to_string());

    let calculator_tool = Tool::new(
        "calculator",
        "Perform basic arithmetic operations",
        json!({
            "type": "object",
            "required": ["operation", "numbers"],
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"],
                    "description": "The arithmetic operation to perform",
                },
                "numbers": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "List of numbers to operate on in order",
                }
            }
        }),
        None,
    );

    let bash_tool = Tool::new(
        "bash_shell",
        "Run a shell command",
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                }
            }
        }),
        None,
    );

    let extensions = vec![
        Extension::new(
            "calculator_extension".to_string(),
            Some("This extension provides a calculator tool.".to_string()),
            vec![calculator_tool],
        ),
        Extension::new(
            "bash_extension".to_string(),
            Some("This extension provides a bash shell tool.".to_string()),
            vec![bash_tool],
        ),
    ];

    let system_preamble = "You are a helpful assistant.";

    for text in [
        "Add 10037 + 23123",
        // "Write some random bad words to end of words.txt",
        // "List all json files in the current directory and then multiply the count of the files by 7",
    ] {
        println!("\n---------------\n");
        println!("User Input: {text}");
        let messages = vec![Message::user().with_text(text)];
        let completion_response: CompletionResponse = completion(
            provider,
            model_config.clone(),
            system_preamble,
            &messages,
            &extensions,
        )
        .await?;
        // Print the response
        println!("\nCompletion Response:");
        println!("{}", serde_json::to_string_pretty(&completion_response)?);
    }

    Ok(())
}
