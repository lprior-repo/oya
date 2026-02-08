//! Compression benchmarks for checkpoint data.
//!
//! Run with: cargo bench --bench compression

#![allow(clippy::significant_drop_in_scrutinee)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oya_workflow::checkpoint::compression::{compress, compression_ratio, decompress};

/// Generate test data with varying characteristics.
fn generate_data(size: usize, pattern: DataPattern) -> Vec<u8> {
    match pattern {
        DataPattern::Random => {
            // Random-like data (worst case for compression)
            (0..size).map(|i| (i % 256) as u8).collect()
        }
        DataPattern::Repetitive => {
            // Highly repetitive data (best case for compression)
            b"Hello, world! This is repetitive data. "
                .iter()
                .cycle()
                .take(size)
                .copied()
                .collect()
        }
        DataPattern::Structured => {
            // Structured data (typical checkpoint data)
            let mut data = Vec::with_capacity(size);
            while data.len() < size {
                // Simulate structured data with some repetition
                data.extend_from_slice(b"{\"worker_id\":\"worker-");
                data.extend_from_slice(format!("{}", data.len() / 100).as_bytes());
                data.extend_from_slice(b"\",\"state\":\"Running");
                data.extend_from_slice(format!("{}", data.len() % 10).as_bytes());
                data.extend_from_slice(b"\",\"checkpoint\":");
                data.extend_from_slice(format!("{},", data.len()).as_bytes());
            }
            data.truncate(size);
            data
        }
    }
}

/// Data patterns for benchmarking.
#[derive(Debug, Clone, Copy)]
enum DataPattern {
    Random,
    Repetitive,
    Structured,
}

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    let sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024]; // 1KB, 10KB, 100KB, 1MB

    for size in sizes {
        // Benchmark with structured data (typical checkpoint)
        let data = generate_data(size, DataPattern::Structured);
        group.bench_with_input(
            BenchmarkId::new("compress_structured", size),
            &data,
            |b, data| {
                b.iter(|| compress(black_box(data)));
            },
        );

        // Benchmark with repetitive data (best case)
        let data = generate_data(size, DataPattern::Repetitive);
        group.bench_with_input(
            BenchmarkId::new("compress_repetitive", size),
            &data,
            |b, data| {
                b.iter(|| compress(black_box(data)));
            },
        );

        // Benchmark with random data (worst case)
        let data = generate_data(size, DataPattern::Random);
        group.bench_with_input(
            BenchmarkId::new("compress_random", size),
            &data,
            |b, data| {
                b.iter(|| compress(black_box(data)));
            },
        );
    }

    group.finish();
}

fn bench_decompression(c: &mut Criterion) {
    let mut group = c.benchmark_group("decompression");

    let sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024];

    for size in sizes {
        let data = generate_data(size, DataPattern::Structured);
        let compressed = match compress(&data) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Compression failed: {}", e);
                continue;
            }
        };

        group.bench_with_input(
            BenchmarkId::new("decompress_structured", size),
            &compressed,
            |b, compressed| {
                b.iter(|| decompress(black_box(compressed), black_box(size)).ok());
            },
        );
    }

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    let sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024];

    for size in sizes {
        // Structured data
        let data = generate_data(size, DataPattern::Structured);
        let compressed = match compress(&data) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Compression failed: {}", e);
                continue;
            }
        };
        let ratio = compression_ratio(size as u64, compressed.len() as u64);

        group.bench_with_input(BenchmarkId::new("ratio_structured", size), &size, |b, _| {
            b.iter(|| black_box(ratio));
        });

        println!(
            "Structured data ({} bytes): compressed to {} bytes (ratio: {:.2}x, {:.1}%)",
            size,
            compressed.len(),
            ratio,
            100.0 / ratio
        );

        // Repetitive data
        let data = generate_data(size, DataPattern::Repetitive);
        let compressed = match compress(&data) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Compression failed: {}", e);
                continue;
            }
        };
        let ratio = compression_ratio(size as u64, compressed.len() as u64);

        println!(
            "Repetitive data ({} bytes): compressed to {} bytes (ratio: {:.2}x, {:.1}%)",
            size,
            compressed.len(),
            ratio,
            100.0 / ratio
        );

        // Random data
        let data = generate_data(size, DataPattern::Random);
        let compressed = match compress(&data) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Compression failed: {}", e);
                continue;
            }
        };
        let ratio = compression_ratio(size as u64, compressed.len() as u64);

        println!(
            "Random data ({} bytes): compressed to {} bytes (ratio: {:.2}x, {:.1}%)",
            size,
            compressed.len(),
            ratio,
            100.0 / ratio
        );
    }

    group.finish();
}

fn bench_compression_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_levels");

    let data = generate_data(100 * 1024, DataPattern::Structured);

    for level in [1, 3, 9, 15, 21] {
        group.bench_with_input(BenchmarkId::new("level", level), &level, |b, &level| {
            b.iter(|| {
                oya_workflow::checkpoint::compression::compress_with_level(
                    black_box(&data),
                    black_box(level),
                )
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_compression,
    bench_decompression,
    bench_compression_ratio,
    bench_compression_levels
);
criterion_main!(benches);
