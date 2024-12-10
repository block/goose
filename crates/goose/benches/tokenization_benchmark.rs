use criterion::{black_box, criterion_group, criterion_main, Criterion};
use goose::token_counter::TokenCounter;

fn benchmark_tokenization(c: &mut Criterion) {
    let counter = TokenCounter::new();
    let lengths = [1_000, 5_000, 10_000, 50_000, 100_000, 124_000, 200_000];
    let models = [
        "gpt-4o",
        "claude-3.5-sonnet"
    ];

    for &length in &lengths {
        for model_name in models {
            let text = "hello ".repeat(length);
            c.bench_function(&format!("{}_{}_tokens", model_name, length), |b| {
                b.iter(|| counter.count_tokens(black_box(&text), Some(model_name)))
            });
        }
    }
}

criterion_group!(benches, benchmark_tokenization);
criterion_main!(benches);
