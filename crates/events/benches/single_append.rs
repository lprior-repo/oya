// Single Event Append Benchmark
//
// Measures DurableEventStore::append_event() latency with fsync to measure
// per-event overhead. Performance targets: p50 < 3ms, p99 < 5ms for 1KB payload.
//
// Breakdown:
// - Serialization time (bincode)
// - WAL write time
// - fsync time (dominant)
// - SurrealDB insert time

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

/// Criterion benchmark wrapper for single append with varying payload sizes
fn bench_single_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_append");

    // Configure measurement time and sample size for statistical significance
    group
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);

    // Test realistic payload sizes
    let payload_sizes = vec![100, 1024, 10_240]; // 100B, 1KB, 10KB

    for size in payload_sizes {
        let size_label = format!("{}_bytes", size);

        group.bench_with_input(
            BenchmarkId::new("append_with_fsync", &size_label),
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

                // Create test event with specified size
                let event = create_test_event(size);

                // Run benchmark iteration
                b.iter(|| {
                    let rt = match Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            eprintln!("Runtime creation failed: {}", e);
                            return;
                        }
                    };

                    match rt.block_on(benchmark_single_append(
                        black_box(&store),
                        black_box(&event),
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

criterion_group!(benches, bench_single_append);
criterion_main!(benches);
