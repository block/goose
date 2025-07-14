use anyhow::Result;
use futures::future::join_all;
use goose_llm::{
    completion, configure_provider_pool, get_pool_stats, init_provider_pool, Message, ModelConfig,
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the provider pool with default configuration
    init_provider_pool();
    
    // Configure the pool with custom settings if desired
    // Maximum pool size: 5
    // Maximum idle time: 60 seconds
    // Maximum lifetime: 300 seconds (5 minutes)
    // Maximum uses: 50
    configure_provider_pool(5, 60, 300, 50);
    
    // Create a request template
    let model = ModelConfig::new("gpt-4o".to_string());
    
    // Get the OpenAI API key from environment variable
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    
    // Create the provider config
    let provider_config = json!({
        "api_key": api_key,
        "timeout": 60
    });
    
    // Create multiple concurrent requests
    let mut handles = Vec::new();
    for i in 0..3 {
        let model_clone = model.clone();
        let provider_config_clone = provider_config.clone();
        
        let handle = tokio::spawn(async move {
            // Create a simple message
            let messages = vec![Message::user().with_text(&format!(
                "Count from {} to {}",
                i * 5 + 1,
                i * 5 + 5
            ))];

            // Create a request
            let req = goose_llm::types::completion::create_completion_request(
                "openai",
                provider_config_clone,
                model_clone,
                Some("You are a helpful assistant.".to_string()),
                None,
                messages,
                vec![],
                Some(true), // use the provider pool
            );
            
            // Execute the request
            let response = completion(req).await?;
            
            // Print the response
            println!(
                "Request {}: {:?}\n",
                i,
                response.message.content[0]
            );
            
            Ok::<_, anyhow::Error>(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    join_all(handles).await;
    
    // Print pool statistics
    println!("\nProvider Pool Statistics:\n{}", get_pool_stats());
    
    Ok(())
}