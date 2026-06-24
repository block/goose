use anyhow::Result;
use futures::StreamExt;
use goose_providers::{
    base::Provider, conversation::message::Message, declarative::EnvKeyResolver, model::ModelConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    let json = include_str!("deepseek.json");
    let provider = goose_providers::declarative::from_json(json, None, EnvKeyResolver {})?;

    let system = "You are a knowledgable geography expert";
    let messages = [Message::user().with_text("what is the capital of France?")];

    let model = ModelConfig::new("deepseek-v4-flash");
    let mut stream = provider
        .stream(
            &model,
            "", // session-id
            system,
            &messages,
            &[],
        )
        .await?;

    while let Some((Some(msg), _)) = stream.next().await.transpose()? {
        print!("{}", msg.as_concat_text());
    }
    println!();

    Ok(())
}
