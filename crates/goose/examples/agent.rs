use dotenvy::dotenv;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent, ExtensionConfig};
use goose::config::{DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::conversation::message::Message;
use goose::conversation::Conversation;
use goose::providers::create_with_named_model;
use goose::providers::databricks::DATABRICKS_DEFAULT_MODEL;

#[tokio::main]
async fn main() {
    let _ = dotenv();

    let provider = create_with_named_model("databricks", DATABRICKS_DEFAULT_MODEL)
        .await
        .expect("Couldn't create provider");

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

    let user_message = Message::user()
        .with_text("can you summarize the readme.md in this dir using just a haiku?");
    let conversation = Conversation::new(vec![user_message.clone()]).unwrap();

    let mut stream = agent.reply(user_message, None, None).await.unwrap();
    while let Some(Ok(AgentEvent::Message(message))) = stream.next().await {
        println!("{}", serde_json::to_string_pretty(&message).unwrap());
        println!("\n");
    }
}
