use goose::message::{Message, MessageContent};
use goose::providers::base::Provider;
use goose::providers::factory::create_provider;
use goose::model::ModelConfig;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple model config (you'll need to set up your own provider config)
    let model_config = ModelConfig::new("gpt-4o".to_string());
    
    // Create a provider (you'll need to set up your API keys)
    let provider = create_provider("openai", &model_config)?;
    let provider = Arc::new(provider);

    // Create a message with context files
    let message = Message::user()
        .with_text("Please analyze these files and tell me what they contain")
        .with_context_files(vec![
            ContextPathItem { path: "path/to/file1.txt".to_string(), path_type: "file".to_string() },
            ContextPathItem { path: "path/to/file2.py".to_string(), path_type: "file".to_string() },
            ContextPathItem { path: "path/to/config.json".to_string(), path_type: "file".to_string() },
        ]);

    println!("Created message with context files:");
    println!("Role: {:?}", message.role);
    println!("Content count: {}", message.content.len());
    
    // Print the context files content
    for content in &message.content {
        match content {
            MessageContent::Text(text) => {
                println!("Text content: {}", text.text);
            }
            MessageContent::ContextPaths(context_files) => {
                println!("Context files:");
                for path_item in &context_files.paths {
                    println!("  - {} ({})", path_item.path, path_item.path_type);
                }
            }
            _ => {
                println!("Other content type: {:?}", content);
            }
        }
    }

    // Note: To actually send this to an LLM, you would use:
    // let response = provider.complete("You are a helpful assistant.", &[message], &[]).await?;
    // println!("LLM Response: {}", response.0.as_concat_text());

    println!("\nThe ContextPaths content will be converted to text when sent to the LLM:");
    println!("'The following files have been added to the context:'");
    println!("followed by the list of file paths.");

    Ok(())
} 