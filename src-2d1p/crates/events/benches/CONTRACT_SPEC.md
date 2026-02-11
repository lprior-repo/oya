# Contract Specification: Fsync Overhead Benchmarking

## Context
- **Feature**: Event sourcing performance benchmarking with fsync overhead measurement
- **Bead ID**: src-uya
- **Domain Terms**:
  - **fsync**: System call that flushes file system buffers to durable storage
  - **WAL (Write-Ahead Log)**: Append-only log for event persistence before database commit
  - **p50/p95/p99**: Percentile latencies (50th, 95th, 99th percentile)
  - **Criterion**: Rust statistical benchmarking library with confidence intervals
  - **Sync operation**: Tokio's `file.sync_all()` which invokes fsync
  - **Append latency**: Time from `append_event()` call to durable persistence

- **Assumptions**:
  1. Benchmark runs on local filesystem (ext4/xfs), not network-mounted storage
  2. Criterion's default measurement time (5s) provides sufficient samples
  3. Tokio runtime with `rt-multi-thread` for concurrent operations
  4. Filesystem supports async I/O operations
  5. `DurableEventStore.append_event()` is the primary persistence API
  6. Existing `durable_store.rs` uses `file.sync_all()` for fsync

- **Open Questions**:
  1. Q: Should benchmarks run against actual SurrealDB or mock only the fsync layer?
     - **A**: Mock only the WAL layer to isolate fsync overhead (database adds noise)
  2. Q: Should we benchmark batch appends (multiple events in one fsync)?
     - **A**: Yes, include batch sizes of 1, 10, 100 events
  3. Q: Should benchmarks be CI-blocking or informational?
     - **A**: Informational only (filesystem-dependent), but alert if p99 > 3ms threshold

## Preconditions

### Environment Preconditions
- Filesystem with write permissions (e.g., `/tmp` or tempfile)
- Sufficient disk space for WAL files (>100MB)
- Linux system with `fsync()` syscall support (not WSL or macOS)
- CPU not under heavy load (<80% utilization)
- Disk I/O not saturated (check via `iostat`)

### Code Preconditions
- `DurableEventStore` is properly initialized with temp directory
- `BeadEvent` can be constructed for benchmarking
- Criterion harness is configured with `harness = false` in Cargo.toml
- `[dev-dependencies]` includes `criterion = "0.5"`

### Data Preconditions
- Events are properly constructed with valid `BeadId` and `EventId`
- Event serialization size <1KB (per existing constraint)
- WAL directory exists and is writable

## Postconditions

### Measurement Postconditions
- All latency measurements include fsync overhead
- Percentiles (p50/p95/p99) are calculated with 95% confidence intervals
- Benchmark results are saved to `target/criterion/` HTML reports
- Sample count >100 per benchmark (statistical significance)

### Assertions Postconditions
- **With fsync**: p99 append latency <3ms (passes if below threshold)
- **Without fsync**: p99 append latency <0.5ms (baseline)
- **Read 1000 events**: Completes in <50ms
- **Replay 1000 events**: Completes in <5s

### Cleanup Postconditions
- All temporary WAL files are deleted after benchmark
- No file descriptors leaked
- Tokio runtime is properly shut down

## Invariants

### Filesystem Invariants
- WAL files are created in designated temp directory
- Each append operation writes length-prefixed binary data
- fsync is called **after** write completes, not before
- File position advances monotonically (append-only)

### Performance Invariants
- fsync overhead is consistently measurable (non-zero)
- Batch appends amortize fsync cost (cost per event decreases)
- Read operations are faster than write operations (no fsync)
- Replay is O(n) where n = number of events

### Statistical Invariants
- Benchmark results are reproducible (±10% variance across runs)
- Criterion's warm-up period stabilizes measurements
- No outliers >3 standard deviations from mean

## Error Taxonomy

### Benchmark Initialization Errors
- **BenchmarkError::TempDirFailed** - Tempfile creation fails (permissions, disk full)
- **BenchmarkError::RuntimeInitFailed** - Tokio runtime cannot be created
- **BenchmarkError::StoreInitFailed** - `DurableEventStore::new()` fails

### Measurement Errors
- **BenchmarkError::FsyncVerificationFailed** - strace shows no fsync syscall
- **BenchmarkError::InsufficientSamples** - Criterion collected <100 samples
- **BenchmarkError::StatisticalSignificanceFailed** - Confidence interval >10% of mean

### Assertion Errors
- **BenchmarkError::FsyncLatencyTooHigh** - p99 >3ms with fsync
- **BenchmarkError::BaselineTooSlow** - p99 >0.5ms without fsync (system issue)
- **BenchmarkError::ReadLatencyTooHigh** - Read 1000 events >50ms
- **BenchmarkError::ReplayLatencyTooHigh** - Replay 1000 events >5s

### Cleanup Errors
- **BenchmarkError::WalCleanupFailed** - Failed to delete WAL files
- **BenchmarkError::FileDescriptorLeak** - lsof shows unclosed files after benchmark

## Contract Signatures

### Benchmark Setup
```rust
/// Creates a temporary DurableEventStore for benchmarking
/// Returns error if temp directory cannot be created or store init fails
fn setup_benchmark_store() -> Result<DurableEventStore, BenchmarkError>

/// Verifies fsync is being called via strace
/// Returns Ok(true) if fsync is present, Ok(false) if not, Err on strace failure
fn verify_fsync_enabled(store: &DurableEventStore) -> Result<bool, BenchmarkError>
```

### Benchmark Functions
```rust
/// Benchmarks single event append with fsync
/// Measures p50/p95/p99 latency, asserts p99 <3ms
fn benchmark_append_with_fsync(c: &mut Criterion)

/// Benchmarks single event append without fsync (baseline)
/// Measures p50/p95/p99 latency, asserts p99 <0.5ms
fn benchmark_append_without_fsync(c: &mut Criterion)

/// Benchmarks batch appends with varying batch sizes
/// Tests throughput with 1, 10, 100 events per batch
fn benchmark_batch_append(c: &mut Criterion)

/// Benchmarks reading 1000 events from database
/// Measures query performance, asserts <50ms
fn benchmark_read_events(c: &mut Criterion)

/// Benchmarks replaying 1000 events from checkpoint
/// Measures recovery time, asserts <5s
fn benchmark_replay_from_checkpoint(c: &mut Criterion)
```

### Metrics Collection
```rust
/// Collects latency metrics from Criterion benchmark
/// Returns structured p50/p95/p99 with confidence intervals
fn collect_metrics(benchmark_id: &str) -> Result<LatencyMetrics, BenchmarkError>

/// Compares fsync vs baseline overhead
/// Returns percentage increase and absolute difference
fn calculate_fsync_overhead(with_fsync: &LatencyMetrics, without_fsync: &LatencyMetrics)
    -> Result<FsyncOverhead, BenchmarkError>
```

## Non-goals

- **NOT** benchmarking SurrealDB query performance (isolated to fsync layer)
- **NOT** optimizing fsync behavior (measurement only, no code changes)
- **NOT** setting up production monitoring (benchmarks are ad-hoc)
- **NOT** testing persistence guarantees (correctness is unit test responsibility)
- **NOT** benchmarking network storage (assumes local filesystem only)
- **NOT** comparing filesystem types (ext4 vs xfs vs btrfs out of scope)
- **NOT** profiling CPU usage (use perf separately if needed)
