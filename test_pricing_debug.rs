use goose::providers::pricing::{parse_model_id, get_model_pricing, get_all_pricing};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test the parse_model_id function
    println!("Testing parse_model_id function:");
    
    let test_cases = vec![
        "anthropic/claude-sonnet-4",
        "anthropic/claude-3.5-sonnet",
        "openai/gpt-4",
        "invalid-format"
    ];
    
    for model_id in test_cases {
        match parse_model_id(model_id) {
            Some((provider, model)) => {
                println!("  {} -> provider: '{}', model: '{}'", model_id, provider, model);
            }
            None => {
                println!("  {} -> failed to parse", model_id);
            }
        }
    }
    
    println!("\nTesting get_model_pricing for anthropic/claude-sonnet-4:");
    
    // Test the specific model that's failing
    match get_model_pricing("anthropic", "claude-sonnet-4").await {
        Some(pricing) => {
            println!("  Found pricing: input_cost={}, output_cost={}, context_length={:?}", 
                     pricing.input_cost, pricing.output_cost, pricing.context_length);
        }
        None => {
            println!("  No pricing found for anthropic/claude-sonnet-4");
        }
    }
    
    println!("\nTesting all cached anthropic models:");
    let all_pricing = get_all_pricing().await;
    if let Some(anthropic_models) = all_pricing.get("anthropic") {
        println!("  Found {} anthropic models in cache:", anthropic_models.len());
        for (model_name, pricing) in anthropic_models {
            if model_name.contains("sonnet-4") {
                println!("    {} -> input_cost={}, output_cost={}", 
                         model_name, pricing.input_cost, pricing.output_cost);
            }
        }
    } else {
        println!("  No anthropic models found in cache");
    }
    
    Ok(())
}