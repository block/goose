//! Minimal goose-embed example: spin up an agent, send one prompt, stream
//! the response to stdout.
//!
//! ```bash
//! GOOSE_EMBED_PROVIDER=anthropic \
//! GOOSE_EMBED_MODEL=claude-sonnet-4 \
//! cargo run -p goose-embed --example hello_agent
//! ```
//!
//! If `GOOSE_EMBED_PROVIDER` or `GOOSE_EMBED_MODEL` are unset, the example
//! prints a friendly hint and exits cleanly so it remains CI-friendly.

use futures::StreamExt;

use goose_embed::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some((provider, model)) = provider_from_env() else {
        eprintln!(
            "Set GOOSE_EMBED_PROVIDER and GOOSE_EMBED_MODEL to run this example.\n\
             Example: GOOSE_EMBED_PROVIDER=anthropic GOOSE_EMBED_MODEL=claude-sonnet-4 cargo run -p goose-embed --example hello_agent"
        );
        return Ok(());
    };

    let goose = Goose::builder()
        .provider(provider, model)
        .working_dir(std::env::current_dir()?)
        .build()
        .await?;

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "What is 2 + 2? Answer in a single sentence.".to_string());

    let mut stream = goose.reply(prompt).await?;
    while let Some(event) = stream.next().await {
        if let Ok(AgentEvent::Message(message)) = event {
            let text = message.as_concat_text();
            if !text.is_empty() {
                println!("{text}");
            }
        }
    }

    Ok(())
}

fn provider_from_env() -> Option<(String, String)> {
    let provider = std::env::var("GOOSE_EMBED_PROVIDER").ok()?;
    let model = std::env::var("GOOSE_EMBED_MODEL").ok()?;
    Some((provider, model))
}
