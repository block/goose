use std::vec;

use anyhow::Result;
use goose_llm::{
    completion,
    extractors::generate_tooltip,
    types::completion::{
        CompletionRequest, CompletionResponse, ExtensionConfig, ToolApprovalMode, ToolConfig,
    },
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

    let list_dir_tool = ToolConfig::new(
        "list_directory",
        "List files in a directory",
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list files from"
                }
            }
        }),
        ToolApprovalMode::Auto,
    );

    let extensions = vec![
        ExtensionConfig::new(
            "calculator_extension".to_string(),
            Some("This extension provides a calculator tool.".to_string()),
            vec![calculator_tool],
        ),
        ExtensionConfig::new(
            "bash_extension".to_string(),
            Some("This extension provides a bash shell tool.".to_string()),
            vec![bash_tool, list_dir_tool],
        ),
    ];

    let system_preamble = "You are a helpful assistant.";

    for text in [
        "Add 10037 + 23123 using calculator and also run 'date -u' using bash",
        "List all files in the current directory",
    ] {
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
            Some(system_preamble.to_string()),
            None,
            messages.clone(),
            extensions.clone(),
        ))
        .await?;
        // Print the response
        println!("\nCompletion Response:");
        println!("{}", serde_json::to_string_pretty(&completion_response)?);

        let tooltip = generate_tooltip(provider, provider_config.clone(), &messages).await?;
        println!("\nTooltip: {}", tooltip);
    }

    Ok(())
}
