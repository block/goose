use goose::providers::pricing::{get_model_pricing, initialize_pricing_cache, refresh_pricing};
use std::time::Instant;
use tempfile::TempDir;

#[tokio::test]
async fn test_pricing_cache_performance() {
    // Use a unique cache directory for this test to avoid conflicts
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("GOOSE_CACHE_DIR", temp_dir.path());

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

    // First fetch (potentially uncached or cache warming)
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

    // Run many iterations to test cache performance
    const ITERATIONS: u32 = 100;
    let mut total_duration = std::time::Duration::ZERO;
    let mut min_duration = std::time::Duration::MAX;
    let mut max_duration = std::time::Duration::ZERO;

    for i in 0..ITERATIONS {
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
        let iteration_duration = start.elapsed();
        total_duration += iteration_duration;
        min_duration = min_duration.min(iteration_duration);
        max_duration = max_duration.max(iteration_duration);

        // Print progress every 20 iterations
        if (i + 1) % 20 == 0 {
            println!("Completed {} iterations", i + 1);
        }
    }

    let avg_duration = total_duration / ITERATIONS;

    println!("\nCache performance over {} iterations:", ITERATIONS);
    println!("  Average duration: {:?}", avg_duration);
    println!("  Min duration: {:?}", min_duration);
    println!("  Max duration: {:?}", max_duration);
    println!("  First fetch duration: {:?}", first_fetch_duration);

    // The average cached fetch should not be slower than the first fetch
    // We allow some margin for variance and system load
    assert!(
        avg_duration <= first_fetch_duration,
        "Average cache fetch ({:?}) should not be slower than initial fetch ({:?})",
        avg_duration,
        first_fetch_duration
    );

    // Also check that eventually (min duration) the cache is faster
    // This ensures that after warming up, the cache provides benefit
    assert!(
        min_duration <= first_fetch_duration,
        "Best cache performance ({:?}) should be at least as fast as initial fetch ({:?})",
        min_duration,
        first_fetch_duration
    );

    // Clean up
    std::env::remove_var("GOOSE_CACHE_DIR");
}

#[tokio::test]
async fn test_pricing_refresh() {
    // Use a unique cache directory for this test to avoid conflicts
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("GOOSE_CACHE_DIR", temp_dir.path());

    const MAX_RETRIES: u32 = 5;
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < MAX_RETRIES {
        attempt += 1;
        println!("Attempt {} of {}", attempt, MAX_RETRIES);

        // Try to run the test
        match run_pricing_refresh_test().await {
            Ok(_) => {
                println!("Test passed on attempt {}", attempt);
                break;
            }
            Err(e) => {
                println!("Attempt {} failed: {}", attempt, e);
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    println!("Retrying in 1 second...");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    // If all attempts failed, panic with the last error
    if attempt == MAX_RETRIES && last_error.is_some() {
        panic!(
            "Test failed after {} attempts. Last error: {}",
            MAX_RETRIES,
            last_error.unwrap()
        );
    }

    // Clean up
    std::env::remove_var("GOOSE_CACHE_DIR");
}

async fn run_pricing_refresh_test() -> Result<(), String> {
    // Initialize first
    initialize_pricing_cache()
        .await
        .map_err(|e| format!("Failed to initialize pricing cache: {}", e))?;

    // Get initial pricing (using a model that actually exists)
    let initial_pricing = get_model_pricing("anthropic", "claude-3.5-sonnet").await;
    if initial_pricing.is_none() {
        return Err("Expected initial pricing but got None".to_string());
    }

    // Force refresh
    let start = Instant::now();
    refresh_pricing()
        .await
        .map_err(|e| format!("Failed to refresh pricing: {}", e))?;
    let refresh_duration = start.elapsed();
    println!("Pricing refresh took: {:?}", refresh_duration);

    // Get pricing after refresh
    let refreshed_pricing = get_model_pricing("anthropic", "claude-3.5-sonnet").await;
    if refreshed_pricing.is_none() {
        return Err("Expected pricing after refresh but got None".to_string());
    }

    Ok(())
}

#[tokio::test]
async fn test_model_not_in_openrouter() {
    // Use a unique cache directory for this test to avoid conflicts
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("GOOSE_CACHE_DIR", temp_dir.path());

    initialize_pricing_cache()
        .await
        .expect("Failed to initialize pricing cache");

    // Test a model that likely doesn't exist
    let pricing = get_model_pricing("fake-provider", "fake-model").await;
    assert!(
        pricing.is_none(),
        "Should return None for non-existent model"
    );

    // Clean up
    std::env::remove_var("GOOSE_CACHE_DIR");
    // TempDir automatically cleans up when dropped
}

#[tokio::test]
async fn test_concurrent_access() {
    // Use a unique cache directory for this test to avoid conflicts
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("GOOSE_CACHE_DIR", temp_dir.path());

    const MAX_RETRIES: u32 = 5;
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < MAX_RETRIES {
        attempt += 1;
        println!("Attempt {} of {}", attempt, MAX_RETRIES);

        // Try to run the test
        match run_concurrent_access_test().await {
            Ok(_) => {
                println!("Test passed on attempt {}", attempt);
                break;
            }
            Err(e) => {
                println!("Attempt {} failed: {}", attempt, e);
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    println!("Retrying in 1 second...");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    // If all attempts failed, panic with the last error
    if attempt == MAX_RETRIES && last_error.is_some() {
        panic!(
            "Test failed after {} attempts. Last error: {}",
            MAX_RETRIES,
            last_error.unwrap()
        );
    }

    // Clean up
    std::env::remove_var("GOOSE_CACHE_DIR");
}

async fn run_concurrent_access_test() -> Result<(), String> {
    use tokio::task;

    initialize_pricing_cache()
        .await
        .map_err(|e| format!("Failed to initialize pricing cache: {}", e))?;

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
    for (idx, handle) in handles.into_iter().enumerate() {
        let (task_id, has_pricing, duration) = handle
            .await
            .map_err(|e| format!("Task {} panicked: {}", idx, e))?;

        if !has_pricing {
            return Err(format!("Task {} should have gotten pricing", task_id));
        }
        println!("Task {} took: {:?}", task_id, duration);
    }

    Ok(())
}
