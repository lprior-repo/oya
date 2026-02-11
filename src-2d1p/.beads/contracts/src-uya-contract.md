# Contract Specification: fsync Overhead Benchmarks

## Context
- **Feature**: Benchmark fsync write latency to verify 2-3ms overhead is acceptable
- **Domain Terms**:
  - **fsync**: System call that synchronizes a file's in-core state with storage device
  - **Event append**: Write operation to persist a BeadEvent to durable storage
  - **p50/p95/p99**: Percentiles measuring latency distribution (50th, 95th, 99th percentile)
  - **Criterion**: Rust statistical benchmarking framework that provides confidence intervals
  - **WAL (Write-Ahead Log)**: Append-only log for durability before database write
- **Assumptions**:
  - DurableEventStore already implements `append_event()` with WAL fsync (line 303 in durable_store.rs)
  - BeadEvent serialization via bincode produces events < 1KB (enforced in event.rs:515-525)
  - Storage backend is local RocksDB via SurrealDB
  - Filesystem is ext4/xfs with typical fsync latency of 1-10ms
  - Benchmarks run on single-core, single-threaded (no concurrency)
- **Open Questions**:
  - None - context is clear from existing DurableEventStore implementation

## Preconditions

### Environment Setup
- [P1] Storage path must be writable: `test_db_path` directory must exist or be creatable
- [P2] Disk space must be available: At least 100MB free for benchmark artifacts
- [P3] Criterion library must be available: Dev-dependency `criterion = "0.5"` in Cargo.toml
- [P4] System must be idle: No other I/O-intensive processes running during benchmarks

### Database State
- [P5] Fresh SurrealDB instance: Each benchmark uses unique storage path to avoid interference
- [P6] No existing events: Database starts empty for each benchmark iteration

### Event Data
- [P7] Valid BeadEvent instances: Events must be constructible via `BeadEvent::created()`, etc.
- [P8] Event size within limits: Serialized event < 1KB (enforced by contract in event.rs)

## Postconditions

### Benchmark Execution
- [PO1] Benchmarks complete successfully: All criterion iterations finish without error
- [PO2] Statistical significance achieved: Criterion reports confidence intervals (default: >100 samples)
- [PO3] Results are reproducible: Re-running benchmarks yields similar results (±10%)

### Performance Targets
- [PO4] Single event append with fsync: p99 latency < 3ms
- [PO5] Single event append without fsync: p99 latency < 0.5ms
- [PO6] Batch append (100 events): Average throughput > 100 events/sec with fsync
- [PO7] Read 1000 events: Total time < 50ms (no fsync involved, baseline performance)
- [PO8] Replay 1000 events: Total time < 5s (includes database query overhead)

### Output Artifacts
- [PO9] Benchmark report generated: `target/criterion/fsync_overhead/report/index.html`
- [PO10] JSON data exported: `target/criterion/fsync_overhead/benchmark.json` for CI integration
- [PO11] Console output includes: p50/p95/p99 latencies for each benchmark

## Invariants

### During Benchmark Execution
- [I1] File handle management: Each benchmark opens/closes file properly (no resource leaks)
- [I2] Write ordering: fsync happens AFTER data written to WAL (durable_store.rs:295-305)
- [I3] Event consistency: Serialized event roundtrips correctly (bincode encode/decode)

### Statistical Validity
- [I4] Sample size: Criterion collects ≥100 samples per benchmark (configurable)
- [I5] Warmup period: Criterion discards first 3 iterations as warmup (default behavior)
- [I6] Outlier handling: Criterion uses robust statistical methods (not raw mean)

### System State
- [I7] No database corruption: SurrealDB remains consistent after benchmarks
- [I8] No file descriptors leaked: All WAL files closed properly
- [I9] Temp files cleaned up: Test databases removed after benchmark completion

## Error Taxonomy

### BenchmarkSetup Errors
- **BenchmarkError::StoragePathNotWritable** - when test directory cannot be created
- **BenchmarkError::InsufficientDiskSpace** - when <100MB free space available
- **BenchmarkError::DatabaseInitializationFailed** - when SurrealDB fails to start

