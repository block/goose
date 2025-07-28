/// Demo showing the async token counter improvement
///
/// This example demonstrates the key improvement: no blocking runtime creation
///
/// BEFORE (blocking):
/// ```rust
/// let content = tokio::runtime::Runtime::new()?.block_on(async {
///     let response = reqwest::get(&file_url).await?;
///     // ... download logic
/// })?;
/// ```
///
/// AFTER (async):
/// ```rust
/// let client = reqwest::Client::new();
/// let response = client.get(&file_url).send().await?;
/// let bytes = response.bytes().await?;
/// tokio::fs::write(&file_path, bytes).await?;
/// ```
use goose::token_counter::{create_async_token_counter, TokenCounter};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Async Token Counter Demo");
    println!("===========================");

    // Test text samples
    let samples = vec![
        "Hello, world!",
        "This is a longer text sample for tokenization testing.",
        "The quick brown fox jumps over the lazy dog.",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "async/await patterns eliminate blocking operations",
    ];

    println!("\n📊 Performance Comparison");
    println!("-------------------------");

    // Test original TokenCounter
    let start = Instant::now();
    let sync_counter = TokenCounter::new();
    let sync_init_time = start.elapsed();

    let start = Instant::now();
    let mut sync_total = 0;
    for sample in &samples {
        sync_total += sync_counter.count_tokens(sample);
    }
    let sync_count_time = start.elapsed();

    println!("🔴 Synchronous TokenCounter:");
    println!("   Init time: {:?}", sync_init_time);
    println!("   Count time: {:?}", sync_count_time);
    println!("   Total tokens: {}", sync_total);

    // Test AsyncTokenCounter
    let start = Instant::now();
    let async_counter = create_async_token_counter().await?;
    let async_init_time = start.elapsed();

    let start = Instant::now();
    let mut async_total = 0;
    for sample in &samples {
        async_total += async_counter.count_tokens(sample);
    }
    let async_count_time = start.elapsed();

    println!("\n🟢 Async TokenCounter:");
    println!("   Init time: {:?}", async_init_time);
    println!("   Count time: {:?}", async_count_time);
    println!("   Total tokens: {}", async_total);
    println!("   Cache size: {}", async_counter.cache_size());

    // Test caching benefit
    let start = Instant::now();
    let mut cached_total = 0;
    for sample in &samples {
        cached_total += async_counter.count_tokens(sample); // Should hit cache
    }
    let cached_time = start.elapsed();

    println!("\n⚡ Cached TokenCounter (2nd run):");
    println!("   Count time: {:?}", cached_time);
    println!("   Total tokens: {}", cached_total);
    println!("   Cache size: {}", async_counter.cache_size());

    // Verify same results
    assert_eq!(sync_total, async_total);
    assert_eq!(async_total, cached_total);

    println!(
        "   Token result caching: {}x faster on cached text",
        async_count_time.as_nanos() / cached_time.as_nanos().max(1)
    );

    Ok(())
}
