use anyhow::Result;
use futures::StreamExt;
use gosling_providers::{
    base::Provider, conversation::message::Message, declarative::EnvKeyResolver, model::ModelConfig,
};

async fn complete(provider: &dyn Provider, model: ModelConfig) -> Result<()> {
    let system = "You are a knowledgable geography expert";
    let messages = [Message::user().with_text("what is the capital of France?")];
    let mut stream = provider.stream(&model, system, &messages, &[]).await?;

    while let Some((Some(msg), _)) = stream.next().await.transpose()? {
        print!("{}", msg.as_concat_text());
    }
    println!();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let deepseek = include_str!("deepseek.json");
    let deepseek_model = ModelConfig::new("deepseek-v4-flash");
    let zai = include_str!("zai.json");
    let zai_model = ModelConfig::new("glm-4.5-flash");

    for (json, model) in [(deepseek, deepseek_model), (zai, zai_model)] {
        let provider = gosling_providers::declarative::from_json(json, None, EnvKeyResolver {})?;
        println!("{}:", provider.get_name());
        complete(provider.as_ref(), model).await?;
    }
    Ok(())
}
