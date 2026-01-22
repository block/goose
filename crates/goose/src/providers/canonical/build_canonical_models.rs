/// Build canonical models from models.dev API
///
/// This script fetches models from models.dev and converts them to canonical format.
/// By default, it also checks which models from top providers are properly mapped.
///
/// Usage:
///   cargo run --bin build_canonical_models              # Build and check (default)
///   cargo run --bin build_canonical_models --no-check   # Build only, skip checker
///
use anyhow::{Context, Result};
use clap::Parser;
use goose::providers::canonical::{
    canonical_name, CanonicalModel, CanonicalModelRegistry, Limit, Modalities, Pricing,
};
use goose::providers::{canonical::ModelMapping, create_with_named_model};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";

// Providers to include in canonical models
const ALLOWED_PROVIDERS: &[&str] = &[
    "anthropic",
    "google",
    "openai",
    "openrouter",
    "llama",
    "mistral",
    "xai",
    "deepseek",
    "cohere",
    "azure",
    "amazon-bedrock",
    "venice",
    "google-vertex",
];

// Normalize provider names from models.dev to our canonical format
fn normalize_provider_name(provider: &str) -> &str {
    match provider {
        "llama" => "meta-llama",
        "xai" => "x-ai",
        "mistral" => "mistralai",
        _ => provider,
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Skip the canonical model checker (only build models)
    #[arg(long)]
    no_check: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct ProviderModelPair {
    provider: String,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MappingEntry {
    provider: String,
    model: String,
    canonical: String,
    recommended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MappingReport {
    /// Timestamp of this report
    timestamp: String,

    /// Models that are NOT mapped to canonical models
    unmapped_models: Vec<ProviderModelPair>,

    /// All mappings: (provider, model) -> canonical model
    /// Stored per provider for backward compatibility
    all_mappings: BTreeMap<String, Vec<ModelMapping>>,

    /// Flat list of all mappings for easier comparison (lock file format)
    mapped_models: Vec<MappingEntry>,

    /// Total models checked per provider
    model_counts: BTreeMap<String, usize>,

    /// Canonical models referenced
    canonical_models_used: BTreeSet<String>,
}

impl MappingReport {
    fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            unmapped_models: Vec::new(),
            all_mappings: BTreeMap::new(),
            mapped_models: Vec::new(),
            model_counts: BTreeMap::new(),
            canonical_models_used: BTreeSet::new(),
        }
    }

    fn add_provider_results(
        &mut self,
        provider_name: &str,
        fetched_models: Vec<String>,
        mappings: Vec<ModelMapping>,
        recommended_models: Vec<String>,
    ) {
        let mapping_map: HashMap<String, String> = mappings
            .iter()
            .map(|m| (m.provider_model.clone(), m.canonical_model.clone()))
            .collect();

        // Create a set of recommended model names for quick lookup
        let recommended_set: std::collections::HashSet<String> =
            recommended_models.into_iter().collect();

        for model in &fetched_models {
            if !mapping_map.contains_key(model) {
                self.unmapped_models.push(ProviderModelPair {
                    provider: provider_name.to_string(),
                    model: model.clone(),
                });
            }
        }

        for (model, canonical) in &mapping_map {
            self.canonical_models_used.insert(canonical.clone());
            self.mapped_models.push(MappingEntry {
                provider: provider_name.to_string(),
                model: model.clone(),
                canonical: canonical.clone(),
                recommended: recommended_set.contains(model),
            });
        }

        self.all_mappings
            .insert(provider_name.to_string(), mappings);
        self.model_counts
            .insert(provider_name.to_string(), fetched_models.len());
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("CANONICAL MODEL MAPPING REPORT");
        println!("{}", "=".repeat(80));
        println!("\nGenerated: {}\n", self.timestamp);

        println!("Models Checked Per Provider:");
        println!("{}", "-".repeat(80));
        let mut providers: Vec<_> = self.model_counts.iter().collect();
        providers.sort_by_key(|(name, _)| *name);
        for (provider, count) in providers {
            let mapped = self
                .all_mappings
                .get(provider)
                .map(|m| m.len())
                .unwrap_or(0);
            let unmapped = count - mapped;
            println!(
                "  {:<20} Total: {:>3}  Mapped: {:>3}  Unmapped: {:>3}",
                provider, count, mapped, unmapped
            );
        }

        println!("\n{}", "=".repeat(80));
        println!("UNMAPPED MODELS ({})", self.unmapped_models.len());
        println!("{}", "=".repeat(80));

        if self.unmapped_models.is_empty() {
            println!("✓ All models are mapped to canonical models!");
        } else {
            let mut unmapped_by_provider: HashMap<&str, Vec<&str>> = HashMap::new();
            for pair in &self.unmapped_models {
                unmapped_by_provider
                    .entry(pair.provider.as_str())
                    .or_default()
                    .push(pair.model.as_str());
            }

            let mut providers: Vec<_> = unmapped_by_provider.keys().collect();
            providers.sort();

            for provider in providers {
                println!("\n{}:", provider);
                let mut models = unmapped_by_provider[provider].to_vec();
                models.sort();
                for model in models {
                    println!("  - {}", model);
                }
            }
        }

        println!("\n{}", "=".repeat(80));
        println!(
            "CANONICAL MODELS REFERENCED ({})",
            self.canonical_models_used.len()
        );
        println!("{}", "=".repeat(80));
        if self.canonical_models_used.is_empty() {
            println!("  (none yet)");
        } else {
            let mut canonical: Vec<_> = self.canonical_models_used.iter().collect();
            canonical.sort();
            for model in canonical {
                println!("  - {}", model);
            }
        }

        println!("\n{}", "=".repeat(80));
    }

    fn compare_with_previous(&self, previous: &MappingReport) {
        println!("\n{}", "=".repeat(80));
        println!("CHANGES SINCE PREVIOUS RUN");
        println!("{}", "=".repeat(80));

        let mut prev_map: HashMap<(String, String), String> = HashMap::new();
        for entry in &previous.mapped_models {
            prev_map.insert(
                (entry.provider.clone(), entry.model.clone()),
                entry.canonical.clone(),
            );
        }

        let mut curr_map: HashMap<(String, String), String> = HashMap::new();
        for entry in &self.mapped_models {
            curr_map.insert(
                (entry.provider.clone(), entry.model.clone()),
                entry.canonical.clone(),
            );
        }

        let mut changed_mappings = Vec::new();
        let mut added_mappings = Vec::new();
        let mut removed_mappings = Vec::new();

        for (key @ (provider, model), canonical) in &curr_map {
            match prev_map.get(key) {
                Some(prev_canonical) if prev_canonical != canonical => {
                    changed_mappings.push((
                        provider.clone(),
                        model.clone(),
                        prev_canonical.clone(),
                        canonical.clone(),
                    ));
                }
                None => {
                    added_mappings.push((provider.clone(), model.clone(), canonical.clone()));
                }
                _ => {
                    // No change
                }
            }
        }

        for (key @ (provider, model), canonical) in &prev_map {
            if !curr_map.contains_key(key) {
                removed_mappings.push((provider.clone(), model.clone(), canonical.clone()));
            }
        }

        if changed_mappings.is_empty() && added_mappings.is_empty() && removed_mappings.is_empty() {
            println!("\nNo changes in model mappings.");
        } else {
            if !changed_mappings.is_empty() {
                println!("\n⚠ Changed Mappings ({}):", changed_mappings.len());
                println!("  (Models that now map to a different canonical model)");
                for (provider, model, old_canonical, new_canonical) in changed_mappings {
                    println!("  {} / {}", provider, model);
                    println!("    WAS: {}", old_canonical);
                    println!("    NOW: {}", new_canonical);
                }
            }

            if !added_mappings.is_empty() {
                println!("\n✓ Added Mappings ({}):", added_mappings.len());
                println!("  (Models that gained a canonical mapping)");
                for (provider, model, canonical) in added_mappings {
                    println!("  {} / {} -> {}", provider, model, canonical);
                }
            }

            if !removed_mappings.is_empty() {
                println!("\n✗ Removed Mappings ({}):", removed_mappings.len());
                println!("  (Models that lost their canonical mapping)");
                for (provider, model, canonical) in removed_mappings {
                    println!("  {} / {} (was: {})", provider, model, canonical);
                }
            }
        }

        println!("\n{}", "=".repeat(80));
    }

    fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let mut report = self.clone();

        report.unmapped_models.sort_by(|a, b| {
            a.provider
                .cmp(&b.provider)
                .then_with(|| a.model.cmp(&b.model))
        });

        report.mapped_models.sort_by(|a, b| {
            a.provider
                .cmp(&b.provider)
                .then_with(|| a.model.cmp(&b.model))
        });

        for mappings in report.all_mappings.values_mut() {
            mappings.sort_by(|a, b| a.provider_model.cmp(&b.provider_model));
        }

        let json = serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
        std::fs::write(path, json).context("Failed to write report file")?;
        Ok(())
    }

    fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read report file")?;
        let report: MappingReport =
            serde_json::from_str(&content).context("Failed to parse report file")?;
        Ok(report)
    }
}

