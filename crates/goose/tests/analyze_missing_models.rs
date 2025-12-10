/// Analyze which models are NOT matching to find patterns
/// Run with: cargo test --test analyze_missing_models -- --nocapture --ignored

use goose::providers::canonical::{fuzzy_canonical_name, CanonicalModelRegistry};
use goose::providers::create_with_named_model;

#[tokio::test]
#[ignore]
async fn analyze_missing_models() {
    println!("\n🔍 Analyzing Missing Databricks Models\n");

    // Load canonical registry
    let registry = match CanonicalModelRegistry::bundled() {
        Ok(reg) => reg,
        Err(e) => {
            println!("❌ Failed to load canonical registry: {}", e);
            return;
        }
    };

    // Create Databricks provider
    let provider = match create_with_named_model("databricks", "databricks-meta-llama-3-1-70b-instruct").await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️  Databricks not configured: {}", e);
            return;
        }
    };

    // Fetch models
    let models = match provider.fetch_supported_models().await {
        Ok(Some(models)) => models,
        Ok(None) => {
            println!("⚠️  Databricks does not support model listing");
            return;
        }
        Err(e) => {
            println!("❌ Failed to fetch models: {}", e);
            return;
        }
    };

    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for model in &models {
        let candidates = fuzzy_canonical_name("databricks", model);
        let is_match = candidates.iter().any(|canonical_id| {
            registry.all_models()
                .into_iter()
                .any(|m| m.id == *canonical_id)
        });

        if is_match {
            matched.push((model.clone(), candidates));
        } else {
            unmatched.push((model.clone(), candidates));
        }
    }

    println!("✅ Matched: {} / {} ({:.1}%)", matched.len(), models.len(),
        (matched.len() as f64 / models.len() as f64) * 100.0);
    println!("❌ Unmatched: {} / {} ({:.1}%)\n", unmatched.len(), models.len(),
        (unmatched.len() as f64 / models.len() as f64) * 100.0);

    // Categorize unmatched models
    let mut claude_unmatched = Vec::new();
    let mut gpt_unmatched = Vec::new();
    let mut gemini_unmatched = Vec::new();
    let mut o_series_unmatched = Vec::new();
    let mut custom_unmatched = Vec::new();
    let mut embedding_unmatched = Vec::new();

    for (model, candidates) in &unmatched {
        let lower = model.to_lowercase();

        if lower.contains("embedding") || lower.contains("embed") {
            embedding_unmatched.push((model, candidates));
        } else if lower.contains("claude") {
            claude_unmatched.push((model, candidates));
        } else if lower.contains("gpt") {
            gpt_unmatched.push((model, candidates));
        } else if lower.contains("gemini") || lower.contains("gemma") {
            gemini_unmatched.push((model, candidates));
        } else if lower.starts_with("o1") || lower.starts_with("o3") || lower.starts_with("o4") {
            o_series_unmatched.push((model, candidates));
        } else {
            custom_unmatched.push((model, candidates));
        }
    }

    // Show Claude unmatched
    if !claude_unmatched.is_empty() {
        println!("🤖 Claude Models NOT Matching ({}):", claude_unmatched.len());
        for (model, candidates) in &claude_unmatched {
            println!("  ❌ {}", model);
            println!("     Tried: {:?}", candidates);
        }
        println!();
    }

    // Show GPT unmatched
    if !gpt_unmatched.is_empty() {
        println!("🤖 GPT Models NOT Matching ({}):", gpt_unmatched.len());
        for (model, candidates) in &gpt_unmatched {
            println!("  ❌ {}", model);
            println!("     Tried: {:?}", candidates);
        }
        println!();
    }

    // Show Gemini unmatched
    if !gemini_unmatched.is_empty() {
        println!("🤖 Gemini Models NOT Matching ({}):", gemini_unmatched.len());
        for (model, candidates) in &gemini_unmatched {
            println!("  ❌ {}", model);
            println!("     Tried: {:?}", candidates);
        }
        println!();
    }

    // Show O-series unmatched
    if !o_series_unmatched.is_empty() {
        println!("🤖 O-Series Models NOT Matching ({}):", o_series_unmatched.len());
        for (model, candidates) in &o_series_unmatched {
            println!("  ❌ {}", model);
            println!("     Tried: {:?}", candidates);
        }
        println!();
    }

    // Show embedding models
    if !embedding_unmatched.is_empty() {
        println!("📊 Embedding Models NOT Matching ({}):", embedding_unmatched.len());
        for (model, _) in &embedding_unmatched {
            println!("  ❌ {}", model);
        }
        println!("  💡 Note: Embedding models aren't in the canonical registry (chat models only)\n");
    }

    // Show custom/internal
    if !custom_unmatched.is_empty() {
        println!("🏢 Custom/Internal Models NOT Matching ({}):", custom_unmatched.len());
        for (model, _) in &custom_unmatched {
            println!("  ❌ {}", model);
        }
        println!("  💡 Note: These are internal Databricks models, not in canonical registry\n");
    }

    // Pattern analysis
    println!("📊 Pattern Analysis:");
    println!("  Claude unmatched:    {}", claude_unmatched.len());
    println!("  GPT unmatched:       {}", gpt_unmatched.len());
    println!("  Gemini unmatched:    {}", gemini_unmatched.len());
    println!("  O-series unmatched:  {}", o_series_unmatched.len());
    println!("  Embedding models:    {}", embedding_unmatched.len());
    println!("  Custom/Internal:     {}", custom_unmatched.len());
}
