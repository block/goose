/// Build canonical models from models.dev API
///
/// This script fetches models from models.dev and converts them to canonical format.
///
/// Usage:
///   cargo run --bin build_canonical_models
///
use anyhow::{Context, Result};
use goose::providers::canonical::{
    CanonicalModel, CanonicalModelRegistry, Limit, Modalities, Modality, Pricing, canonical_name,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderMetadata {
    pub id: String,
    pub display_name: String,
    pub npm: Option<String>,
    pub api: Option<String>,
    pub doc: Option<String>,
    pub env: Vec<String>,
    pub model_count: usize,
}

const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";
const DEFAULT_CONTEXT_LIMIT: usize = 128_000;

fn is_compatible_provider(npm: &str) -> bool {
    npm.contains("openai") || npm.contains("anthropic") || npm.contains("ollama")
}

fn normalize_provider_name(provider: &str) -> &str {
    match provider {
        "llama" => "meta-llama",
        "xai" => "x-ai",
        "mistral" => "mistralai",
        _ => provider,
    }
}

fn data_file_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/providers/canonical/data")
        .join(filename)
}

async fn fetch_models_dev() -> Result<Value> {
    println!("Fetching models from models.dev API...");

    let client = reqwest::Client::new();
    let response = client
        .get(MODELS_DEV_API_URL)
        .header("User-Agent", "goose/canonical-builder")
        .send()
        .await
        .context("Failed to fetch from models.dev API")?;

    response
        .json()
        .await
        .context("Failed to parse models.dev response")
}

fn get_string(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(|v| v.as_str()).map(String::from)
}

fn parse_modalities(model_data: &Value, field: &str) -> Vec<Modality> {
    model_data
        .get("modalities")
        .and_then(|m| m.get(field))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| {
                    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
                })
                .collect()
        })
        .unwrap_or_else(|| vec![Modality::Text])
}

fn process_model(
    model_id: &str,
    model_data: &Value,
    normalized_provider: &str,
) -> Result<(String, CanonicalModel)> {
    let name = model_data["name"]
        .as_str()
        .with_context(|| format!("Model {} missing name", model_id))?;

    let canonical_id = canonical_name(normalized_provider, model_id);

    let modalities = Modalities {
        input: parse_modalities(model_data, "input"),
        output: parse_modalities(model_data, "output"),
    };

    let cost = match model_data.get("cost") {
        Some(c) if !c.is_null() => Pricing {
            input: c.get("input").and_then(|v| v.as_f64()),
            output: c.get("output").and_then(|v| v.as_f64()),
            cache_read: c.get("cache_read").and_then(|v| v.as_f64()),
            cache_write: c.get("cache_write").and_then(|v| v.as_f64()),
        },
        _ => Pricing {
            input: None,
            output: None,
            cache_read: None,
            cache_write: None,
        },
    };

    let limit = Limit {
        context: model_data
            .get("limit")
            .and_then(|l| l.get("context"))
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_CONTEXT_LIMIT as u64) as usize,
        output: model_data
            .get("limit")
            .and_then(|l| l.get("output"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
    };

    let canonical_model = CanonicalModel {
        id: canonical_id.clone(),
        name: name.to_string(),
        family: get_string(model_data, "family"),
        attachment: model_data.get("attachment").and_then(|v| v.as_bool()),
        reasoning: model_data.get("reasoning").and_then(|v| v.as_bool()),
        tool_call: model_data
            .get("tool_call")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        temperature: model_data.get("temperature").and_then(|v| v.as_bool()),
        knowledge: get_string(model_data, "knowledge"),
        release_date: get_string(model_data, "release_date"),
        last_updated: get_string(model_data, "last_updated"),
        modalities,
        open_weights: model_data.get("open_weights").and_then(|v| v.as_bool()),
        cost,
        limit,
    };

    let model_name = canonical_id
        .strip_prefix(&format!("{}/", normalized_provider))
        .unwrap_or(model_id)
        .to_string();

    Ok((model_name, canonical_model))
}

fn collect_provider_metadata(
    providers_obj: &serde_json::Map<String, Value>,
) -> Vec<ProviderMetadata> {
    let mut metadata_list = Vec::new();

    for (provider_id, provider_data) in providers_obj {
        let npm = match provider_data.get("npm").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        if !is_compatible_provider(npm) {
            continue;
        }

        let api = provider_data
            .get("api")
            .and_then(|v| v.as_str())
            .map(String::from);

        if api.is_none() {
            continue;
        }

        let normalized_provider = normalize_provider_name(provider_id).to_string();
        let doc = provider_data
            .get("doc")
            .and_then(|v| v.as_str())
            .map(String::from);
        let env = provider_data
            .get("env")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        let display_name = provider_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(provider_id)
            .to_string();
        let model_count = provider_data
            .get("models")
            .and_then(|v| v.as_object())
            .map(|models| models.len())
            .unwrap_or(0);

        metadata_list.push(ProviderMetadata {
            id: normalized_provider,
            display_name,
            npm: Some(npm.to_string()),
            api,
            doc,
            env,
            model_count,
        });

        println!("  Added {} ({}) - {} models", provider_id, npm, model_count);
    }

    metadata_list
}

async fn build_canonical_models() -> Result<()> {
    let json = fetch_models_dev().await?;

    let providers_obj = json
        .as_object()
        .context("Expected object in models.dev response")?;

    let mut registry = CanonicalModelRegistry::new();
    let mut total_models = 0;

    for (provider_key, provider_data) in providers_obj {
        let models = match provider_data.get("models").and_then(|v| v.as_object()) {
            Some(m) => m,
            None => continue,
        };

        let normalized_provider = normalize_provider_name(provider_key);

        println!(
            "\nProcessing {} ({} models)...",
            normalized_provider,
            models.len()
        );

        for (model_id, model_data) in models {
            let (model_name, canonical_model) =
                process_model(model_id, model_data, normalized_provider)?;
            registry.register(normalized_provider, &model_name, canonical_model);
            total_models += 1;
        }
    }

    let output_path = data_file_path("canonical_models.json");
    registry.to_file(&output_path)?;
    println!(
        "\n✓ Wrote {} models to {}",
        total_models,
        output_path.display()
    );

    println!("\n\nCollecting provider metadata from models.dev...");
    let provider_metadata_list = collect_provider_metadata(providers_obj);

    let provider_metadata_path = data_file_path("provider_metadata.json");
    let provider_metadata_json = serde_json::to_string_pretty(&provider_metadata_list)?;
    std::fs::write(&provider_metadata_path, provider_metadata_json)?;
    println!(
        "✓ Wrote {} providers metadata to {}",
        provider_metadata_list.len(),
        provider_metadata_path.display()
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    build_canonical_models().await
}