async fn build_canonical_models() -> Result<()> {
    println!("Fetching models from models.dev API...");

    let client = reqwest::Client::new();
    let response = client
        .get(MODELS_DEV_API_URL)
        .header("User-Agent", "goose/canonical-builder")
        .send()
        .await
        .context("Failed to fetch from models.dev API")?;

    let json: Value = response
        .json()
        .await
        .context("Failed to parse models.dev response")?;

    let providers_obj = json
        .as_object()
        .context("Expected object in models.dev response")?;

    let mut registry = CanonicalModelRegistry::new();
    let mut total_models = 0;

    for provider_key in ALLOWED_PROVIDERS {
        if let Some(provider_data) = providers_obj.get(*provider_key) {
            let models = provider_data["models"]
                .as_object()
                .context(format!("Provider {} missing models object", provider_key))?;

            let normalized_provider = normalize_provider_name(provider_key);

            println!(
                "\nProcessing {} ({} models)...",
                normalized_provider,
                models.len()
            );

            for (model_id, model_data) in models {
                // Skip models without pricing information
                let cost_data = match model_data.get("cost") {
                    Some(c) if !c.is_null() => c,
                    _ => continue,
                };

                let name = model_data["name"]
                    .as_str()
                    .context(format!("Model {} missing name", model_id))?;

                // Use canonical_name to normalize the model ID (strips date stamps, etc.)
                // This deduplicates different versions of the same model
                let canonical_id = canonical_name(normalized_provider, model_id);

                let family = model_data
                    .get("family")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let attachment = model_data.get("attachment").and_then(|v| v.as_bool());

                let reasoning = model_data.get("reasoning").and_then(|v| v.as_bool());

                let tool_call = model_data
                    .get("tool_call")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let temperature = model_data.get("temperature").and_then(|v| v.as_bool());

                let knowledge = model_data
                    .get("knowledge")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let release_date = model_data
                    .get("release_date")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let last_updated = model_data
                    .get("last_updated")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let modalities = Modalities {
                    input: model_data
                        .get("modalities")
                        .and_then(|m| m.get("input"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_else(|| vec!["text".to_string()]),
                    output: model_data
                        .get("modalities")
                        .and_then(|m| m.get("output"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_else(|| vec!["text".to_string()]),
                };

                let open_weights = model_data.get("open_weights").and_then(|v| v.as_bool());

                let cost = Pricing {
                    input: cost_data.get("input").and_then(|v| v.as_f64()),
                    output: cost_data.get("output").and_then(|v| v.as_f64()),
                    cache_read: cost_data.get("cache_read").and_then(|v| v.as_f64()),
                    cache_write: cost_data.get("cache_write").and_then(|v| v.as_f64()),
                };

                let limit = Limit {
                    context: model_data
                        .get("limit")
                        .and_then(|l| l.get("context"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(128_000) as usize,
                    output: model_data
                        .get("limit")
                        .and_then(|l| l.get("output"))
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                };

                let canonical_model = CanonicalModel {
                    id: canonical_id.clone(),
                    name: name.to_string(),
                    family,
                    attachment,
                    reasoning,
                    tool_call,
                    temperature,
                    knowledge,
                    release_date,
                    last_updated,
                    modalities,
                    open_weights,
                    cost,
                    limit,
                };

                // Extract the normalized model name (everything after "provider/")
                let model_name = canonical_id
                    .strip_prefix(&format!("{}/", normalized_provider))
                    .unwrap_or(model_id);
                registry.register(normalized_provider, model_name, canonical_model);
                total_models += 1;
            }
        }
    }

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/providers/canonical/data/canonical_models.json");
    registry.to_file(&output_path)?;
    println!(
        "\n✓ Wrote {} models to {}",
        total_models,
        output_path.display()
    );

    Ok(())
}

async fn check_provider(
    provider_name: &str,
    model_for_init: &str,
) -> Result<(Vec<String>, Vec<ModelMapping>, Vec<String>)> {
    println!("Checking provider: {}", provider_name);

    let provider = match create_with_named_model(provider_name, model_for_init).await {
        Ok(p) => p,
        Err(e) => {
            println!("  ⚠ Failed to create provider: {}", e);
            println!("  This is expected if credentials are not configured.");
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }
    };

    let fetched_models = match provider.fetch_supported_models().await {
        Ok(Some(models)) => {
            println!("  ✓ Fetched {} models", models.len());
            models
        }
        Ok(None) => {
            println!("  ⚠ Provider does not support model listing");
            Vec::new()
        }
        Err(e) => {
            println!("  ⚠ Failed to fetch models: {}", e);
            println!("  This is expected if credentials are not configured.");
            Vec::new()
        }
    };

    // Also fetch recommended models (after deduplication)
    let recommended_models = match provider.fetch_recommended_models().await {
        Ok(Some(models)) => {
            println!("  ✓ Found {} recommended models", models.len());
            models
        }
        Ok(None) => Vec::new(),
        Err(e) => {
            println!("  ⚠ Failed to fetch recommended models: {}", e);
            Vec::new()
        }
    };

    let mut mappings = Vec::new();
    for model in &fetched_models {
        match provider.map_to_canonical_model(model).await {
            Ok(Some(canonical)) => {
                mappings.push(ModelMapping::new(model.clone(), canonical));
            }
            Ok(None) => {
                // No mapping found for this model
            }
            Err(e) => {
                println!("  ⚠ Failed to map model '{}': {}", model, e);
            }
        }
    }
    println!("  ✓ Found {} mappings", mappings.len());

    Ok((fetched_models, mappings, recommended_models))
}

async fn check_canonical_mappings() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Canonical Model Checker");
    println!("Checking model mappings for top providers...\n");

    // Define providers to check with their default models
    let providers = vec![
        ("anthropic", "claude-3-5-sonnet-20241022"),
        ("openai", "gpt-4"),
        ("openrouter", "anthropic/claude-3.5-sonnet"),
        ("google", "gemini-1.5-pro-002"),
        ("databricks", "claude-3-5-sonnet-20241022"),
        ("tetrate", "claude-3-5-sonnet-computer-use"),
        ("xai", "grok-code-fast-1"),
        ("azure_openai", "gpt-4o"),
        ("aws_bedrock", "anthropic.claude-3-5-sonnet-20241022-v2:0"),
        ("venice", "llama-3.3-70b"),
        ("gcp_vertex_ai", "gemini-1.5-pro-002"),
    ];

    let mut report = MappingReport::new();

    for (provider_name, default_model) in providers {
        let (fetched, mappings, recommended) = check_provider(provider_name, default_model).await?;
        report.add_provider_results(provider_name, fetched, mappings, recommended);
        println!();
    }

    report.print_summary();

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/providers/canonical/data/canonical_mapping_report.json");

    if output_path.exists() {
        if let Ok(previous) = MappingReport::load_from_file(&output_path) {
            report.compare_with_previous(&previous);
        }
    }

    report.save_to_file(&output_path)?;
    println!("\n✓ Report saved to: {}", output_path.display());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Build canonical models
    build_canonical_models().await?;

    // Run the checker unless --no-check is passed
    if !args.no_check {
        check_canonical_mappings().await?;
    }

    Ok(())
}
