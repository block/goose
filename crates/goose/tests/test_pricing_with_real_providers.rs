/// Test pricing and canonical model coverage against REAL provider model lists
/// This connects to actual providers using your configured API keys
/// Run with: cargo test --test test_pricing_with_real_providers -- --nocapture --ignored
///
/// Mark as #[ignore] so it only runs when explicitly requested (requires API keys)

use goose::providers::canonical::{fuzzy_canonical_name, CanonicalModelRegistry};
use goose::providers::pricing::{get_model_pricing, initialize_pricing_cache};
use goose::providers::create_with_named_model;

#[tokio::test]
#[ignore] // Only run when explicitly requested with --ignored flag
async fn test_real_provider_pricing_coverage() {
    println!("\n🔌 Testing Pricing Coverage with Real Provider APIs");
    println!("   (This test requires configured API keys)\n");

    // Initialize pricing cache
    if let Err(e) = initialize_pricing_cache().await {
        println!("⚠️  Failed to initialize pricing cache: {}", e);
        return;
    }

    // Load canonical models
    let registry = match CanonicalModelRegistry::bundled() {
        Ok(reg) => reg,
        Err(e) => {
            println!("❌ Failed to load canonical registry: {}", e);
            return;
        }
    };

    // Providers to test with their default models for initialization
    let providers_to_test = vec![
        ("anthropic", "claude-3-5-sonnet-20241022"),
        ("openai", "gpt-4"),
        ("google", "gemini-1.5-pro-002"),
        ("openrouter", "anthropic/claude-3.5-sonnet"),
        ("databricks", "databricks-meta-llama-3-1-70b-instruct"),
    ];

    let mut total_models = 0;
    let mut canonical_hits = 0;
    let mut pricing_hits = 0;
    let mut both_hits = 0;
    let mut neither_hits = 0;

    for (provider_name, default_model) in providers_to_test {
        println!("\n{}", "=".repeat(80));
        println!("📡 Provider: {}", provider_name);
        println!("{}", "=".repeat(80));

        // Try to create provider with user's credentials
        let provider = match create_with_named_model(provider_name, default_model).await {
            Ok(p) => p,
            Err(e) => {
                println!("  ⚠️  Skipping {} - not configured: {}", provider_name, e);
                println!("  Configure your API key to test this provider");
                continue;
            }
        };

        // Fetch real model list from provider API
        let models = match provider.fetch_supported_models().await {
            Ok(Some(models)) => {
                println!("  ✅ Fetched {} models from provider API", models.len());
                models
            }
            Ok(None) => {
                println!("  ⚠️  Provider does not support model listing");
                continue;
            }
            Err(e) => {
                println!("  ❌ Failed to fetch models: {}", e);
                continue;
            }
        };

        // Sample first 10 models for detailed output
        let sample_size = 10.min(models.len());
        if sample_size > 0 {
            println!("\n  📋 Testing coverage (showing first {} models):\n", sample_size);
            println!("  {:<50} {:<12} {:<12}", "Model", "Canonical", "Pricing");
            println!("  {}", "-".repeat(78));
        }

        let mut provider_canonical = 0;
        let mut provider_pricing = 0;
        let mut provider_both = 0;
        let mut provider_neither = 0;

        for (idx, model) in models.iter().enumerate() {
            // Check canonical model mapping with fuzzy matching
            let candidates = fuzzy_canonical_name(provider_name, model);
            let in_canonical = candidates.iter().any(|canonical_id| {
                registry.all_models()
                    .into_iter()
                    .any(|m| m.id == *canonical_id)
            });

            // Check pricing cache
            let in_pricing = get_model_pricing(provider_name, model).await.is_some();

            // Track stats
            match (in_canonical, in_pricing) {
                (true, true) => {
                    provider_both += 1;
                    both_hits += 1;
                }
                (true, false) => {
                    provider_canonical += 1;
                    canonical_hits += 1;
                }
                (false, true) => {
                    provider_pricing += 1;
                    pricing_hits += 1;
                }
                (false, false) => {
                    provider_neither += 1;
                    neither_hits += 1;
                }
            }
            total_models += 1;

            // Show sample
            if idx < sample_size {
                let canonical_mark = if in_canonical { "✅" } else { "❌" };
                let pricing_mark = if in_pricing { "✅" } else { "❌" };
                let model_display = if model.len() > 47 {
                    format!("{}...", &model[..44])
                } else {
                    model.clone()
                };
                println!("  {:<50} {:<12} {:<12}",
                    model_display, canonical_mark, pricing_mark);
            }
        }

        // Provider summary
        let provider_total = models.len();
        println!("\n  📊 Provider Summary:");
        println!("     Total models:      {}", provider_total);
        println!("     Both systems:      {} ({:.1}%)",
            provider_both, (provider_both as f64 / provider_total as f64) * 100.0);
        println!("     Canonical only:    {} ({:.1}%)",
            provider_canonical, (provider_canonical as f64 / provider_total as f64) * 100.0);
        println!("     Pricing only:      {} ({:.1}%)",
            provider_pricing, (provider_pricing as f64 / provider_total as f64) * 100.0);
        println!("     Neither:           {} ({:.1}%)",
            provider_neither, (provider_neither as f64 / provider_total as f64) * 100.0);
    }

    // Overall summary
    println!("\n{}", "=".repeat(80));
    println!("📈 OVERALL SUMMARY ACROSS ALL PROVIDERS");
    println!("{}", "=".repeat(80));
    println!("Total models tested:    {}", total_models);
    println!("Both systems:           {} ({:.1}%)",
        both_hits, (both_hits as f64 / total_models as f64) * 100.0);
    println!("Canonical only:         {} ({:.1}%)",
        canonical_hits, (canonical_hits as f64 / total_models as f64) * 100.0);
    println!("Pricing only:           {} ({:.1}%)",
        pricing_hits, (pricing_hits as f64 / total_models as f64) * 100.0);
    println!("Neither:                {} ({:.1}%)",
        neither_hits, (neither_hits as f64 / total_models as f64) * 100.0);

    println!("\n💡 Interpretation:");
    println!("   • Both systems:     Model pricing and metadata available");
    println!("   • Canonical only:   Model metadata available, pricing may be missing");
    println!("   • Pricing only:     Pricing available, but not in canonical registry");
    println!("   • Neither:          Model not recognized by either system");
}
