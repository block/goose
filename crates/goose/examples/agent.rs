use dotenv::dotenv;
use futures::StreamExt;
use goose::agents::{AgentFactory, SystemConfig};
use goose::message::Message;
use goose::providers::configs::{DatabricksProviderConfig, ProviderConfig};
use goose::providers::get_provider;

#[tokio::main]
async fn main() {
    // Setup a model provider from env vars
    let _ = dotenv();
    let host =
        std::env::var("DATABRICKS_HOST").expect("DATABRICKS_HOST environment variable is required");
    let model = std::env::var("DATABRICKS_MODEL")
        .expect("DATABRICKS_MODEL environment variable is required");

    let config = ProviderConfig::Databricks(DatabricksProviderConfig::with_oauth(host, model));
    let provider = get_provider(config).expect("should create provider");

    // Setup an agent with the developer system
    let mut agent = AgentFactory::create("default", provider).expect("default should exist");

    let config = SystemConfig::stdio("./target/debug/developer");
    agent.add_system(config).await.unwrap();

    println!("Systems:");
    for system in agent.list_systems().await {
        println!("  {}", system);
    }

    let messages = vec![Message::user()
        .with_text("can you summarize the readme.md in this dir using just a haiku?")];

    let mut stream = agent.reply(&messages).await.unwrap();
    while let Some(message) = stream.next().await {
        println!(
            "{}",
            serde_json::to_string_pretty(&message.unwrap()).unwrap()
        );
        println!("\n");
    }
}
