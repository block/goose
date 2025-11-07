use dotenvy::dotenv;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent, ExtensionConfig, SessionConfig};
use goose::config::{DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use goose::conversation::message::Message;
use goose::providers::create_with_named_model;
use goose::providers::databricks::DATABRICKS_DEFAULT_MODEL;
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use std::path::PathBuf;

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

    let session = SessionManager::create_session(
        PathBuf::default(),
        "max-turn-test".to_string(),
        SessionType::Hidden,
    )
    .await
    .expect("session manager creation failed");

    let session_config = SessionConfig {
        id: session.id,
        schedule_id: None,
        max_turns: None,
        retry_config: None,
    };

    let user_message = Message::user()
        .with_text("can you summarize the readme.md in this dir using just a haiku?");

    let mut stream = agent
        .reply(user_message, session_config, None)
        .await
        .unwrap();

    while let Some(Ok(AgentEvent::Message(message))) = stream.next().await {
        println!("{}", serde_json::to_string_pretty(&message).unwrap());
        println!("\n");
    }
}
