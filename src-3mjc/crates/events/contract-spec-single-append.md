# Contract Specification: Single Event Append Benchmark

```jsonl
{"kind":"contract","skill":"rust-contract","version":"1.0.0","date":"2026-02-09","bead_id":"src-3k9"}
```

## Context

- **Feature**: Benchmark single event append with fsync to measure per-event overhead
- **Domain Terms**:
  - `DurableEventStore`: Persistent event store with Write-Ahead Log (WAL)
  - `BeadEvent`: Domain event representing state transitions
  - `fsync`: System call to flush writes to durable storage
  - `latency percentiles`: Statistical measures (p50, p90, p95, p99) for response time distribution
  - `serialization`: Converting event structs to binary format using bincode
  - `SurrealDB`: Embedded database (RocksDB backend) for persistent storage

- **Assumptions**:
  - Fresh SurrealDB instance per benchmark run (no cache warming)
  - Realistic event payloads of ~1KB typical size
  - Benchmark runs in isolated temporary directory (via tempfile crate)
  - Criterion library handles statistical significance and warm-up
  - Single-threaded append operations (no concurrent writes)

- **Open Questions**:
  - Should benchmark measure cold start vs warm database separately?
  - Should we include WAL-only path (without SurrealDB insert)?
  - What constitutes acceptable variance in measurements? (Assume: <10% std dev)

## Preconditions

- **Database Connection**:
  - SurrealDB instance is initialized with RocksDB storage
  - Namespace and database are selected (`use_ns`, `use_db` succeeded)
  - Connection is valid and not exhausted

- **File System**:
  - WAL directory exists or is creatable
  - Sufficient disk space available (>10MB for test data)
  - Write permissions granted for target directory

- **Event Data**:
  - Event payload size is within realistic bounds (100B - 10KB)
  - Event has valid BeadId and EventId
  - Event serializes successfully to bincode format

- **Benchmark Infrastructure**:
  - Criterion runtime is configured
  - Tokio runtime is available for async execution
  - Temporary directory cleanup is guaranteed

## Postconditions

- **On Success**:
  - Event is written to WAL file with length-prefix encoding
  - WAL file is fsync'd to durable storage
  - Event record is inserted into SurrealDB `state_transition` table
  - Function returns `Ok(())`
  - Measured latency includes: serialize + write + fsync + database insert

- **On Failure**:
  - No partial state is persisted (atomicity)
  - WAL file is closed cleanly if open
  - Error variant indicates failure mode (serialization, write, sync, or database)
  - Temporary files are cleaned up

- **Measurement Integrity**:
  - Timing starts before first operation
  - Timing ends after fsync completes
  - Black box prevents compiler optimization from removing operations
  - Each iteration uses independent event instance (no cache)

## Invariants

- **Write-Ahead Logging**:
  - WAL append always completes before database insert
  - fsync is called on WAL file before SurrealDB create
  - WAL file format: length-prefix (u32 big-endian) + bincode serialized event

- **Event Ordering**:
  - Each event has unique EventId (ULID, time-ordered)
  - Timestamps are monotonically increasing for same bead
  - WAL appends are sequential per bead_id

- **Idempotency**:
  - Duplicate appends with same event_id result in SurrealDB error (unique key)
  - WAL may contain duplicates if process crashes after fsync but before DB insert
  - Replay logic handles duplicate detection

- **Resource Management**:
  - File handles are closed even if errors occur
  - Temporary directories are cleaned up after benchmark
  - Database connections are released

## Error Taxonomy

- `AppendError::WalOpenFailed(String)`:
  - **When**: WAL directory cannot be created or opened
  - **Why**: Permission denied, disk full, invalid path
  - **Recovery**: Check path permissions and disk space

- `AppendError::WalWriteFailed(String)`:
  - **When**: Write to WAL file fails
  - **Why**: Disk full, I/O error, file system read-only
  - **Recovery**: Check disk space and file system health

- `AppendError::WalSyncFailed(String)`:
  - **When**: fsync system call fails
  - **Why**: Hardware error, file system doesn't support fsync
  - **Recovery**: Check file system type and disk health

- `AppendError::SerializationFailed(String)`:
  - **When**: bincode serialization fails
  - **Why**: Event contains non-serializable data (e.g., Rc, Arc)
  - **Recovery**: Verify event structure matches Serialize/Deserialize derives

- `AppendError::DatabaseWriteFailed(String)`:
  - **When**: SurrealDB create operation fails
  - **Why**: Connection lost, unique constraint violation, database error
  - **Recovery**: Check database connection and logs

## Contract Signatures

```rust
/// Benchmark fixture for isolated test environment
pub struct BenchmarkFixture {
    temp_dir: TempDir,
}

impl BenchmarkFixture {
    /// Create isolated temporary directory for benchmark run
    /// Returns error if tempfile creation fails
    pub fn setup() -> Result<Self, String>;

    /// Get path to test database/storage
    pub fn test_path(&self) -> PathBuf;
}

/// Core benchmark function: measure single event append latency
/// Breaks down into: serialize time + write time + fsync time + db insert time
/// Target: p50 < 3ms, p99 < 5ms for 1KB payload
pub async fn benchmark_single_append(
    store: &DurableEventStore,
    event: &BeadEvent,
) -> Result<Duration, AppendError>;

/// Create realistic test event with specified payload size
pub fn create_test_event(size_bytes: usize) -> BeadEvent;

/// Criterion benchmark wrapper
fn bench_single_append(c: &mut Criterion) {
    // Configures warm_up_time, measurement_time, sample_size
    // Runs benchmarks for different payload sizes
    // Outputs latency percentiles (p50, p90, p95, p99)
}
```

## Non-goals

- **NOT** measuring concurrent append throughput (separate benchmark)
- **NOT** measuring batch append performance (separate benchmark)
- **NOT** testing event replay/read performance (separate benchmark)
- **NOT** validating correctness of append logic (unit test responsibility)
- **NOT** measuring memory usage (separate benchmark)
- **NOT** testing crash recovery (separate benchmark)
- **NOT** optimizing for specific storage hardware (benchmark should be hardware-agnostic)
- **NOT** implementing sharding or partitioning strategies
- **NOT** testing cross-process synchronization