### BenchmarkExecution Errors
- **BenchmarkError::EventSerializationFailed** - when BeadEvent cannot be serialized
- **BenchmarkError::AppendTimeout** - when single append takes >10s (hanging benchmark)
- **BenchmarkError::InsufficientSamples** - when criterion collects <100 samples
- **BenchmarkError::PerformanceTargetMissed** - when p99 latency exceeds target

### SystemEnvironment Errors
- **BenchmarkError::SystemNotIdle** - when background I/O detected during benchmark
- **BenchmarkError::FilesystemSyncBroken** - when fsync() returns error (e.g., NFS)
- **BenchmarkError::CrateNotAvailable** - when criterion dependency missing

## Contract Signatures

### Benchmark Functions
```rust
/// Benchmark single event append WITH fsync (durable write)
/// Measures: p50/p95/p99 latency of append_event() including WAL fsync
/// Target: p99 < 3ms
fn bench_append_with_fsync(c: &mut Criterion, event: &BeadEvent) -> BenchmarkResult;

/// Benchmark single event append WITHOUT fsync (write-only, no durability)
/// Measures: Baseline p50/p95/p99 latency without fsync overhead
/// Target: p99 < 0.5ms
fn bench_append_without_fsync(c: &mut Criterion, event: &BeadEvent) -> BenchmarkResult;

/// Benchmark batch append of multiple events
/// Measures: Throughput (events/sec) when appending 10/100/1000 events
/// Target: >100 events/sec average
fn bench_batch_append(c: &mut Criterion, events: &[BeadEvent]) -> BenchmarkResult;

/// Benchmark read performance (no fsync, baseline query speed)
/// Measures: Time to read 1000 events from database
/// Target: <50ms total
fn bench_read_events(c: &mut Criterion, event_count: usize) -> BenchmarkResult;

/// Benchmark replay from checkpoint
/// Measures: Time to replay 1000 events from checkpoint event_id
/// Target: <5s total
fn bench_replay_from_checkpoint(c: &mut Criterion, event_count: usize) -> BenchmarkResult;
```

### Helper Functions
```rust
/// Setup fresh SurrealDB instance for benchmarking
/// Returns: DurableEventStore with unique storage path
/// Errors: BenchmarkError::DatabaseInitializationFailed
fn setup_benchmark_db() -> Result<Arc<DurableEventStore>, BenchmarkError>;

/// Generate test event sequence for benchmarking
/// Returns: Vec<BeadEvent> with deterministic content
fn generate_test_events(count: usize) -> Result<Vec<BeadEvent>, BenchmarkError>;

/// Verify benchmark results meet performance targets
/// Returns: Ok(()) if targets met, Err otherwise
fn verify_performance_targets(results: &BenchmarkResult) -> Result<(), BenchmarkError>;
```

### Result Type
```rust
type BenchmarkResult = Result<Stats, BenchmarkError>;

struct Stats {
    p50: Duration,  // 50th percentile latency
    p95: Duration,  // 95th percentile latency
    p99: Duration,  // 99th percentile latency
    mean: Duration, // Mean latency
    stddev: Duration, // Standard deviation
    samples: usize,  // Number of samples collected
}

enum BenchmarkError {
    StoragePathNotWritable { path: PathBuf, io_error: String },
    InsufficientDiskSpace { available_mb: u64, required_mb: u64 },
    DatabaseInitializationFailed { reason: String },
    EventSerializationFailed { event_type: String, error: String },
    AppendTimeout { duration_secs: u64 },
    InsufficientSamples { collected: usize, required: usize },
    PerformanceTargetMissed { metric: String, target: String, actual: String },
    SystemNotIdle { detected_processes: Vec<String> },
    FilesystemSyncBroken { syscall_error: String },
    CrateNotAvailable { dependency: String },
}
```

## Non-goals

- [NG1] NOT benchmarking concurrent appends (single-threaded only)
- [NG2] NOT benchmarking network storage (local filesystem only)
- [NG3] NOT benchmarking different filesystem types (ext4 assumed)
- [NG4] NOT benchmarking database read-modify-write operations (append-only workload)
- [NG5] NOT tuning RocksDB configuration (use defaults from DurableEventStore)
- [NG6] NOT implementing fsync batching strategies (single fsync per append)
- [NG7] NOT measuring memory allocation overhead (focus on I/O latency)
- [NG8] NOT benchmarking different event sizes (fixed <1KB events only)
