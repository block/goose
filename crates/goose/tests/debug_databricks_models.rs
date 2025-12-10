/// Debug script to see all Databricks models
/// Run with: cargo test --test debug_databricks_models -- --nocapture --ignored

use goose::providers::create_with_named_model;

#[tokio::test]
#[ignore]
async fn debug_databricks_models() {
    println!("\n🔍 Fetching All Databricks Models\n");

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
        Ok(Some(models)) => {
            println!("✅ Fetched {} models from Databricks\n", models.len());
            models
        }
        Ok(None) => {
            println!("⚠️  Databricks does not support model listing");
            return;
        }
        Err(e) => {
            println!("❌ Failed to fetch models: {}", e);
            return;
        }
    };

    println!("📋 All Databricks Models:\n");
    for (idx, model) in models.iter().enumerate() {
        println!("  {:3}. {}", idx + 1, model);
    }

    // Categorize models
    let mut claude_models = Vec::new();
    let mut gpt_models = Vec::new();
    let mut gemini_models = Vec::new();
    let mut llama_models = Vec::new();
    let mut custom_models = Vec::new();

    for model in &models {
        let lower = model.to_lowercase();
        if lower.contains("claude") {
            claude_models.push(model);
        } else if lower.contains("gpt") || lower.starts_with("o1") || lower.starts_with("o3") {
            gpt_models.push(model);
        } else if lower.contains("gemini") || lower.contains("gemma") {
            gemini_models.push(model);
        } else if lower.contains("llama") {
            llama_models.push(model);
        } else {
            custom_models.push(model);
        }
    }

    println!("\n📊 Categorization:\n");
    println!("  Claude models (Anthropic): {}", claude_models.len());
    for model in &claude_models {
        println!("    - {}", model);
    }

    println!("\n  GPT models (OpenAI): {}", gpt_models.len());
    for model in &gpt_models {
        println!("    - {}", model);
    }

    println!("\n  Gemini models (Google): {}", gemini_models.len());
    for model in &gemini_models {
        println!("    - {}", model);
    }

    println!("\n  Llama models: {}", llama_models.len());
    for model in &llama_models {
        println!("    - {}", model);
    }

    println!("\n  Custom/Internal models: {}", custom_models.len());
    for model in &custom_models {
        println!("    - {}", model);
    }
}
