use dotenv::dotenv;
use futures::StreamExt;
use goose::agents::AgentFactory;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::eternalai::{EternalAiProvider, ETERNAL_AI_DEFAULT_MODEL};

#[tokio::main]
async fn main() {
    // Setup a model provider from env vars
    let _ = dotenv();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let model = ModelConfig::new(ETERNAL_AI_DEFAULT_MODEL.parse().unwrap());
    let provider_result = EternalAiProvider::from_env(model, Some("8453".to_string()));

    let provider = match provider_result {
        Ok(provider) => provider,
        Err(e) => panic!("Failed to create provider: {}", e),
    };

    // Setup an agent
    let agent = AgentFactory::create("reference", Box::new(provider)).expect("default should exist");

    // println!("Extensions:");
    for extension in agent.list_extensions().await {
        println!("  {}", extension);
    }

    let messages = vec![Message::user()
        .with_text("Three people stand in a line. Each has a hat that is either red or blue, but they don’t know their own hat color. The first person says, “I don’t know my hat color.” The second person says, “I don’t know my hat color.” The third person immediately says, “I know my hat color.” What is the color of the third person’s hat? Why?")];

    let mut stream = agent.reply(&messages).await.unwrap();
    while let Some(message) = stream.next().await {
        println!(
            "{}",
            serde_json::to_string_pretty(&message.unwrap()).unwrap()
        );
        println!("\n");
    }
}
