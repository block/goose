use dotenv::dotenv;
use futures::StreamExt;
use goose::agents::{AgentFactory, ExtensionConfig};
use goose::config::{DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::message::Message;
use goose::providers::githubcopilot::GithubCopilotProvider;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Setup a model provider from env vars
    let _ = dotenv();

    let provider = Box::new(GithubCopilotProvider::default());

    // Setup an agent with the developer extension
    let mut agent = AgentFactory::create("reference", provider).expect("default should exist");

    let config = ExtensionConfig::stdio(
        "mcp-server",
        "./target/debug/mcp-server",
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
