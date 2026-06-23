use std::env;

use anyhow::Result;
use futures::StreamExt;
use goose_providers::{
    api_client::{ApiClient, AuthMethod},
    base::Provider,
    conversation::message::Message,
    model::ModelConfig,
    openai::OpenAiProvider,
};

#[tokio::main]
async fn main() -> Result<()> {
    let model = ModelConfig::new("gpt-5.4-mini")?;
    let key = env::var("OPENAI_API_KEY").map_err(|_| anyhow::anyhow!("need an OpenAI key"))?;
    let api_client = ApiClient::new_with_tls(
        "https://api.openai.com".to_string(),
        AuthMethod::BearerToken(key),
        Some(Default::default()),
    )?;
    let provider = OpenAiProvider::new(api_client, model.clone());

    let system = "You are a knowledgable geography expert";
    let messages = [Message::user().with_text("what is the capital of France?")];

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
