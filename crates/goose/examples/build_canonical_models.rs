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

// Providers we want to include
const ALLOWED_PROVIDERS: &[&str] = &["anthropic", "google", "openai"];

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

        // Skip OpenRouter-specific pricing variants (:free, :nitro)
        // Keep :extended since it has different context length
        // :exacto will be stripped to map to base model
        if id.contains(":free") || id.contains(":nitro") {
            continue;
        }

        let canonical_id = canonical_name("openrouter", id);

        // Filter to only allowed providers
        let provider = canonical_id.split('/').next().unwrap_or("");
        if !ALLOWED_PROVIDERS.contains(&provider) {
            continue;
        }

        // Get pricing info for this model
        let prompt_cost = model
            .get("pricing")
            .and_then(|p| p.get("prompt"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let completion_cost = model
            .get("pricing")
            .and_then(|p| p.get("completion"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let has_paid_pricing = prompt_cost > 0.0 || completion_cost > 0.0;

        // Check if we've seen this canonical ID before
        if let Some(existing_model) = canonical_groups.get(&canonical_id) {
            let existing_name = shortest_names.get(&canonical_id).unwrap();

            // Get existing pricing info
            let existing_prompt = existing_model
                .get("pricing")
                .and_then(|p| p.get("prompt"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            let existing_completion = existing_model
                .get("pricing")
                .and_then(|p| p.get("completion"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            let existing_has_paid = existing_prompt > 0.0 || existing_completion > 0.0;

            // Prefer paid pricing over free, otherwise prefer shorter name
            let should_replace = if has_paid_pricing != existing_has_paid {
                has_paid_pricing  // Prefer the one with paid pricing
            } else {
                name.len() < existing_name.len()  // Both same pricing tier, prefer shorter name
            };

            if should_replace {
                println!("  Updating {} from '{}' (paid: {}) to '{}' (paid: {})",
                    canonical_id, existing_model["id"].as_str().unwrap(), existing_has_paid, id, has_paid_pricing);
                // Keep the shorter name, but use the new model data (for pricing)
                if name.len() >= existing_name.len() {
                    // New model's name is longer or equal, keep the existing shorter name
                    // (Don't update shortest_names)
                } else {
                    // New model's name is shorter, update it
                    shortest_names.insert(canonical_id.clone(), name.to_string());
                }
                canonical_groups.insert(canonical_id, model);
            }
        } else {
            println!("  Adding: {} (from {}, paid: {})", canonical_id, id, has_paid_pricing);
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
    use std::path::PathBuf;

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/providers/canonical/data/canonical_models.json");
    registry.to_file(&output_path)?;
    println!("\n✓ Wrote {} models to {}", registry.count(), output_path.display());

    // Also write a timestamped report
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let report_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("src/providers/canonical/data/report_{}.json", timestamp));
    registry.to_file(&report_path)?;
    println!("✓ Wrote report to {}", report_path.display());

    Ok(())
}
