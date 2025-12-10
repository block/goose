/// Debug script to see what OpenRouter models look like and why they don't match
/// Run with: cargo test --test debug_openrouter_models -- --nocapture --ignored

use goose::providers::canonical::{canonical_name, CanonicalModelRegistry};
use goose::providers::create_with_named_model;

#[tokio::test]
#[ignore]
async fn debug_openrouter_models() {
    println!("\n🔍 Debugging OpenRouter Model Matching\n");

    // Load canonical registry
    let registry = match CanonicalModelRegistry::bundled() {
        Ok(reg) => reg,
        Err(e) => {
            println!("❌ Failed to load canonical registry: {}", e);
            return;
        }
    };

    // Get all canonical model IDs for quick lookup
    let canonical_ids: std::collections::HashSet<String> = registry
        .all_models()
        .into_iter()
        .map(|m| m.id.clone())
        .collect();

    println!("📦 Canonical registry has {} models\n", canonical_ids.len());

    // Create OpenRouter provider
    let provider = match create_with_named_model("openrouter", "anthropic/claude-3.5-sonnet").await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️  Skipping - OpenRouter not configured: {}", e);
            return;
        }
    };

    // Fetch models from OpenRouter
    let models = match provider.fetch_supported_models().await {
        Ok(Some(models)) => {
            println!("✅ OpenRouter API returned {} models\n", models.len());
            models
        }
        Ok(None) => {
            println!("⚠️  OpenRouter does not support model listing");
            return;
        }
        Err(e) => {
            println!("❌ Failed to fetch models: {}", e);
            return;
        }
    };

    println!("🔍 Analyzing first 20 models:\n");
    println!("{:<50} {:<50} {:<10}", "OpenRouter Model", "Canonical Name", "Match?");
    println!("{}", "=".repeat(115));

    let mut matched = 0;
    let mut unmatched = 0;
    let mut unmatched_examples = Vec::new();

    for (idx, model) in models.iter().enumerate() {
        let canonical_id = canonical_name("openrouter", model);
        let is_match = canonical_ids.contains(&canonical_id);

        if idx < 20 {
            let model_display = if model.len() > 47 {
                format!("{}...", &model[..44])
            } else {
                model.clone()
            };
            let canonical_display = if canonical_id.len() > 47 {
                format!("{}...", &canonical_id[..44])
            } else {
                canonical_id.clone()
            };
            let match_display = if is_match { "✅" } else { "❌" };

            println!("{:<50} {:<50} {:<10}", model_display, canonical_display, match_display);
        }

        if is_match {
            matched += 1;
        } else {
            unmatched += 1;
            if unmatched_examples.len() < 10 {
                unmatched_examples.push((model.clone(), canonical_id));
            }
        }
    }

    println!("\n{}", "=".repeat(115));
    println!("\n📊 Summary:");
    println!("   Matched:     {} ({:.1}%)", matched, (matched as f64 / models.len() as f64) * 100.0);
    println!("   Unmatched:   {} ({:.1}%)", unmatched, (unmatched as f64 / models.len() as f64) * 100.0);

    println!("\n❌ Sample Unmatched Models:");
    for (or_model, canonical_id) in unmatched_examples {
        println!("   {} → {}", or_model, canonical_id);
    }

    println!("\n💡 Analysis:");
    println!("   OpenRouter returns models in format: provider/model-name");
    println!("   Canonical registry includes: anthropic, google, openai models");
    println!("   Likely issue: OpenRouter has many providers not in canonical registry");
    println!("                 (ai21, alibaba, allenai, amazon, etc.)");
}
