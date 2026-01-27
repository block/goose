/// Check context limits: Current (hardcoded) vs Canonical Models
///
/// This script compares what we currently use for context limits (hardcoded in model.rs
/// and provider code) with what canonical_models provides. It generates a detailed report
/// showing input and output limits for all models from our core providers.
///
/// Usage:
///   cargo run --bin check_context_limits
///   cargo run --bin check_context_limits > before.txt  # Run before changes
///   cargo run --bin check_context_limits > after.txt   # Run after changes
///   diff before.txt after.txt                          # Compare

use anyhow::{Context, Result};
use goose::providers::{create_with_named_model, canonical::CanonicalModelRegistry};
use std::collections::HashMap;

const DEFAULT_INPUT_CONTEXT: usize = 128_000;
const DEFAULT_OUTPUT_TOKENS: i32 = 4_096;

#[derive(Debug)]
struct ModelLimits {
    model_name: String,
    provider: String,

    // What we currently would use (hardcoded)
    current_input: usize,
    current_output: i32,

    // What canonical provides
    canonical_input: Option<usize>,
    canonical_output: Option<i32>,

    // What the new system would use
    new_input: usize,
    new_output: i32,

    // Whether there's a difference
    input_differs: bool,
    output_differs: bool,
}

async fn get_provider_models_with_canonical(
    provider_name: &str,
    init_model: &str,
) -> Result<Vec<(String, Option<String>)>> {
    let provider = match create_with_named_model(provider_name, init_model).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠ Skipping {}: {}", provider_name, e);
            return Ok(Vec::new());
        }
    };

    let session_id = uuid::Uuid::new_v4().to_string();
    let models = match provider.fetch_supported_models(&session_id).await {
        Ok(Some(models)) => models,
        Ok(None) => Vec::new(),
        Err(e) => {
            println!("⚠ Failed to fetch models from {}: {}", provider_name, e);
            Vec::new()
        }
    };

    // Map each model to its canonical name
    let mut result = Vec::new();
    for model in models {
        let canonical = provider.map_to_canonical_model(&model).await.ok().flatten();
        result.push((model, canonical));
    }

    Ok(result)
}

fn get_current_hardcoded_input(model_name: &str) -> usize {
    // This mimics the logic in model.rs MODEL_SPECIFIC_LIMITS
    const LIMITS: &[(&str, usize)] = &[
        ("gpt-5.2-codex", 400_000),
        ("gpt-5.2", 400_000),
        ("gpt-5.1-codex-max", 256_000),
        ("gpt-5.1-codex-mini", 256_000),
        ("gpt-4-turbo", 128_000),
        ("gpt-4.1", 1_000_000),
        ("gpt-4-1", 1_000_000),
        ("gpt-4o", 128_000),
        ("o4-mini", 200_000),
        ("o3-mini", 200_000),
        ("o3", 200_000),
        ("claude", 200_000),
        ("gemini-1.5-flash", 1_000_000),
        ("gemini-1", 128_000),
        ("gemini-2", 1_000_000),
        ("gemma-3-27b", 128_000),
        ("gemma-3-12b", 128_000),
        ("gemma-3-4b", 128_000),
        ("gemma-3-1b", 32_000),
        ("gemma3-27b", 128_000),
        ("gemma3-12b", 128_000),
        ("gemma3-4b", 128_000),
        ("gemma3-1b", 32_000),
        ("gemma-2-27b", 8_192),
        ("gemma-2-9b", 8_192),
        ("gemma-2-2b", 8_192),
        ("gemma2-", 8_192),
        ("gemma-7b", 8_192),
        ("gemma-2b", 8_192),
        ("gemma1", 8_192),
        ("gemma", 8_192),
        ("llama-2-1b", 32_000),
        ("llama", 128_000),
        ("qwen3-coder", 262_144),
        ("qwen2-7b", 128_000),
        ("qwen2-14b", 128_000),
        ("qwen2-32b", 131_072),
        ("qwen2-70b", 262_144),
        ("qwen2", 128_000),
        ("qwen3-32b", 131_072),
        ("grok-4", 256_000),
        ("grok-code-fast-1", 256_000),
        ("grok", 131_072),
        ("kimi-k2", 131_072),
    ];

    LIMITS
        .iter()
        .find(|(pattern, _)| model_name.contains(pattern))
        .map(|(_, limit)| *limit)
        .unwrap_or(DEFAULT_INPUT_CONTEXT)
}

fn get_current_hardcoded_output(model_name: &str, provider: &str) -> i32 {
    // This mimics the logic scattered in provider code
    match provider {
        "anthropic" | "aws_bedrock" => {
            if model_name.contains("claude-3-haiku") {
                4096
            } else if model_name.contains("claude-opus-4-0") || model_name.contains("claude-opus-4-1") {
                32000
            } else if model_name.contains("claude") {
                64000
            } else {
                DEFAULT_OUTPUT_TOKENS
            }
        }
        _ => DEFAULT_OUTPUT_TOKENS
    }
}

fn get_canonical_limits_by_id(
    registry: &CanonicalModelRegistry,
    canonical_id: &str,
) -> (Option<usize>, Option<i32>) {
    // Look up by the full canonical ID (e.g., "anthropic/claude-3-5-sonnet")
    if let Some((provider, model)) = canonical_id.split_once('/') {
        if let Some(canonical) = registry.get(provider, model) {
            return (
                Some(canonical.limit.context),
                canonical.limit.output.map(|o| o as i32),
            );
        }
    }

    (None, None)
}

