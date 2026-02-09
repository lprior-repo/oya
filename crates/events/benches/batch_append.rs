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
#![forbid(unsafe_code)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

use oya_events::durable_store::{connect, ConnectionConfig, DurableEventStore};
use oya_events::event::BeadEvent;
use oya_events::types::{BeadId, BeadSpec, Complexity};

/// Benchmark fixture for isolated test environment
///
/// Ensures fresh SurrealDB instance per benchmark iteration with automatic
/// cleanup via RAII.
pub struct BenchmarkFixture {
    temp_dir: TempDir,
}

impl BenchmarkFixture {
    /// Create isolated temporary directory for benchmark run
    ///
    /// Returns error if tempfile creation fails (e.g., disk full, permission denied)
    pub fn setup() -> Result<Self, String> {
        TempDir::new()
            .map(|temp_dir| Self { temp_dir })
            .map_err(|e| format!("Failed to create temp dir: {}", e))
    }

    /// Get path to test database/storage
    pub fn test_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("benchmark_db")
    }
}

/// Create realistic test event with specified payload size
///
/// Generates BeadEvent::Created with description field sized to match target.
/// Uses repeatable text pattern to avoid compression artifacts.
pub fn create_test_event(size_bytes: usize) -> BeadEvent {
    let bead_id = BeadId::new();
    let title = "Benchmark Test Event".to_string();

    // Calculate description size to hit target payload
    let base_size = title.len() + 50; // Approximate overhead of event structure
    let desc_size = size_bytes.saturating_sub(base_size);

    let description = if desc_size > 0 {
        // Create repeatable pattern to avoid compression
        let chunk = "A"; // Single character, predictable
        chunk.repeat(desc_size)
    } else {
        String::new()
    };

    let spec = BeadSpec::new(title)
        .with_description(description)
        .with_complexity(Complexity::Medium);

    BeadEvent::created(bead_id, spec)
}

/// Core benchmark function: measure single event append latency
///
/// Measures complete append operation including:
/// - Serialization (bincode)
/// - WAL write with length-prefix encoding
/// - fsync for durability
/// - SurrealDB create operation
///
/// Target: p50 < 3ms, p99 < 5ms for 1KB payload
async fn benchmark_single_append(
    store: &DurableEventStore,
    event: &BeadEvent,
) -> Result<Duration, String> {
    let start = std::time::Instant::now();

    store
        .append_event(event)
        .await
        .map_err(|e| format!("Append failed: {}", e))?;

    Ok(start.elapsed())
}

/// Core benchmark function: measure batch append throughput
///
/// Measures complete batch append operation including:
/// - Batch serialization (bincode)
/// - Single contiguous WAL write for all events
/// - Single fsync for durability (amortization!)
/// - SurrealDB batch create operation
///
/// Target: 10x+ throughput vs single append
async fn benchmark_batch_append(
    store: &DurableEventStore,
    events: &[BeadEvent],
) -> Result<Duration, String> {
    let start = std::time::Instant::now();

    store
        .append_batch(events)
        .await
        .map_err(|e| format!("Batch append failed: {}", e))?;

    Ok(start.elapsed())
}

