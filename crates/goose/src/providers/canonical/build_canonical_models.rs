/// Build canonical models from OpenRouter API
///
/// This script fetches models from OpenRouter and converts them to canonical format.
/// By default, it also checks which models from top providers are properly mapped.
///
/// Usage:
///   cargo run --bin build_canonical_models              # Build and check (default)
///   cargo run --bin build_canonical_models --no-check   # Build only, skip checker
///
use anyhow::{Context, Result};
use clap::Parser;
use goose::providers::canonical::{
    canonical_name, CanonicalModel, CanonicalModelRegistry, Pricing,
};
use goose::providers::{canonical::ModelMapping, create_with_named_model};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/models";
const ALLOWED_PROVIDERS: &[&str] = &[
    "anthropic",
    "google",
    "openai",
    "meta-llama",
    "mistralai",
    "x-ai",
    "deepseek",
    "cohere",
    "ai21",
    "qwen",
];

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
    ) {
        let mapping_map: HashMap<String, String> = mappings
            .iter()
            .map(|m| (m.provider_model.clone(), m.canonical_model.clone()))
            .collect();

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

#[allow(clippy::too_many_lines)]
async fn build_canonical_models() -> Result<()> {
    println!("Fetching models from OpenRouter API...");

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
        let name = model["name"].as_str().context("Model missing id field")?;

        // Skip OpenRouter-specific pricing variants (:free, :nitro)
        // Keep :extended since it has different context length
        if id.contains(":free") || id.contains(":nitro") {
            continue;
        }

        let canonical_id = canonical_name("openrouter", id);

        let provider = canonical_id.split('/').next().unwrap_or("");
        if !ALLOWED_PROVIDERS.contains(&provider) {
            continue;
        }

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

        if let Some(existing_model) = canonical_groups.get(&canonical_id) {
            let existing_name = shortest_names.get(&canonical_id).unwrap();

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

            let should_replace = if has_paid_pricing != existing_has_paid {
                has_paid_pricing // Prefer the one with paid pricing
            } else {
                name.len() < existing_name.len() // Both same pricing tier, prefer shorter name
            };

            if should_replace {
                println!(
                    "  Updating {} from '{}' (paid: {}) to '{}' (paid: {})",
                    canonical_id,
                    existing_model["id"].as_str().unwrap(),
                    existing_has_paid,
                    id,
                    has_paid_pricing
                );
                shortest_names.insert(canonical_id.clone(), name.to_string());
                canonical_groups.insert(canonical_id, model);
            }
        } else {
            println!(
                "  Adding: {} (from {}, paid: {})",
                canonical_id, id, has_paid_pricing
            );
            shortest_names.insert(canonical_id.clone(), name.to_string());
            canonical_groups.insert(canonical_id, model);
        }
    }

    // Filter out beta/preview variants if non-beta version exists
    let beta_suffixes = ["-beta", "-preview", "-alpha"];
    let mut to_remove = Vec::new();

    for canonical_id in canonical_groups.keys() {
        for suffix in &beta_suffixes {
            if canonical_id.ends_with(suffix) {
                // Check if non-beta version exists
                let base_id = canonical_id.strip_suffix(suffix).unwrap();
                if canonical_groups.contains_key(base_id) {
                    println!(
                        "  Filtering out {} (non-beta version {} exists)",
                        canonical_id, base_id
                    );
                    to_remove.push(canonical_id.clone());
                    break;
                }
            }
        }
    }

    for id in to_remove {
        canonical_groups.remove(&id);
        shortest_names.remove(&id);
    }

    // Second pass: Build the registry with the selected models
    let mut registry = CanonicalModelRegistry::new();

    for (canonical_id, model) in canonical_groups.iter() {
        let name = shortest_names.get(canonical_id).unwrap();

        let context_length = model["context_length"].as_u64().unwrap_or(128_000) as usize;

        let max_completion_tokens = model
            .get("top_provider")
            .and_then(|tp| tp.get("max_completion_tokens"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let mut input_modalities: Vec<String> = model
            .get("architecture")
            .and_then(|arch| arch.get("input_modalities"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| vec!["text".to_string()]);
        input_modalities.sort();

        let mut output_modalities: Vec<String> = model
            .get("architecture")
            .and_then(|arch| arch.get("output_modalities"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_else(|| vec!["text".to_string()]);
        output_modalities.sort();

        let supports_tools = model
            .get("supported_parameters")
            .and_then(|v| v.as_array())
            .map(|params| params.iter().any(|param| param.as_str() == Some("tools")))
            .unwrap_or(false);

        let pricing_obj = model
            .get("pricing")
            .context("Model missing pricing field")?;
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
            supports_tools,
            pricing,
        };

        registry.register(canonical_model);
    }

    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/providers/canonical/data/canonical_models.json");
    registry.to_file(&output_path)?;
    println!(
        "\n✓ Wrote {} models to {}",
        registry.count(),
        output_path.display()
    );

    Ok(())
}

async fn check_provider(
    provider_name: &str,
    model_for_init: &str,
) -> Result<(Vec<String>, Vec<ModelMapping>)> {
    println!("Checking provider: {}", provider_name);

    let provider = match create_with_named_model(provider_name, model_for_init).await {
        Ok(p) => p,
        Err(e) => {
            println!("  ⚠ Failed to create provider: {}", e);
            println!("  This is expected if credentials are not configured.");
            return Ok((Vec::new(), Vec::new()));
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

    Ok((fetched_models, mappings))
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
        ("tetrate", "claude-3-5-sonnet-computer-use"),
        ("xai", "grok-code-fast-1"),
    ];

    let mut report = MappingReport::new();

    for (provider_name, default_model) in providers {
        let (fetched, mappings) = check_provider(provider_name, default_model).await?;
        report.add_provider_results(provider_name, fetched, mappings);
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
