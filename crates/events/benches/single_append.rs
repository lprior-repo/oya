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

/// Timing breakdown for append operation phases
#[derive(Debug, Clone, Copy)]
pub struct TimingBreakdown {
    /// Time to serialize event with bincode
    pub serialization: Duration,
    /// Time to write to WAL and insert into SurrealDB
    pub insert: Duration,
    /// Time to fsync WAL file to disk
    pub fsync: Duration,
    /// Total time for all phases
    pub total: Duration,
}

impl TimingBreakdown {
    /// Create new breakdown from component timings
    pub fn new(serialization: Duration, insert: Duration, fsync: Duration) -> Self {
        let total = serialization + insert + fsync;
        Self {
            serialization,
            insert,
            fsync,
            total,
        }
    }

    /// Format timing as microseconds for display
    pub fn fmt_micros(&self) -> String {
        format!(
            "serialize: {:>8}µs | insert: {:>8}µs | fsync: {:>8}µs | total: {:>8}µs",
            self.serialization.as_micros(),
            self.insert.as_micros(),
            self.fsync.as_micros(),
            self.total.as_micros()
        )
    }
}

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

/// Measure single append with detailed timing breakdown
///
/// Reimplements append logic to instrument each phase:
/// - Serialization time (bincode)
/// - WAL write + database insert time
/// - fsync time (file.sync_all)
///
/// Returns TimingBreakdown with all three phases measured separately
async fn measure_append_with_breakdown(
    store: &DurableEventStore,
    event: &BeadEvent,
) -> Result<TimingBreakdown, String> {
    use tokio::io::AsyncWriteExt;

    // Create WAL directory in the store's context
    let wal_dir = ".wal";
    tokio::fs::create_dir_all(wal_dir)
        .await
        .map_err(|e| format!("Failed to create wal dir: {}", e))?;

    // Phase 1: Measure serialization time
    let serialize_start = std::time::Instant::now();
    let serialized_data =
        bincode::serialize(event).map_err(|e| format!("Serialization failed: {}", e))?;
    let serialization_time = serialize_start.elapsed();

    let wal_path = std::path::PathBuf::from(wal_dir).join(format!("{}.wal", event.bead_id()));
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&wal_path)
        .await
        .map_err(|e| format!("Failed to open wal file: {}", e))?;

    let insert_start = std::time::Instant::now();

    // Write serialized event to WAL
    let length_prefix = (serialized_data.len() as u32).to_be_bytes();

    file.write_all(&length_prefix)
        .await
        .map_err(|e| format!("WAL write prefix failed: {}", e))?;

    file.write_all(&serialized_data)
        .await
        .map_err(|e| format!("WAL write data failed: {}", e))?;

    // Phase 3: Measure fsync time
    let fsync_start = std::time::Instant::now();
    file.sync_all()
        .await
        .map_err(|e| format!("fsync failed: {}", e))?;
    let fsync_time = fsync_start.elapsed();

    let insert_time = insert_start.elapsed() - fsync_time;

    // Phase 2: Measure database insert time
    let db_insert_start = std::time::Instant::now();
    store
        .append_event(event)
        .await
        .map_err(|e| format!("Database insert failed: {}", e))?;
    let db_insert_time = db_insert_start.elapsed();

    // Combine WAL insert + DB insert time
    let total_insert = insert_time + db_insert_time;

    Ok(TimingBreakdown::new(
        serialization_time,
        total_insert,
        fsync_time,
    ))
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

/// Criterion benchmark wrapper for single append with timing breakdown
fn bench_single_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_append_breakdown");

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
            BenchmarkId::new("timing_breakdown", &size_label),
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

                // First iteration: print breakdown
                let mut first_run = true;

                // Run benchmark iteration
                b.iter(|| {
                    let breakdown = match rt.block_on(measure_append_with_breakdown(
                        black_box(&store),
                        black_box(&event),
                    )) {
                        Ok(b) => b,
                        Err(e) => {
                            eprintln!("Benchmark error: {}", e);
                            return;
                        }
                    };

                    if first_run {
                        println!("\n[{}] Timing breakdown:", size_label);
                        println!("{}", breakdown.fmt_micros());
                        first_run = false;
                    }

                    // Black box the result to prevent optimization
                    black_box(breakdown);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_single_append);
criterion_main!(benches);
