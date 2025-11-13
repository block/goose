/// Build canonical models from OpenRouter API
///
/// This script fetches models from OpenRouter and converts them to canonical format.
/// Usage:
///   cargo run --example build_canonical_models
///
use anyhow::{Context, Result};
use goose::providers::canonical::{canonical_name, CanonicalModel, CanonicalModelRegistry, Pricing};
use serde_json::Value;
use std::collections::HashMap;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/models";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Fetching models from OpenRouter API...");

    // Fetch models from OpenRouter
    let client = reqwest::Client::new();
    let response = client
        .get(OPENROUTER_API_URL)
        .header("User-Agent", "goose/canonical-builder")
        .send()
        .await
        .context("Failed to fetch from OpenRouter API")?;

    let json: Value = response
        .json()
        .await
        .context("Failed to parse OpenRouter response")?;

    let models = json["data"]
        .as_array()
        .context("Expected 'data' array in OpenRouter response")?
        .clone();

    println!("Processing {} models from OpenRouter...", models.len());

    // First pass: Group models by canonical ID and track the one with shortest name
    let mut canonical_groups: HashMap<String, &Value> = HashMap::new();
    let mut shortest_names: HashMap<String, String> = HashMap::new();

    for model in &models {
        let id = model["id"].as_str().unwrap();
        let name = model["name"].as_str().unwrap_or(id);
        let canonical_id = canonical_name("openrouter", id);

        // Check if we've seen this canonical ID before
        if let Some(existing_name) = shortest_names.get(&canonical_id) {
            if name.len() < existing_name.len() {
                println!("  Updating {} to use shorter name: '{}' (len {}) -> '{}' (len {})",
                    canonical_id, existing_name, existing_name.len(), name, name.len());
                shortest_names.insert(canonical_id.clone(), name.to_string());
                canonical_groups.insert(canonical_id, model);
            } else {
                println!("  Skipping duplicate {}: keeping shorter name '{}'", id, existing_name);
            }
        } else {
            println!("  Adding: {} (from {})", canonical_id, id);
            shortest_names.insert(canonical_id.clone(), name.to_string());
            canonical_groups.insert(canonical_id, model);
        }
    }

    // Second pass: Build the registry with the selected models
    let mut registry = CanonicalModelRegistry::new();

    for (canonical_id, model) in canonical_groups.iter() {
        let name = shortest_names.get(canonical_id).unwrap();

        // Parse context length
        let context_length = model["context_length"].as_u64().unwrap_or(128_000) as usize;

        // Parse max completion tokens (if available)
        let max_completion_tokens = model
            .get("top_provider")
            .and_then(|tp| tp.get("max_completion_tokens"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        // Parse modalities
        let input_modalities: Vec<String> = model
            .get("supported_parameters")
            .and_then(|v| v.as_array())
            .map(|arr| {
                let mut mods = vec!["text".to_string()];
                for param in arr {
                    if let Some(s) = param.as_str() {
                        match s {
                            "image" | "image_url" => {
                                if !mods.contains(&"image".to_string()) {
                                    mods.push("image".to_string());
                                }
                            }
                            "audio" => {
                                if !mods.contains(&"audio".to_string()) {
                                    mods.push("audio".to_string());
                                }
                            }
                            "video" => {
                                if !mods.contains(&"video".to_string()) {
                                    mods.push("video".to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // Check if model has file support
                if model.get("architecture").and_then(|a| a.get("multimodality")).is_some() {
                    if !mods.contains(&"file".to_string()) {
                        mods.push("file".to_string());
                    }
                }
                mods
            })
            .unwrap_or_else(|| vec!["text".to_string()]);

        let output_modalities = vec!["text".to_string()];

        // Determine tokenizer based on provider
        let tokenizer = if canonical_id.starts_with("anthropic/") {
            "Claude"
        } else if canonical_id.starts_with("openai/") {
            "GPT"
        } else if canonical_id.starts_with("google/") {
            "Gemini"
        } else {
            "Unknown"
        }
        .to_string();

        // Check if model supports tool calling
        let supports_tools = model
            .get("supported_parameters")
            .and_then(|v| v.as_array())
            .map(|params| {
                params
                    .iter()
                    .any(|param| param.as_str() == Some("tools"))
            })
            .unwrap_or(false);

        // Parse pricing (convert strings to f64)
        let pricing_obj = model.get("pricing").unwrap();
        let pricing = Pricing {
            prompt: pricing_obj
                .get("prompt")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            completion: pricing_obj
                .get("completion")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            request: pricing_obj
                .get("request")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            image: pricing_obj
                .get("image")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
        };

        let canonical_model = CanonicalModel {
            id: canonical_id.clone(),
            name: name.to_string(),
            context_length,
            max_completion_tokens,
            input_modalities,
            output_modalities,
            tokenizer,
            supports_tools,
            pricing,
        };

        registry.register(canonical_model);
    }

    // Write to file
    let output_path = "src/providers/canonical/data/canonical_models.json";
    registry.to_file(output_path)?;
    println!("\n✓ Wrote {} models to {}", registry.count(), output_path);

    // Also write a timestamped report
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let report_path = format!("src/providers/canonical/data/report_{}.json", timestamp);
    registry.to_file(&report_path)?;
    println!("✓ Wrote report to {}", report_path);

    Ok(())
}
