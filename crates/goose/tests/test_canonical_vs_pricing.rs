/// Test to compare canonical model lookups vs pricing.rs lookups
/// Run with: cargo test --test test_canonical_vs_pricing -- --nocapture

use goose::providers::canonical::{canonical_name, CanonicalModelRegistry};
use goose::providers::pricing::{get_model_pricing, initialize_pricing_cache};

#[tokio::test]
async fn test_canonical_vs_pricing_coverage() {
    // Initialize pricing cache
    if let Err(e) = initialize_pricing_cache().await {
        println!("⚠️  Failed to initialize pricing cache: {}", e);
        println!("    This test requires network access");
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

    // Test cases with various model name formats
    let test_cases = vec![
        // Anthropic - various formats
        ("anthropic", "claude-3.5-sonnet"),         // Dot format (canonical)
        ("anthropic", "claude-3-5-sonnet"),         // Dash format (provider API)
        ("anthropic", "claude-3.5-sonnet-20241022"), // With date
        ("anthropic", "claude-3-5-sonnet-latest"),  // With -latest
        ("anthropic", "claude-3.5-haiku"),
        ("anthropic", "claude-3.5-haiku-20241022"),
        ("anthropic", "claude-3-opus"),
        ("anthropic", "claude-3-opus-20240229"),
        ("anthropic", "claude-sonnet-4"),
        ("anthropic", "claude-sonnet-4.5"),
        ("anthropic", "claude-opus-4"),
        ("anthropic", "claude-haiku-4.5"),

        // Google - various formats
        ("google", "gemini-2.5-flash"),
        ("google", "gemini-2.5-flash-preview"),
        ("google", "gemini-2.5-flash-preview-09-2025"),
        ("google", "gemini-2.5-pro"),
        ("google", "gemini-2.5-pro-exp"),
        ("google", "gemini-2.0-flash"),
        ("google", "gemini-2.0-flash-001"),
        ("google", "gemini-2.0-flash-exp"),
        ("google", "gemini-2.0-flash-lite-001"),
        ("google", "gemma-2-27b-it"),
        ("google", "gemma-2-9b-it"),

        // OpenAI - various formats
        ("openai", "gpt-4o"),
        ("openai", "gpt-4o-2024-11-20"),
        ("openai", "gpt-4o-2024-08-06"),
        ("openai", "gpt-4o-latest"),
        ("openai", "gpt-4o-mini"),
        ("openai", "gpt-4o-mini-2024-07-18"),
        ("openai", "gpt-4-turbo"),
        ("openai", "gpt-4-turbo-2024-04-09"),
        ("openai", "gpt-4-turbo-preview"),
        ("openai", "gpt-4"),
        ("openai", "gpt-4-1106-preview"),
        ("openai", "gpt-3.5-turbo"),
        ("openai", "gpt-3.5-turbo-0613"),
        ("openai", "gpt-3.5-turbo-16k"),
        ("openai", "chatgpt-4o-latest"),
        ("openai", "o1"),
        ("openai", "o1-preview"),
        ("openai", "o1-mini"),
        ("openai", "o3-mini"),
    ];

    println!("\n📊 Canonical Models vs Pricing Cache Coverage\n");
    println!("{:<15} {:<40} {:<12} {:<12}", "Provider", "Model", "Canonical", "Pricing");
    println!("{}", "=".repeat(85));

    let mut canonical_only = 0;
    let mut pricing_only = 0;
    let mut both = 0;
    let mut neither = 0;

    for (provider, model) in &test_cases {
        // Try canonical lookup
        let canonical_id = canonical_name(provider, model);
        let in_canonical = registry.all_models()
            .into_iter()
            .any(|m| m.id == canonical_id);

        // Try pricing lookup
        let in_pricing = get_model_pricing(provider, model).await.is_some();

        let canonical_status = if in_canonical { "✅" } else { "❌" };
        let pricing_status = if in_pricing { "✅" } else { "❌" };

        println!(
            "{:<15} {:<40} {:<12} {:<12}",
            provider, model, canonical_status, pricing_status
        );

        match (in_canonical, in_pricing) {
            (true, true) => both += 1,
            (true, false) => canonical_only += 1,
            (false, true) => pricing_only += 1,
            (false, false) => neither += 1,
        }
    }

    let total = test_cases.len();
    println!("\n{}", "=".repeat(85));
    println!("📈 Coverage Analysis:");
    println!("   Both systems:        {} / {} ({:.1}%)", both, total, (both as f64 / total as f64) * 100.0);
    println!("   Canonical only:      {} / {} ({:.1}%)", canonical_only, total, (canonical_only as f64 / total as f64) * 100.0);
    println!("   Pricing only:        {} / {} ({:.1}%)", pricing_only, total, (pricing_only as f64 / total as f64) * 100.0);
    println!("   Neither:             {} / {} ({:.1}%)", neither, total, (neither as f64 / total as f64) * 100.0);

    println!("\n💡 Key Insight:");
    println!("   Canonical models use smart normalization → Better coverage for variant names");
    println!("   Pricing cache uses raw OpenRouter names → Only exact matches work");
}
