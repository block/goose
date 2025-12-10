/// Test to check pricing lookup hit rate for common models
/// Run with: cargo test --test test_pricing_hit_rate -- --nocapture

use goose::providers::pricing::{get_model_pricing, initialize_pricing_cache};

#[tokio::test]
async fn test_pricing_hit_rate() {
    // Initialize cache
    if let Err(e) = initialize_pricing_cache().await {
        println!("⚠️  Failed to initialize pricing cache: {}", e);
        println!("    This test requires network access to fetch pricing data");
        return;
    }

    // Common model names that users might request
    let test_cases = vec![
        // Anthropic models
        ("anthropic", "claude-3.5-sonnet"),
        ("anthropic", "claude-3-5-sonnet"),
        ("anthropic", "claude-3.5-sonnet-20241022"),
        ("anthropic", "claude-sonnet-4"),
        ("anthropic", "claude-sonnet-4.5"),
        ("anthropic", "claude-opus-4"),
        ("anthropic", "claude-haiku-4.5"),

        // OpenAI models
        ("openai", "gpt-4o"),
        ("openai", "gpt-4o-2024-11-20"),
        ("openai", "gpt-4o-mini"),
        ("openai", "gpt-4-turbo"),
        ("openai", "gpt-4"),
        ("openai", "gpt-3.5-turbo"),
        ("openai", "o1"),
        ("openai", "o1-pro"),
        ("openai", "o3-mini"),

        // Google models
        ("google", "gemini-2.5-flash"),
        ("google", "gemini-2.5-flash-preview"),
        ("google", "gemini-2.5-pro"),
        ("google", "gemini-2.0-flash"),
    ];

    let mut hits = 0;
    let mut misses = 0;
    let mut miss_list = Vec::new();

    println!("\n📊 Testing Pricing Lookup Hit Rate\n");
    println!("{:<20} {:<40} {}", "Provider", "Model", "Result");
    println!("{}", "=".repeat(70));

    for (provider, model) in test_cases {
        match get_model_pricing(provider, model).await {
            Some(pricing) => {
                hits += 1;
                println!(
                    "{:<20} {:<40} ✅ ${:.6} / ${:.6}",
                    provider, model, pricing.input_cost, pricing.output_cost
                );
            }
            None => {
                misses += 1;
                miss_list.push((provider, model));
                println!("{:<20} {:<40} ❌ NOT FOUND", provider, model);
            }
        }
    }

    let total = hits + misses;
    let hit_rate = (hits as f64 / total as f64) * 100.0;

    println!("\n{}", "=".repeat(70));
    println!("📈 Results:");
    println!("   Hits:     {} / {} ({:.1}%)", hits, total, hit_rate);
    println!("   Misses:   {} / {} ({:.1}%)", misses, total, 100.0 - hit_rate);

    if !miss_list.is_empty() {
        println!("\n❌ Missed lookups:");
        for (provider, model) in miss_list {
            println!("   - {}/{}", provider, model);
        }
    }

    println!("\n💡 Note: Misses could be due to:");
    println!("   1. Model name normalization issues");
    println!("   2. Model not in OpenRouter catalog");
    println!("   3. Outdated cache");

    // Show what models are actually available in cache for Anthropic
    use goose::providers::pricing::get_all_pricing;
    let all_pricing = get_all_pricing().await;

    if let Some(anthropic_models) = all_pricing.get("anthropic") {
        println!("\n📦 Sample models in cache (Anthropic):");
        let mut models: Vec<_> = anthropic_models.keys().collect();
        models.sort();
        for model in models.iter().take(10) {
            println!("   - {}", model);
        }
        println!("   ... ({} total anthropic models)", anthropic_models.len());
    }
}
