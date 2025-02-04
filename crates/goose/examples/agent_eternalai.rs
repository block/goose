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
