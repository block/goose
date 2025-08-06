use std::sync::Arc;

use dotenvy::dotenv;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent, ExtensionConfig};
use goose::config::{DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::conversation::message::Message;
use goose::conversation::Conversation;
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
        "./target/debug/goose",
        DEFAULT_EXTENSION_DESCRIPTION,
        DEFAULT_EXTENSION_TIMEOUT,
    )
    .with_args(vec!["mcp", "developer"]);
    agent.add_extension(config).await.unwrap();

    println!("Extensions:");
    for extension in agent.list_extensions().await {
        println!("  {}", extension);
    }

    let conversation = Conversation::new(vec![Message::user()
        .with_text("can you summarize the readme.md in this dir using just a haiku?")])
    .unwrap();

    let mut stream = agent.reply(conversation, None, None).await.unwrap();
    while let Some(Ok(AgentEvent::Message(message))) = stream.next().await {
        println!("{}", serde_json::to_string_pretty(&message).unwrap());
        println!("\n");
    }
}