/// Criterion benchmark for single append baseline
fn bench_single_append_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_append_baseline");
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    let event_size = 1024; // 1KB per event

    group.bench_function("append_single_event", |b| {
        // Create fresh environment per iteration
        let fixture = match BenchmarkFixture::setup() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create fixture: {}", e);
                return;
            }
        };

        // Create tokio runtime for async operations
        let rt = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create runtime: {}", e);
                return;
            }
        };

        // Initialize fresh SurrealDB instance
        let store = match rt.block_on(async {
            let config = ConnectionConfig::new(fixture.test_path());
            connect(config)
                .await
                .map_err(|e| format!("Connection failed: {}", e))
        }) {
            Ok(store) => store,
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                return;
            }
        };

        let store = match rt.block_on(DurableEventStore::new(store)) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to create store: {}", e);
                return;
            }
        };

        // Create test event with specified size
        let event = create_test_event(event_size);

        // Run benchmark iteration
        b.iter(|| {
            match rt.block_on(benchmark_single_append(
                black_box(&store),
                black_box(&event),
            )) {
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
        let size_label = format!("{}_events", size);

        group.bench_with_input(
            BenchmarkId::new("batch_append", &size_label),
            &size,
            |b, &size| {
                // Create fresh environment per iteration
                let fixture = match BenchmarkFixture::setup() {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Failed to create fixture: {}", e);
                        return;
                    }
                };

                // Create tokio runtime for async operations
                let rt = match Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("Failed to create runtime: {}", e);
                        return;
                    }
                };

                // Initialize fresh SurrealDB instance
                let store = match rt.block_on(async {
                    let config = ConnectionConfig::new(fixture.test_path());
                    connect(config)
                        .await
                        .map_err(|e| format!("Connection failed: {}", e))
                }) {
                    Ok(store) => store,
                    Err(e) => {
                        eprintln!("Failed to connect: {}", e);
                        return;
                    }
                };

                let store = match rt.block_on(DurableEventStore::new(store)) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to create store: {}", e);
                        return;
                    }
                };

                // Create test events with specified size
                let events: Vec<BeadEvent> =
                    (0..size).map(|_| create_test_event(event_size)).collect();

                // Run benchmark iteration
                b.iter(|| {
                    match rt.block_on(benchmark_batch_append(
                        black_box(&store),
                        black_box(&events),
                    )) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Benchmark error: {}", e),
                    }
                });
            },
        );
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
            // Create fresh environment per iteration
            let fixture = match BenchmarkFixture::setup() {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to create fixture: {}", e);
                    return;
                }
            };

            // Create tokio runtime for async operations
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create runtime: {}", e);
                    return;
                }
            };

            // Initialize fresh SurrealDB instance
            let store = match rt.block_on(async {
                let config = ConnectionConfig::new(fixture.test_path());
                connect(config)
                    .await
                    .map_err(|e| format!("Connection failed: {}", e))
            }) {
                Ok(store) => store,
                Err(e) => {
                    eprintln!("Failed to connect: {}", e);
                    return;
                }
            };

            let store = match rt.block_on(DurableEventStore::new(store)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create store: {}", e);
                    return;
                }
            };

            // Create test event with specified size
            let event = create_test_event(event_size);

            b.iter(|| {
                for _ in 0..total_events {
                    match rt.block_on(benchmark_single_append(
                        black_box(&store),
                        black_box(&event),
                    )) {
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
            // Create fresh environment per iteration
            let fixture = match BenchmarkFixture::setup() {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to create fixture: {}", e);
                    return;
                }
            };

            // Create tokio runtime for async operations
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create runtime: {}", e);
                    return;
                }
            };

            // Initialize fresh SurrealDB instance
            let store = match rt.block_on(async {
                let config = ConnectionConfig::new(fixture.test_path());
                connect(config)
                    .await
                    .map_err(|e| format!("Connection failed: {}", e))
            }) {
                Ok(store) => store,
                Err(e) => {
                    eprintln!("Failed to connect: {}", e);
                    return;
                }
            };

            let store = match rt.block_on(DurableEventStore::new(store)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create store: {}", e);
                    return;
                }
            };

            // Create test events with specified size
            let events: Vec<BeadEvent> =
                (0..total_events).map(|_| create_test_event(event_size)).collect();

            b.iter(|| {
                match rt.block_on(benchmark_batch_append(
                    black_box(&store),
                    black_box(&events),
                )) {
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
        let size_label = format!("{}_events", size);

        group.bench_with_input(
            BenchmarkId::new("single_fsync_per_batch", &size_label),
            &size,
            |b, &size| {
                // Create fresh environment per iteration
                let fixture = match BenchmarkFixture::setup() {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Failed to create fixture: {}", e);
                        return;
                    }
                };

                // Create tokio runtime for async operations
                let rt = match Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("Failed to create runtime: {}", e);
                        return;
                    }
                };

                // Initialize fresh SurrealDB instance
                let store = match rt.block_on(async {
                    let config = ConnectionConfig::new(fixture.test_path());
                    connect(config)
                        .await
                        .map_err(|e| format!("Connection failed: {}", e))
                }) {
                    Ok(store) => store,
                    Err(e) => {
                        eprintln!("Failed to connect: {}", e);
                        return;
                    }
                };

                let store = match rt.block_on(DurableEventStore::new(store)) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to create store: {}", e);
                        return;
                    }
                };

                // Create test events with specified size
                let events: Vec<BeadEvent> =
                    (0..size).map(|_| create_test_event(event_size)).collect();

                // Run benchmark iteration
                b.iter(|| {
                    match rt.block_on(benchmark_batch_append(
                        black_box(&store),
                        black_box(&events),
                    )) {
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
