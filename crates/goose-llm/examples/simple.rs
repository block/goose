use std::vec;

use anyhow::Result;
use goose_llm::{
    Message, ModelConfig, completion,
    types::completion::{
        CompletionResponse, ExtensionConfig, ExtensionType, ToolApprovalMode, ToolConfig,
    },
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = "databricks";
    // let model_name = "goose-claude-3-5-sonnet"; // sequential tool calls
    let model_name = "goose-gpt-4-1"; // parallel tool calls
    let model_config = ModelConfig::new(model_name.to_string());

    let calculator_tool = ToolConfig::new(
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
        ToolApprovalMode::Auto,
    );

    let bash_tool = ToolConfig::new(
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
        ToolApprovalMode::Manual,
    );

    let extensions = vec![
        ExtensionConfig::new(
            "calculator_extension".to_string(),
            Some("This extension provides a calculator tool.".to_string()),
            vec![calculator_tool],
            ExtensionType::McpHttp,
        ),
        ExtensionConfig::new(
            "bash_extension".to_string(),
            Some("This extension provides a bash shell tool.".to_string()),
            vec![bash_tool],
            ExtensionType::Frontend,
        ),
    ];

    let system_preamble = "You are a helpful assistant.";

    for text in [
        "Add 10037 + 23123 using calculator and also run 'date -u' using bash",
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