fn calculate_new_limits(
    _canonical_input: Option<usize>,
    _canonical_output: Option<i32>,
    provider: &str,
    model_name: &str,
) -> (usize, i32) {
    // Actually test what ModelConfig::new() returns!
    // This is the real code path users will hit
    match goose::model::ModelConfig::new(model_name, provider) {
        Ok(config) => (
            config.context_limit(),
            config.max_output_tokens(),
        ),
        Err(_) => {
            // If model config creation fails, use defaults
            (DEFAULT_INPUT_CONTEXT, DEFAULT_OUTPUT_TOKENS)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("================================================================================");
    println!("CONTEXT LIMITS COMPARISON: Current vs Canonical Models");
    println!("================================================================================\n");

    let registry = CanonicalModelRegistry::bundled()
        .context("Failed to load canonical models")?;

    // Core providers we have keys for (from build_canonical_models)
    // Note: aws_bedrock is slow to fetch, so skipping for now
    let providers = vec![
        ("anthropic", "claude-3-5-sonnet-20241022"),
        ("openai", "gpt-4"),
        ("google", "gemini-1.5-pro-002"),
        ("xai", "grok-2"),
    ];

    let mut all_limits: Vec<ModelLimits> = Vec::new();
    let mut provider_summaries: HashMap<String, (usize, usize, usize)> = HashMap::new();

    for (provider_name, default_model) in providers {
        println!("Fetching models from {}...", provider_name);
        let models_with_canonical = get_provider_models_with_canonical(provider_name, default_model).await?;

        if models_with_canonical.is_empty() {
            println!("  No models found\n");
            continue;
        }

        println!("  Found {} models\n", models_with_canonical.len());

        let mut same_count = 0;
        let mut different_count = 0;
        let mut missing_canonical = 0;

        for (model, canonical_id_opt) in models_with_canonical {
            let current_input = get_current_hardcoded_input(&model);
            let current_output = get_current_hardcoded_output(&model, provider_name);

            let (canonical_input, canonical_output) = if let Some(ref canonical_id) = canonical_id_opt {
                get_canonical_limits_by_id(&registry, canonical_id)
            } else {
                (None, None)
            };

            let (new_input, new_output) = calculate_new_limits(
                canonical_input,
                canonical_output,
                provider_name,
                &model,
            );

            let input_differs = current_input != new_input;
            let output_differs = current_output != new_output;

            if canonical_input.is_none() {
                missing_canonical += 1;
            } else if input_differs || output_differs {
                different_count += 1;
            } else {
                same_count += 1;
            }

            all_limits.push(ModelLimits {
                model_name: model.clone(),
                provider: provider_name.to_string(),
                current_input,
                current_output,
                canonical_input,
                canonical_output,
                new_input,
                new_output,
                input_differs,
                output_differs,
            });
        }

        provider_summaries.insert(
            provider_name.to_string(),
            (same_count, different_count, missing_canonical),
        );
    }

    // Print summary
    println!("\n{}", "=".repeat(80));
    println!("SUMMARY BY PROVIDER");
    println!("{}", "=".repeat(80));
    for (provider, (same, different, missing)) in &provider_summaries {
        let total = same + different + missing;
        println!("{}:", provider);
        println!("  Total models:          {}", total);
        println!("  Same as canonical:     {} ({:.1}%)", same, (*same as f64 / total as f64) * 100.0);
        println!("  Different:             {} ({:.1}%)", different, (*different as f64 / total as f64) * 100.0);
        println!("  Missing canonical:     {} ({:.1}%)", missing, (*missing as f64 / total as f64) * 100.0);
        println!();
    }

    // Print detailed diff for models that would change
    println!("{}", "=".repeat(80));
    println!("MODELS THAT WOULD CHANGE");
    println!("{}", "=".repeat(80));

    let changes: Vec<_> = all_limits
        .iter()
        .filter(|l| l.input_differs || l.output_differs)
        .collect();

    if changes.is_empty() {
        println!("No changes!\n");
    } else {
        println!("Found {} models with different limits\n", changes.len());

        for limit in changes {
            println!("{} / {}", limit.provider, limit.model_name);

            if limit.input_differs {
                println!("  Input context:");
                println!("    Current:   {:>10}", format_number(limit.current_input));
                println!("    Canonical: {:>10}",
                    limit.canonical_input.map(format_number).unwrap_or_else(|| "N/A".to_string()));
                println!("    New:       {:>10} {}",
                    format_number(limit.new_input),
                    if limit.new_input > limit.current_input { "↑" } else { "↓" });
            }

            if limit.output_differs {
                println!("  Output tokens:");
                println!("    Current:   {:>10}", format_number(limit.current_output as usize));
                println!("    Canonical: {:>10}",
                    limit.canonical_output.map(|o| format_number(o as usize)).unwrap_or_else(|| "N/A".to_string()));
                println!("    New:       {:>10} {}",
                    format_number(limit.new_output as usize),
                    if limit.new_output > limit.current_output { "↑" } else { "↓" });
            }

            println!();
        }
    }

    // Print all models in structured format for diffing
    println!("{}", "=".repeat(80));
    println!("COMPLETE MODEL LISTING");
    println!("{}", "=".repeat(80));
    println!("{:<20} {:<50} {:>12} {:>12}", "Provider", "Model", "Input", "Output");
    println!("{}", "-".repeat(96));

    for limit in &all_limits {
        println!("{:<20} {:<50} {:>12} {:>12}",
            limit.provider,
            limit.model_name,
            format_number(limit.new_input),
            format_number(limit.new_output as usize)
        );
    }

    println!("\n{}", "=".repeat(80));

    Ok(())
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
