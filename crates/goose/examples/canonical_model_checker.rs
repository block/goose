/// Canonical Model Checker
///
/// This script checks which models from top providers are properly mapped to canonical models.
/// It maintains a lock file of mappings and detects changes between runs.
///
/// Outputs:
/// - Models that are NOT mapped to canonical models
/// - Full list of (provider, model) <-> canonical-model mappings
/// - Diff report showing mapping changes since last run:
///   * Changed mappings (model now maps to a different canonical model)
///   * Added mappings (model gained a canonical mapping)
///   * Removed mappings (model lost its canonical mapping)
///
/// Output File:
/// - src/providers/canonical/data/canonical_mapping_report.json
///   Contains full report with mapping data (acts as a lock file)
///
/// Usage:
///   cargo run --example canonical_model_checker -- [--output custom_path.json]
///

use anyhow::{Context, Result};
use goose::providers::{
    canonical::ModelMapping,
    create_with_named_model,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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
    all_mappings: HashMap<String, Vec<ModelMapping>>,

    /// Flat list of all mappings for easier comparison (lock file format)
    mapped_models: Vec<MappingEntry>,

    /// Total models checked per provider
    model_counts: HashMap<String, usize>,

    /// Canonical models referenced
    canonical_models_used: HashSet<String>,
}

impl MappingReport {
    fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            unmapped_models: Vec::new(),
            all_mappings: HashMap::new(),
            mapped_models: Vec::new(),
            model_counts: HashMap::new(),
            canonical_models_used: HashSet::new(),
        }
    }

    fn add_provider_results(
        &mut self,
        provider_name: &str,
        fetched_models: Vec<String>,
        mappings: Vec<ModelMapping>,
    ) {
        // Build a map of provider model -> canonical model
        let mapping_map: HashMap<String, String> = mappings
            .iter()
            .map(|m| (m.provider_model.clone(), m.canonical_model.clone()))
            .collect();

        // Find unmapped models
        for model in &fetched_models {
            if !mapping_map.contains_key(model) {
                self.unmapped_models.push(ProviderModelPair {
                    provider: provider_name.to_string(),
                    model: model.clone(),
                });
            }
        }

        // Track canonical models used and build flat mapping list
        for (model, canonical) in &mapping_map {
            self.canonical_models_used.insert(canonical.clone());
            self.mapped_models.push(MappingEntry {
                provider: provider_name.to_string(),
                model: model.clone(),
                canonical: canonical.clone(),
            });
        }

        // Store mappings and counts
        self.all_mappings.insert(provider_name.to_string(), mappings);
        self.model_counts.insert(provider_name.to_string(), fetched_models.len());
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("CANONICAL MODEL MAPPING REPORT");
        println!("{}", "=".repeat(80));
        println!("\nGenerated: {}\n", self.timestamp);

        // Print model counts per provider
        println!("Models Checked Per Provider:");
        println!("{}", "-".repeat(80));
        let mut providers: Vec<_> = self.model_counts.iter().collect();
        providers.sort_by_key(|(name, _)| *name);
        for (provider, count) in providers {
            let mapped = self.all_mappings
                .get(provider)
                .map(|m| m.len())
                .unwrap_or(0);
            let unmapped = count - mapped;
            println!("  {:<20} Total: {:>3}  Mapped: {:>3}  Unmapped: {:>3}",
                     provider, count, mapped, unmapped);
        }

        // Print unmapped models
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

        // Print canonical models used
        println!("\n{}", "=".repeat(80));
        println!("CANONICAL MODELS REFERENCED ({})", self.canonical_models_used.len());
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

        // Build lookup maps for current and previous mappings
        // Key: (provider, model), Value: canonical model
        let mut prev_map: HashMap<(String, String), String> = HashMap::new();
        for entry in &previous.mapped_models {
            prev_map.insert((entry.provider.clone(), entry.model.clone()), entry.canonical.clone());
        }

        let mut curr_map: HashMap<(String, String), String> = HashMap::new();
        for entry in &self.mapped_models {
            curr_map.insert((entry.provider.clone(), entry.model.clone()), entry.canonical.clone());
        }

        // Find changes
        let mut changed_mappings = Vec::new();
        let mut added_mappings = Vec::new();
        let mut removed_mappings = Vec::new();

        // Check current mappings against previous
        for (key @ (provider, model), canonical) in &curr_map {
            match prev_map.get(key) {
                Some(prev_canonical) if prev_canonical != canonical => {
                    // Mapping changed
                    changed_mappings.push((provider.clone(), model.clone(), prev_canonical.clone(), canonical.clone()));
                }
                None => {
                    // New mapping
                    added_mappings.push((provider.clone(), model.clone(), canonical.clone()));
                }
                _ => {
                    // No change
                }
            }
        }

        // Check for removed mappings
        for (key @ (provider, model), canonical) in &prev_map {
            if !curr_map.contains_key(key) {
                removed_mappings.push((provider.clone(), model.clone(), canonical.clone()));
            }
        }

        // Print changes
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
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize report")?;
        std::fs::write(path, json)
            .context("Failed to write report file")?;
        Ok(())
    }

    fn load_from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read report file")?;
        let report: MappingReport = serde_json::from_str(&content)
            .context("Failed to parse report file")?;
        Ok(report)
    }
}

async fn check_provider(
    provider_name: &str,
    model_for_init: &str,
) -> Result<(Vec<String>, Vec<ModelMapping>)> {
    println!("Checking provider: {}", provider_name);

    // Create provider instance (using a default model for initialization)
    let provider = match create_with_named_model(provider_name, model_for_init).await {
        Ok(p) => p,
        Err(e) => {
            println!("  ⚠ Failed to create provider: {}", e);
            println!("  This is expected if credentials are not configured.");
            return Ok((Vec::new(), Vec::new()));
        }
    };

    // Fetch supported models
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

    // Map each fetched model to canonical model
    let mut mappings = Vec::new();
    for model in &fetched_models {
        match provider.map_to_canonical_model(model).await {
            Ok(Some(canonical)) => {
                mappings.push(ModelMapping::new(model.clone(), canonical).verified());
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

#[tokio::main]
async fn main() -> Result<()> {
    println!("Canonical Model Checker");
    println!("Checking model mappings for top providers...\n");

    // Define providers to check with their default models
    let providers = vec![
        ("anthropic", "claude-3-5-sonnet-20241022"),
        ("openai", "gpt-4"),
        ("openrouter", "anthropic/claude-3.5-sonnet"),
        ("databricks", "databricks-meta-llama-3-1-70b-instruct"),
        ("google", "gemini-1.5-pro-002"),
        ("tetrate", "claude-3-5-sonnet-computer-use"),
    ];

    let mut report = MappingReport::new();

    // Check each provider
    for (provider_name, default_model) in providers {
        let (fetched, mappings) = check_provider(provider_name, default_model).await?;
        report.add_provider_results(provider_name, fetched, mappings);
        println!();
    }

    // Print summary
    report.print_summary();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let output_path = if args.len() > 2 && args[1] == "--output" {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/providers/canonical/data/canonical_mapping_report.json")
    };

    // Try to compare with previous run
    if output_path.exists() {
        if let Ok(previous) = MappingReport::load_from_file(&output_path) {
            report.compare_with_previous(&previous);
        }
    }

    // Save report
    report.save_to_file(&output_path)?;
    println!("\n✓ Report saved to: {}", output_path.display());

    Ok(())
}
