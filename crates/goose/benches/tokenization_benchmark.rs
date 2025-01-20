use criterion::{black_box, criterion_group, criterion_main, Criterion};
use goose::token_counter::TokenCounter;

fn benchmark_tokenization(c: &mut Criterion) {
    let lengths = [1_000, 5_000, 10_000, 50_000, 100_000, 124_000, 200_000];
    let tokenizer_names = ["Xenova--gpt-4o", "Xenova--claude-tokenizer"];

    for tokenizer_name in tokenizer_names {
        let counter = TokenCounter::new(tokenizer_name);
        for &length in &lengths {
            let text = "hello ".repeat(length);
            c.bench_function(&format!("{}_{}_tokens", tokenizer_name, length), |b| {
                b.iter(|| counter.count_tokens(black_box(&text)))
            });
        }
    }
}

criterion_group!(benches, benchmark_tokenization);
criterion_main!(benches);
