use dotenv::dotenv;
use futures::StreamExt;
use goose::agents::AgentFactory;
use goose::message::Message;
use goose::providers::eternalai::EternalAiProvider;

#[tokio::main]
async fn main() {
    // Setup a model provider from env vars
    let _ = dotenv();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    let provider = Box::new(EternalAiProvider::default());

    // Setup an agent
    let agent = AgentFactory::create("reference", provider).expect("default should exist");

    // println!("Extensions:");
    for extension in agent.list_extensions().await {
        println!("  {}", extension);
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
