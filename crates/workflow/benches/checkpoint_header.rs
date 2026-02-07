//! Benchmark for checkpoint header creation optimization.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oya_workflow::checkpoint::serialize::{add_version_header, CHECKPOINT_VERSION, MAGIC_BYTES};

/// Generate test data of various sizes
fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Benchmark function for the original iterator chain implementation
fn benchmark_original_impl(data: &[u8]) -> Vec<u8> {
    // Simulate the original iterator chain approach
    let header = MAGIC_BYTES
        .iter()
        .chain(CHECKPOINT_VERSION.to_le_bytes().iter())
        .chain(data.iter())
        .copied()
        .collect::<Vec<_>>();
    header
}

/// Benchmark function for the optimized pre-allocated Vec implementation
fn benchmark_optimized_impl(data: &[u8]) -> Vec<u8> {
    // Use the optimized implementation from the source
    let mut header = Vec::with_capacity(MAGIC_BYTES.len() + 4 + data.len());
    header.extend_from_slice(MAGIC_BYTES);
    header.extend_from_slice(&CHECKPOINT_VERSION.to_le_bytes());
    header.extend_from_slice(data);
    header
}

fn criterion_benchmark(c: &mut Criterion) {
    // Test various data sizes to show performance characteristics
    let test_sizes = [
        10,     // Very small (few bytes)
        100,    // Small
        1000,   // Medium
        10000,  // Large
        100000, // Very large
    ];

    for &size in &test_sizes {
        let test_data = generate_test_data(size);

        // Group benchmarks by data size
        let mut group = c.benchmark_group(format!("header_creation_{}b", size));

        // Benchmark original implementation
        group.bench_function("original", |b| {
            b.iter(|| black_box(benchmark_original_impl(&test_data)))
        });

        // Benchmark optimized implementation
        group.bench_function("optimized", |b| {
            b.iter(|| black_box(benchmark_optimized_impl(&test_data)))
        });

        // Benchmark the actual function from the source
        group.bench_function("actual_function", |b| {
            b.iter(|| black_box(add_version_header(test_data.clone())).unwrap())
        });

        group.finish();
    }
}

fn criterion_benchmark_memory(c: &mut Criterion) {
    // Memory allocation patterns test
    let large_data = generate_test_data(10_000);

    c.bench_function("memory_allocation_overhead", |b| {
        b.iter(|| {
            // Test the allocation pattern
            let mut headers = Vec::new();
            for _ in 0..100 {
                let header = benchmark_optimized_impl(&large_data);
                headers.push(header);
            }
            black_box(headers);
        })
    });
}

fn criterion_benchmark_cache_efficiency(c: &mut Criterion) {
    // Test cache locality impact
    let small_data = generate_test_data(64);
    let medium_data = generate_test_data(1024);
    let large_data = generate_test_data(16384);

    let mut group = c.benchmark_group("cache_efficiency");

    for (name, data) in [
        ("small", small_data),
        ("medium", medium_data),
        ("large", large_data),
    ] {
        group.bench_function(format!("cache_{}", name), |b| {
            b.iter(|| {
                // Repeated access to test cache patterns
                for _ in 0..1000 {
                    black_box(benchmark_optimized_impl(&data));
                }
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    criterion_benchmark,
    criterion_benchmark_memory,
    criterion_benchmark_cache_efficiency
);
criterion_main!(benches);
