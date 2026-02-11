# Contract Specification: Batch Append Benchmark

## Context

**Feature**: Benchmark batch append operations for event-sourcing system

**Bead ID**: src-ufu

**Domain Terms**:
- **Batch append**: Atomic write operation for multiple events with single fsync
- **Fsync amortization**: Single fsync per batch instead of per-event fsync
- **Throughput**: Events processed per second (events/sec)
- **WAL (Write-Ahead Log)**: Durable event log with length-prefix encoding
- **DurableEventStore**: Event store with SurrealDB backend and WAL persistence
- **Criterion**: Rust statistical benchmarking library

**Assumptions**:
1. `DurableEventStore::append_batch()` method exists (confirmed in durable_store.rs:388)
2. Method signature: `pub async fn append_batch(&self, events: &[BeadEvent]) -> Result<Vec<EventId>, AppendBatchError>`
3. Maximum batch size is 1000 events (enforced by precondition check)
4. Single fsync is performed per batch (amortization benefit)
5. SurrealDB is used as the database backend
6. Benchmark should compare against single append baseline

**Open Questions**:
- None identified - API exists and is functional

## Preconditions

- **BEFORE benchmark execution**:
  - Temporary directory exists and is writable
  - SurrealDB can be initialized at test path
  - DurableEventStore can be created successfully
  - Tokio runtime is available for async execution

- **BEFORE each benchmark iteration**:
  - Fresh DurableEventStore instance is created
  - Test events are pre-generated with known payload sizes
  - Benchmark harness has measured time baseline

## Postconditions

- **AFTER successful benchmark execution**:
  - Criterion generates statistical report (mean, std dev, percentiles)
  - Throughput measurements are calculated (events/sec)
  - Comparison data is generated (single vs batch append)
  - Fsync amortization is verified (single fsync per batch)

- **AFTER each benchmark iteration**:
  - All events in batch are persisted to WAL
  - Exactly one fsync was performed for the batch
  - SurrealDB contains all batch events
  - Event IDs are returned in same order as input

## Invariants

- **ALWAYS true during benchmark**:
  - Batch size is between 1 and 1000 (inclusive)
  - Each event has valid BeadId and payload
  - Single fsync per batch (not per event)
  - Events are processed atomically (all or nothing)
  - Event IDs are returned in input order

- **ALWAYS true for measurements**:
  - Warm-up time is 3 seconds (allows JIT optimization)
  - Measurement time is 10 seconds (statistical significance)
  - Sample size is 100 iterations (reliable distribution)
  - Black box prevents compiler optimizations

## Error Taxonomy

**Benchmark-specific errors** (non-fatal, logged):

```rust
pub enum BenchmarkError {
    /// Temp directory creation failed (e.g., disk full, permission denied)
    TempDirFailed { reason: String },

    /// SurrealDB initialization failed (e.g., port conflict, corruption)
    DatabaseInitFailed { reason: String },

    /// DurableEventStore creation failed (e.g., WAL directory error)
    StoreCreationFailed { reason: String },

    /// Tokio runtime creation failed (system resource limit)
    RuntimeCreationFailed { reason: String },

    /// Event generation failed (invalid payload size)
    EventGenerationFailed { size: usize, reason: String },
}
```

**DurableEventStore errors** (from append_batch contract):

```rust
pub enum AppendBatchError {
    /// Batch is empty (precondition violation)
    EmptyBatch,

    /// Batch exceeds maximum size (precondition violation)
    BatchTooLarge { size: usize, max: usize },

    /// Event serialization failed (data corruption)
    SerializationFailed { index: usize, event_id: String, reason: String },

    /// WAL file operation failed (I/O error)
    WalOpenFailed(String),
    WalWriteFailed(String),
    WalSyncFailed(String),
    WalCloseFailed(String),

    /// Database batch write failed (SurrealDB error)
    DatabaseWriteFailed(String),

    /// Precondition violated (logic error)
    PreconditionViolation(String),
}
```

## Contract Signatures

### Benchmark Fixture

```rust
/// RAII fixture for temporary directory management
pub struct BenchmarkFixture {
    temp_dir: TempDir,
}

impl BenchmarkFixture {
    /// Create isolated temporary directory
    /// Returns error if tempfile creation fails
    pub fn setup() -> Result<Self, BenchmarkError>;
}
```

### Event Generation

```rust
/// Generate test event with specified payload size
/// Returns BeadEvent::Created with sized description
pub fn create_test_event(size_bytes: usize) -> BeadEvent;
```

### Benchmark Functions

```rust
/// Measure batch append latency and throughput
/// Returns duration and events per second
async fn benchmark_batch_append(
    store: &DurableEventStore,
    events: &[BeadEvent],
) -> Result<Duration, BenchmarkError>;

/// Baseline: measure single append for comparison
/// Returns duration for single event
async fn benchmark_single_append(
    store: &DurableEventStore,
    event: &BeadEvent,
) -> Result<Duration, BenchmarkError>;
```

### Criterion Wrappers

```rust
/// Batch append throughput by batch size
fn bench_batch_append_throughput(c: &mut Criterion);

/// Single vs batch append comparison
fn bench_single_vs_batch_comparison(c: &mut Criterion);

/// Fsync amortization verification
fn bench_fsync_amortization(c: &mut Criterion);
```

## Performance Targets

**Success Criteria** (from bead description):

1. **Throughput Improvement**: Batch append achieves 10x+ throughput vs single append
   - Single append baseline: ~1000 events/sec (1ms per event)
   - Batch append target: ~10,000 events/sec (0.1ms per event amortized)

2. **Fsync Amortization**: Exactly ONE fsync per batch
   - Single append: N fsyncs for N events
   - Batch append: 1 fsync for N events

3. **Test Coverage**: Batch sizes tested
   - Small: 1, 10 events
   - Medium: 50, 100 events
   - Large: 500, 1000 events

4. **Payload Size**: 1KB per event (realistic size)
   - Total batch size: 1KB to 1MB

## Non-goals

- **NOT implementing append_batch()** - method already exists
- **NOT modifying DurableEventStore** - benchmark only
- **NOT testing correctness** - functional tests cover that
- **NOT optimizing SurrealDB** - out of scope
- **NOT testing concurrent batch appends** - single-threaded benchmark only
- **NOT testing failure modes** - happy path benchmark only

## File Output

**Generated file**: `crates/events/benches/batch_append.rs`

**Structure**:
- BenchmarkFixture setup
- create_test_event helper
- benchmark_batch_append function
- benchmark_single_append baseline
- Criterion benchmark groups (3 benchmarks)
- criterion_group! and criterion_main! macros

**Dependencies**:
- `criterion` = "0.5" (benchmarking)
- `tempfile` = "3.10" (temp directories)
- `tokio` = { features = ["rt", "fs", "io-util"] }
- `oya_events` (local crate)

**Lint configuration**:
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
```

## Verification Checklist

- [ ] Benchmark compiles without errors
- [ ] Benchmark runs without panics
- [ ] Criterion generates report with all three groups
- [ ] Throughput is calculated (events/sec)
- [ ] Single vs batch comparison shows 10x+ improvement
- [ ] Fsync amortization is verified (1 fsync per batch)
- [ ] All batch sizes are tested (1, 10, 50, 100, 500, 1000)
- [ ] Statistical significance is achieved (10s measurement, 100 samples)
- [ ] Zero unwrap/expect/panic violations
