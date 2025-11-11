/// Fetch Canonical Models from OpenRouter
///
/// This script fetches model metadata from OpenRouter's API and generates
/// a canonical_models.json file with standardized model information.
///
/// Usage:
///   cargo run --example fetch_canonical_models
///
/// The generated file will be written to:
///   crates/goose/src/providers/canonical/canonical_models.json
///

use anyhow::{Context, Result};
use goose::providers::canonical::{CanonicalModel, CanonicalModelRegistry};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    data: Vec<CanonicalModel>,
}

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/models";

// Providers we want to include
const ALLOWED_PROVIDERS: &[&str] = &["anthropic", "google", "openai"];

#[tokio::main]
async fn main() -> Result<()> {
    println!("Fetching models from OpenRouter API...\n");

    // Fetch models from OpenRouter
    let client = reqwest::Client::new();
    let response = client
        .get(OPENROUTER_API_URL)
        .header("User-Agent", "goose/canonical-fetcher")
        .send()
        .await
        .context("Failed to fetch from OpenRouter API")?;

    let openrouter_response: OpenRouterResponse = response
        .json()
        .await
        .context("Failed to parse OpenRouter response")?;

    println!("✓ Fetched {} models from OpenRouter\n", openrouter_response.data.len());

    // Filter to only allowed providers
    let filtered_models: Vec<CanonicalModel> = openrouter_response
        .data
        .into_iter()
        .filter(|model| {
            if let Some(provider) = model.provider() {
                ALLOWED_PROVIDERS.contains(&provider)
            } else {
                false
            }
        })
        .collect();

    println!("Filtered to {} models from allowed providers:", filtered_models.len());

    // Count models per provider
    let mut provider_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for model in &filtered_models {
        if let Some(provider) = model.provider() {
            *provider_counts.entry(provider.to_string()).or_insert(0) += 1;
        }
    }

    let mut providers: Vec<_> = provider_counts.iter().collect();
    providers.sort_by_key(|(name, _)| *name);
    for (provider, count) in providers {
        println!("  {}: {} models", provider, count);
    }

    // Create registry and add models
    let mut registry = CanonicalModelRegistry::new();
    for model in filtered_models {
        registry.register(model);
    }

    // Determine output path
    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/providers/canonical/canonical_models.json");

    // Save to file
    registry.to_file(&output_path)
        .context("Failed to save canonical models file")?;

    println!("\n✓ Saved {} models to: {}", registry.count(), output_path.display());

    Ok(())
}
