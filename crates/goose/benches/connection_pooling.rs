use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use goose::providers::provider_common::{create_provider_client, get_shared_client};
use reqwest::Client;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_new_clients(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("create_new_client", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _client = black_box(create_provider_client(Some(600)).unwrap());
            })
        })
    });
}

fn reuse_shared_client(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("get_shared_client", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _client = black_box(get_shared_client());
            })
        })
    });
}

fn concurrent_requests_new_clients(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_requests_new");
    for num_requests in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_requests),
            num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    rt.block_on(async {
                        let tasks: Vec<_> = (0..num_requests)
                            .map(|_| {
                                tokio::spawn(async move {
                                    let client = create_provider_client(Some(600)).unwrap();
                                    // Simulate a request (without actually making one)
                                    black_box(&client);
                                })
                            })
                            .collect();

                        for task in tasks {
                            task.await.unwrap();
                        }
                    })
                })
            },
        );
    }
    group.finish();
}

fn concurrent_requests_shared_client(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_requests_shared");
    for num_requests in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_requests),
            num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    rt.block_on(async {
                        let tasks: Vec<_> = (0..num_requests)
                            .map(|_| {
                                tokio::spawn(async move {
                                    let client = get_shared_client();
                                    // Simulate a request (without actually making one)
                                    black_box(&client);
                                })
                            })
                            .collect();

                        for task in tasks {
                            task.await.unwrap();
                        }
                    })
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    create_new_clients,
    reuse_shared_client,
    concurrent_requests_new_clients,
    concurrent_requests_shared_client
);
criterion_main!(benches);
