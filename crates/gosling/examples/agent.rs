use dotenvy::dotenv;
use futures::StreamExt;
use gosling::agents::{Agent, AgentEvent, ExtensionConfig, SessionConfig};
use gosling::config::{GoslingMode, DEFAULT_EXTENSION_DESCRIPTION, DEFAULT_EXTENSION_TIMEOUT};
use gosling::conversation::message::Message;
use gosling::providers::create_with_named_model;
use gosling::providers::databricks::DATABRICKS_DEFAULT_MODEL;
use gosling::session::session_manager::SessionType;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv();

    let provider = create_with_named_model("databricks", Vec::new()).await?;
    let model_config = gosling::model_config::model_config_from_user_config(
        "databricks",
        DATABRICKS_DEFAULT_MODEL,
    )?;

    let agent = Agent::new();

    let session = agent
        .config
        .session_manager
        .create_session(
            PathBuf::default(),
            "max-turn-test".to_string(),
            SessionType::Hidden,
            GoslingMode::default(),
        )
        .await?;

    agent
        .update_provider(provider, model_config, &session.id)
        .await?;

    let config = ExtensionConfig::stdio(
        "developer",
        "./target/debug/gosling",
        DEFAULT_EXTENSION_DESCRIPTION,
        DEFAULT_EXTENSION_TIMEOUT,
    )
    .with_args(vec!["mcp", "developer"]);
    agent.add_extension(config, &session.id).await?;

    println!("Extensions:");
    for extension in agent.list_extensions().await {
        println!("  {}", extension);
    }

    let session_config = SessionConfig {
        id: session.id,
        max_turns: None,
    };

    let user_message = Message::user()
        .with_text("can you summarize the readme.md in this dir using just a haiku?");

    let mut stream = agent.reply(user_message, session_config, None).await?;

    while let Some(Ok(AgentEvent::Message(message))) = stream.next().await {
        println!("{}", serde_json::to_string_pretty(&message)?);
        println!("\n");
    }

    Ok(())
}
