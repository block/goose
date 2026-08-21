use std::{collections::HashMap, env};

use anyhow::{Context, Result};
use futures::StreamExt;
use goose_providers::{
    api_client::{ApiClient, AuthMethod},
    base::Provider,
    conversation::message::Message,
    model::ModelConfig,
    openai::{parse_openai_base_url, OpenAiProviderBuilder},
};
use rmcp::{model::Tool, object};

fn tool(name: &str) -> Tool {
    Tool::new(
        name.to_string(),
        format!("Reproduction tool named {name}"),
        object!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        }),
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    let key = env::var("OPENAI_API_KEY").context("OPENAI_API_KEY is required")?;
    let base_url = env::var("OPENAI_BASE_URL").context("OPENAI_BASE_URL is required")?;
    let base_path = env::var("OPENAI_BASE_PATH").unwrap_or_else(|_| "v1/chat/completions".into());
    let model_name = env::var("GOOSE_MODEL").unwrap_or_else(|_| "gpt-5.6-luna".into());
    let thinking_effort = env::var("GOOSE_THINKING_EFFORT").unwrap_or_else(|_| "max".into());

    let (host, query, _) = parse_openai_base_url(&base_url)?;
    let api_client =
        ApiClient::new_with_tls(host, AuthMethod::BearerToken(key), Some(Default::default()))?
            .with_query(query);
    let provider = OpenAiProviderBuilder::new(api_client)
        .base_path(base_path)
        .build();
    let model = ModelConfig::new(model_name).with_merged_request_params(HashMap::from([(
        "thinking_effort".to_string(),
        serde_json::json!(thinking_effort),
    )]));
    let tools = ["edit", "read_image", "shell", "tree", "write"]
        .map(tool)
        .to_vec();
    let messages = [Message::user().with_text("Reply with exactly OK.")];

    eprintln!(
        "POST {} with model={} thinking_effort={} and {} function tools",
        provider.get_name(),
        model.model_name,
        model
            .request_param::<String>("thinking_effort")
            .unwrap_or_default(),
        tools.len()
    );

    let mut stream = provider
        .stream(&model, "Reply with exactly OK.", &messages, &tools)
        .await?;
    while let Some(event) = stream.next().await {
        let (message, _) = event?;
        if let Some(message) = message {
            print!("{}", message.as_concat_text());
        }
    }
    println!();

    Ok(())
}
