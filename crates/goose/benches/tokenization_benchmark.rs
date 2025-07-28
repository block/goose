use criterion::{black_box, criterion_group, criterion_main, Criterion};
use goose::token_counter::TokenCounter;

fn benchmark_tokenization(c: &mut Criterion) {
    let lengths = [1_000, 5_000, 10_000, 50_000, 100_000, 124_000, 200_000];

    // Create a single token counter using the fixed o200k_base encoding
    let counter = TokenCounter::new(); // Uses fixed o200k_base encoding

    for &length in &lengths {
        let text = "hello ".repeat(length);
        c.bench_function(&format!("o200k_base_{}_tokens", length), |b| {
            b.iter(|| counter.count_tokens(black_box(&text)))
        });
    }
}

fn benchmark_async_tokenization(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let lengths = [1_000, 5_000, 10_000, 50_000, 100_000, 124_000, 200_000];

    // Create an async token counter
    let counter = rt.block_on(async {
        goose::token_counter::create_async_token_counter()
            .await
            .unwrap()
    });

    for &length in &lengths {
        let text = "hello ".repeat(length);
        c.bench_function(&format!("async_o200k_base_{}_tokens", length), |b| {
            b.iter(|| counter.count_tokens(black_box(&text)))
        });
    }
}

fn benchmark_cache_performance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create an async token counter for cache testing
    let counter = rt.block_on(async {
        goose::token_counter::create_async_token_counter()
            .await
            .unwrap()
    });

    let test_texts = vec![
        "This is a test sentence for cache performance.",
        "Another different sentence to test caching.",
        "A third unique sentence for the benchmark.",
        "This is a test sentence for cache performance.", // Repeat first one
        "Another different sentence to test caching.",    // Repeat second one
    ];

    c.bench_function("cache_hit_miss_pattern", |b| {
        b.iter(|| {
            for text in &test_texts {
                counter.count_tokens(black_box(text));
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_tokenization,
    benchmark_async_tokenization,
    benchmark_cache_performance
);
criterion_main!(benches);
