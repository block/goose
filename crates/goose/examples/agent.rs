use std::sync::Arc;

use dotenv::dotenv;
use futures::StreamExt;
use goose::agents::{Agent, ExtensionConfig};
use goose::config::{DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::message::Message;
use goose::providers::databricks::DatabricksProvider;

#[tokio::main]
async fn main() {
    // Setup a model provider from env vars
    let _ = dotenv();

    let provider = Arc::new(DatabricksProvider::default());

    // Setup an agent with the developer extension
    let agent = Agent::new();
    let _ = agent.update_provider(provider).await;

    let config = ExtensionConfig::stdio(
        "developer",
        "./target/debug/developer",
        DEFAULT_EXTENSION_DESCRIPTION,
        DEFAULT_EXTENSION_TIMEOUT,
    );
    agent.add_extension(config).await.unwrap();

    println!("Extensions:");
    for extension in agent.list_extensions().await {
        println!("  {}", extension);
    }

    let messages = vec![Message::user()
        .with_text("can you summarize the readme.md in this dir using just a haiku?")];

    let mut stream = agent.reply(&messages, None).await.unwrap();
    while let Some(message) = stream.next().await {
        println!(
            "{}",
            serde_json::to_string_pretty(&message.unwrap()).unwrap()
        );
        println!("\n");
    }
}
