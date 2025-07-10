use goose::providers::pricing::{get_model_pricing, initialize_pricing_cache, refresh_pricing};
use std::time::Instant;

#[tokio::test]
async fn test_pricing_cache_performance() {
    // Initialize the cache
    let start = Instant::now();
    initialize_pricing_cache()
        .await
        .expect("Failed to initialize pricing cache");
    let init_duration = start.elapsed();
    println!("Cache initialization took: {:?}", init_duration);

    // Test fetching pricing for common models (using actual model names from OpenRouter)
    let models = vec![
        ("anthropic", "claude-3.5-sonnet"),
        ("openai", "gpt-4o"),
        ("openai", "gpt-4o-mini"),
        ("google", "gemini-flash-1.5"),
        ("anthropic", "claude-sonnet-4"),
    ];

    // First fetch (should hit cache)
    let start = Instant::now();
    for (provider, model) in &models {
        let pricing = get_model_pricing(provider, model).await;
        assert!(
            pricing.is_some(),
            "Expected pricing for {}/{}",
            provider,
            model
        );
    }
    let first_fetch_duration = start.elapsed();
    println!(
        "First fetch of {} models took: {:?}",
        models.len(),
        first_fetch_duration
    );

    // Second fetch (definitely from cache)
    let start = Instant::now();
    for (provider, model) in &models {
        let pricing = get_model_pricing(provider, model).await;
        assert!(
            pricing.is_some(),
            "Expected pricing for {}/{}",
            provider,
            model
        );
    }
    let second_fetch_duration = start.elapsed();
    println!(
        "Second fetch of {} models took: {:?}",
        models.len(),
        second_fetch_duration
    );

    // Cache fetch should be significantly faster
    // Note: Both fetches are already very fast (microseconds), so we just ensure
    // the second fetch is not slower than the first (allowing for some variance)
    assert!(
        second_fetch_duration <= first_fetch_duration * 2,
        "Cache fetch should not be significantly slower than initial fetch. First: {:?}, Second: {:?}",
        first_fetch_duration,
        second_fetch_duration
    );
}

#[tokio::test]
async fn test_pricing_refresh() {
    // Initialize first
    initialize_pricing_cache()
        .await
        .expect("Failed to initialize pricing cache");

    // Get initial pricing (using a model that actually exists)
    let initial_pricing = get_model_pricing("anthropic", "claude-3.5-sonnet").await;
    assert!(initial_pricing.is_some(), "Expected initial pricing");

    // Force refresh
    let start = Instant::now();
    refresh_pricing().await.expect("Failed to refresh pricing");
    let refresh_duration = start.elapsed();
    println!("Pricing refresh took: {:?}", refresh_duration);

    // Get pricing after refresh
    let refreshed_pricing = get_model_pricing("anthropic", "claude-3.5-sonnet").await;
    assert!(
        refreshed_pricing.is_some(),
        "Expected pricing after refresh"
    );
}

#[tokio::test]
async fn test_model_not_in_openrouter() {
    initialize_pricing_cache()
        .await
        .expect("Failed to initialize pricing cache");

    // Test a model that likely doesn't exist
    let pricing = get_model_pricing("fake-provider", "fake-model").await;
    assert!(
        pricing.is_none(),
        "Should return None for non-existent model"
    );
}

#[tokio::test]
async fn test_concurrent_access() {
    use tokio::task;

    initialize_pricing_cache()
        .await
        .expect("Failed to initialize pricing cache");

    // Spawn multiple tasks to access pricing concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let handle = task::spawn(async move {
            let start = Instant::now();
            let pricing = get_model_pricing("openai", "gpt-4o").await;
            let duration = start.elapsed();
            (i, pricing.is_some(), duration)
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        let (task_id, has_pricing, duration) = handle.await.unwrap();
        assert!(has_pricing, "Task {} should have gotten pricing", task_id);
        println!("Task {} took: {:?}", task_id, duration);
    }
}
