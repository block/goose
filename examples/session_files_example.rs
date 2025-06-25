use goose::message::{Message, MessageContent, SessionFile};
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

    // Create a message with session files
    let message = Message::user()
        .with_text("Please analyze these files and tell me what they contain")
        .with_session_files(vec![
            SessionFile { 
                id: "file1".to_string(),
                path: "path/to/file1.txt".to_string(), 
                file_type: "file".to_string(),
                data_url: None,
                file_path: None,
                is_loading: None,
                error: None,
            },
            SessionFile { 
                id: "file2".to_string(),
                path: "path/to/file2.py".to_string(), 
                file_type: "file".to_string(),
                data_url: None,
                file_path: None,
                is_loading: None,
                error: None,
            },
            SessionFile { 
                id: "file3".to_string(),
                path: "path/to/config.json".to_string(), 
                file_type: "file".to_string(),
                data_url: None,
                file_path: None,
                is_loading: None,
                error: None,
            },
        ]);

    println!("Created message with session files:");
    println!("Role: {:?}", message.role);
    println!("Content count: {}", message.content.len());
    
    // Print the session files content
    for content in &message.content {
        match content {
            MessageContent::Text(text) => {
                println!("Text content: {}", text.text);
            }
            MessageContent::SessionFiles(session_files) => {
                println!("Session files:");
                for file in &session_files.files {
                    println!("  - {} ({})", file.path, file.file_type);
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

    println!("\nThe SessionFiles content will be converted to text when sent to the LLM:");
    println!("'The following files have been added to the context:'");
    println!("followed by the list of file paths.");

    Ok(())
} 