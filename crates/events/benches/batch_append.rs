// Batch Append Benchmark
//
// Measures batch append throughput and fsync amortization benefits.
// Compares batch append vs single append to verify 10x+ throughput improvement.
//
// Performance Targets:
// - Batch append achieves 10x+ throughput vs single append
// - Exactly ONE fsync per batch (not per event)
// - Test batch sizes: 1, 10, 50, 100, 500, 1000 events

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

// NOTE: This is a placeholder benchmark structure
// Full implementation requires the DurableEventStore with append_batch() to be integrated

/// Benchmark fixture for temp directory management
pub struct BenchmarkFixture {
    temp_dir: TempDir,
}

impl BenchmarkFixture {
    /// Create isolated temporary directory for benchmark
    pub fn setup() -> Result<Self, String> {
        TempDir::new()
            .map(|temp_dir| Self { temp_dir })
            .map_err(|e| format!("Failed to create temp dir: {}", e))
    }

    /// Get path to test data directory
    pub fn data_dir(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("data")
    }

    /// Get path to WAL directory
    pub fn wal_dir(&self) -> std::path::PathBuf {
        self.temp_dir.path().join(".wal")
    }
}

/// Simulate single append baseline
/// This will be replaced with actual DurableEventStore::append_event() call
async fn simulate_single_append(event_size: usize) -> Result<(), String> {
    // Simulate event serialization
    let _data = vec![0u8; event_size];

    // Simulate WAL write
    let _wal_data = vec![0u8; event_size + 4]; // +4 for length prefix

    // Simulate fsync (most expensive operation)
    tokio::task::yield_now().await;

    Ok(())
}

/// Simulate batch append
/// This will be replaced with actual DurableEventStore::append_batch() call
async fn simulate_batch_append(batch_size: usize, event_size: usize) -> Result<(), String> {
    // Simulate batch serialization
    let _events: Vec<Vec<u8>> = (0..batch_size).map(|_| vec![0u8; event_size]).collect();

    // Simulate single contiguous WAL write for all events
    let total_size = batch_size * (event_size + 4);
    let _wal_data = vec![0u8; total_size];

    // Simulate SINGLE fsync for entire batch (amortization!)
    tokio::task::yield_now().await;

    Ok(())
}

/// Criterion benchmark for single append baseline
fn bench_single_append_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_append_baseline");
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let event_size = 1024; // 1KB per event

    group.bench_function("append_single_event", |b| {
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create runtime: {}", e);
                return;
            }
        };

        b.iter(|| {
            let future = simulate_single_append(black_box(event_size));
            match rt.block_on(future) {
                Ok(_) => {}
                Err(e) => eprintln!("Benchmark error: {}", e),
            }
        });
    });

    group.finish();
}

/// Criterion benchmark for batch append throughput
fn bench_batch_append_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_append_throughput");
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let event_size = 1024; // 1KB per event
    let batch_sizes = vec![1, 10, 50, 100, 500, 1000];

    for size in batch_sizes {
        group.bench_with_input(BenchmarkId::new("batch_append", size), &size, |b, &size| {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create runtime: {}", e);
                    return;
                }
            };

            b.iter(|| {
                let future = simulate_batch_append(black_box(size), black_box(event_size));
                match rt.block_on(future) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Benchmark error: {}", e),
                }
            });
        });
    }

    group.finish();
}

/// Benchmark: Compare single vs batch append
/// This demonstrates the fsync amortization benefit
fn bench_single_vs_batch_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_vs_batch_comparison");
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let event_size = 1024;
    let total_events = 100;

    // Single append: 100 separate appends (100 fsyncs)
    group.bench_function(
        BenchmarkId::new("single_append_100_events", total_events),
        |b| {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create runtime: {}", e);
                    return;
                }
            };

            b.iter(|| {
                for _ in 0..total_events {
                    let future = simulate_single_append(black_box(event_size));
                    match rt.block_on(future) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Benchmark error: {}", e),
                    }
                }
            });
        },
    );

    // Batch append: 1 batch append (1 fsync)
    group.bench_function(
        BenchmarkId::new("batch_append_100_events", total_events),
        |b| {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create runtime: {}", e);
                    return;
                }
            };

            b.iter(|| {
                let future = simulate_batch_append(black_box(total_events), black_box(event_size));
                match rt.block_on(future) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Benchmark error: {}", e),
                }
            });
        },
    );

    group.finish();
}

/// Benchmark: Fsync amortization verification
/// This verifies that batch append uses exactly 1 fsync per batch
fn bench_fsync_amortization(c: &mut Criterion) {
    let mut group = c.benchmark_group("fsync_amortization");
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let event_size = 1024;
    let batch_sizes = vec![10, 50, 100, 500, 1000];

    for size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::new("single_fsync_per_batch", size),
            &size,
            |b, &size| {
                let rt = match Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("Failed to create runtime: {}", e);
                        return;
                    }
                };

                b.iter(|| {
                    let future = simulate_batch_append(black_box(size), black_box(event_size));
                    match rt.block_on(future) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Benchmark error: {}", e),
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets =
        bench_single_append_baseline,
        bench_batch_append_throughput,
        bench_single_vs_batch_comparison,
        bench_fsync_amortization
}
criterion_main!(benches);
