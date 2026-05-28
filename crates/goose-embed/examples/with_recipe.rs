//! Load a goose recipe from a YAML file and run it inside an embedded agent.
//!
//! ```bash
//! GOOSE_EMBED_PROVIDER=anthropic \
//! GOOSE_EMBED_MODEL=claude-sonnet-4 \
//! cargo run -p goose-embed --example with_recipe -- path/to/recipe.yaml
//! ```
//!
//! Provider and model come from `GOOSE_EMBED_PROVIDER` / `GOOSE_EMBED_MODEL`
//! if set, otherwise from the recipe's `settings.goose_provider` /
//! `settings.goose_model`. If neither is available the example prints a hint
//! and exits 0.

use std::path::PathBuf;

use anyhow::Context;
use futures::StreamExt;

use goose_embed::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Some(recipe_path) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("Usage: cargo run -p goose-embed --example with_recipe -- <path/to/recipe.yaml>");
        return Ok(());
    };

    let recipe_text = std::fs::read_to_string(&recipe_path)
        .with_context(|| format!("reading recipe at {}", recipe_path.display()))?;
    let recipe: Recipe = serde_yaml::from_str(&recipe_text)
        .with_context(|| format!("parsing recipe at {}", recipe_path.display()))?;

    let mut builder = Goose::builder()
        .working_dir(std::env::current_dir()?)
        .recipe(recipe.clone());

    if let (Ok(provider), Ok(model)) = (
        std::env::var("GOOSE_EMBED_PROVIDER"),
        std::env::var("GOOSE_EMBED_MODEL"),
    ) {
        builder = builder.provider(provider, model);
    } else if recipe
        .settings
        .as_ref()
        .and_then(|s| s.goose_provider.as_ref())
        .is_none()
    {
        eprintln!(
            "Set GOOSE_EMBED_PROVIDER and GOOSE_EMBED_MODEL or provide settings.goose_provider / settings.goose_model in the recipe."
        );
        return Ok(());
    }

    let goose = builder.build().await?;

    let prompt = recipe
        .prompt
        .clone()
        .or_else(|| recipe.instructions.clone())
        .unwrap_or_else(|| "Run this recipe.".to_string());

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
