# Martin Fowler Test Plan: fsync Overhead Benchmarks

## Happy Path Tests

### test_append_with_fsync_confirms_p99_latency_under_3ms
**Given**: Fresh SurrealDB instance with writable storage path
**When**: Benchmark single event append with fsync for 100+ iterations
**Then**:
- Benchmark completes without errors
- Criterion reports p99 latency < 3ms
- p50 latency is significantly lower than p99 (shows distribution tail)
- Confidence interval is tight (stddev < mean × 0.5)
- Benchmark report is generated in `target/criterion/`

### test_append_without_fsync_confirms_baseline_performance
**Given**: Fresh SurrealDB instance with writable storage path
**When**: Benchmark single event append WITHOUT fsync for 100+ iterations
**Then**:
- Benchmark completes without errors
- Criterion reports p99 latency < 0.5ms (10x faster than with fsync)
- fsync overhead is measurable (with_fsync / without_fsync ratio > 5x)
- Results are reproducible across multiple runs (±10%)

### test_batch_append_confirms_throughput_target
**Given**: Fresh SurrealDB instance and 100 pre-generated BeadEvents
**When**: Benchmark batch append of 100 events with fsync
**Then**:
- All 100 events are appended successfully
- Average throughput > 100 events/sec
- Total time < 1s for 100 events
- Each event has unique event_id (no duplicates)

### test_read_events_confirms_baseline_query_performance
**Given**: SurrealDB instance with 1000 pre-appended events
**When**: Benchmark read_events() to retrieve all 1000 events
**Then**:
- All 1000 events are retrieved successfully
- Total read time < 50ms (no fsync involved)
- Retrieved events match appended events (deterministic)
- Events are ordered by timestamp (OLDEST to NEWEST)

### test_replay_from_checkpoint_completes_under_5_seconds
**Given**: SurrealDB instance with 1000 events and checkpoint at event 500
**When**: Benchmark replay_from() to retrieve events 501-1000
**Then**:
- Exactly 500 events are replayed (events 501-1000)
- Total replay time < 5s
- Replay is deterministic (same events in same order as direct read)
- Checkpoint event itself is excluded from replay results

### test_benchmark_report_is_generated_and_valid
**Given**: All benchmarks have completed successfully
**When**: Check file system for benchmark artifacts
**Then**:
- `target/criterion/fsync_overhead/report/index.html` exists
- HTML file is valid and viewable in browser
- `target/criterion/fsync_overhead/benchmark.json` exists
- JSON contains all required fields: mean, stddev, p50, p95, p99

## Error Path Tests

### test_append_with_fsync_returns_error_when_storage_not_writable
**Given**: Storage path points to read-only directory (e.g., `/root/oya-test`)
**When**: Benchmark attempts to append event with fsync
**Then**:
- Benchmark returns `BenchmarkError::StoragePathNotWritable`
- Error message includes the path and underlying OS error
- No files are created (fail-fast before database init)

### test_append_returns_error_when_disk_space_insufficient
**Given**: Storage path has <10MB free space (simulate with small partition)
**When**: Benchmark attempts to append 1000 events
**Then**:
- Benchmark returns `BenchmarkError::InsufficientDiskSpace`
- Error message includes available and required space
- Database is cleaned up (no partial state left)

### test_append_timeout_hangs_when_fsync_broken
**Given**: Filesystem with broken fsync (e.g., NFS mount with stale handle)
**When**: Benchmark attempts to append event with fsync
**Then**:
- Benchmark times out after 10s (configurable)
- Returns `BenchmarkError::AppendTimeout`
- Error message includes duration waited

### test_replay_returns_error_when_checkpoint_nonexistent
**Given**: SurrealDB instance with events
**When**: Benchmark attempts to replay from fake checkpoint_id
**Then**:
- Benchmark returns `DatabaseError::NotFound` (from existing contract)
- No events are returned
- Benchmark completes quickly (<1s, no actual replay work)

### test_benchmark_fails_when_criterion_dependency_missing
**Given**: Cargo.toml without criterion dev-dependency
**When**: Attempt to compile benchmark binary
**Then**:
- Compilation fails with "cannot find criterion crate"
- Error message is clear: add `criterion = "0.5"` to dev-dependencies

## Edge Case Tests

### test_append_with_empty_event_data
**Given**: BeadEvent with minimal/empty payload (Created event with empty spec)
**When**: Benchmark append with fsync
**Then**:
- Append succeeds (empty events are valid)
- Latency is similar to normal-sized events (fsync dominates)
- Event can be read back correctly

### test_append_with_maximum_size_event
**Given**: BeadEvent with serialized size approaching 1KB limit
**When**: Benchmark append with fsync
**Then**:
- Append succeeds (size < 1KB enforced by contract)
- Latency is within 2x of median event (linear in size)
- No `SerializationError::SizeExceeded` is raised

### test_batch_append_with_single_event
**Given**: Batch size of 1 event
**When**: Benchmark batch append
**Then**:
- Behaves identically to single event append
- Throughput ~1 event/sec (fsync dominates)
- No special case handling for size=1

### test_read_events_from_empty_database
**Given**: Fresh SurrealDB instance with no events
**When**: Benchmark read_events() for any bead_id
**Then**:
- Returns empty Vec (not an error)
- Completes instantly (<1ms)
- No database errors raised

### test_replay_from_checkpoint_at_first_event
**Given**: Database with 100 events, checkpoint at event 1
**When**: Benchmark replay_from(first_event_id)
**Then**:
- Returns 99 events (events 2-100)
- Completes quickly (small result set)
- Checkpoint event excluded from results

### test_replay_from_checkpoint_at_last_event
**Given**: Database with 100 events, checkpoint at event 100
**When**: Benchmark replay_from(last_event_id)
**Then**:
- Returns 0 events (nothing after last event)
- Completes instantly
- No errors raised

### test_benchmark_with_single_sample
**Given**: Criterion misconfigured to collect only 1 sample
**When**: Benchmark executes
**Then**:
- Benchmark returns `BenchmarkError::InsufficientSamples`
- Error message includes collected=1, required=100
- Results are not reported (statistically invalid)

### test_benchmark_with_high_variance_workload
**Given**: System with periodic background I/O (simulate with cron job)
**When**: Benchmark append with fsync
**Then**:
- Benchmark detects high variance (stddev > mean)
- Returns `BenchmarkError::SystemNotIdle`
- Suggests re-running when system is idle

## Contract Verification Tests

### test_precondition_storage_path_writable
**Given**: Benchmark setup code
**When**: Call setup_benchmark_db() with read-only path
**Then**:
- Returns `BenchmarkError::StoragePathNotWritable`
- Storage path is validated before database initialization
- No partial database state created

### test_precondition_fresh_database_per_iteration
**Given**: Criterion benchmark loop (executes same function 100x)
**When**: Each iteration calls setup_benchmark_db()
**Then**:
- Each iteration gets unique storage path (via ulid::Ulid::new())
- Databases do not interfere with each other
- No lock contention or database corruption

### test_postcondition_performance_targets_documented
**Given**: Benchmark results
**When**: Call verify_performance_targets(results)
**Then**:
- Returns Ok(()) if all targets met (p99 < 3ms, etc.)
- Returns Err if any target missed with detailed comparison
- Error message includes: metric name, target value, actual value

### test_postcondition_benchmark_report_generated
**Given**: Successful benchmark execution
**When**: Check filesystem for artifacts
**Then**:
- Report HTML exists at expected path
- JSON data exists and is parseable
- All benchmarks (append, read, replay) have reports

### test_invariant_event_roundtrip_consistency
**Given**: BeadEvent instance
**When**: Serialize to bincode, append to database, read back, deserialize
**Then**:
- Deserialized event matches original exactly
- event_id, bead_id, timestamp are identical
- All event fields match (no data corruption)

### test_invariant_file_handle_cleanup
**Given**: Benchmark appending 1000 events
**When**: Benchmark completes and drops DurableEventStore
**Then**:
- All WAL file descriptors are closed
- No "too many open files" error
- Temp directory can be deleted (no locked files)

### test_invariant_statistical_significance
**Given**: Criterion benchmark configuration
**When**: Benchmark executes
**Then**:
- At least 100 samples collected (default criterion setting)
- Warmup period of 3 iterations completed
- Confidence intervals reported in output

## Given-When-Then Scenarios

### Scenario 1: Developer runs benchmarks locally before committing
**Given**: Developer has modified DurableEventStore append logic
**And**: Wants to verify performance regression hasn't been introduced
**When**: Developer runs `cargo bench --bench fsync_overhead`
**Then**:
- All benchmarks complete in <2 minutes total
- Console output shows p99 latencies for each scenario
- HTML report is generated for visual inspection
- Performance targets are met (green checkmark)
- Developer can commit with confidence

### Scenario 2: CI runs benchmarks and enforces performance regression checks
**Given**: CI pipeline running on GitHub Actions
**And**: Benchmarks must not regress >20% from baseline
**When**: CI executes `cargo bench --bench fsync_overhead`
**Then**:
- Benchmarks run in isolated environment (no background processes)
- JSON results are compared against stored baseline
- If p99 latency regressed >20%, CI fails with error
- Artifact upload includes benchmark report for review

### Scenario 3: Developer suspects fsync is too slow on NFS-mounted home directory
**Given**: Developer's home directory is NFS-mounted (slow fsync)
**And**: Benchmarks show p99 = 15ms (5x slower than target)
**When**: Developer runs benchmarks with custom storage path on local SSD
**Then**:
- Command: `cargo bench --bench fsync_overhead -- --storage-path /tmp/oya-bench`
- Benchmarks use local SSD (fast fsync)
- p99 latency < 3ms (target met)
- Developer learns to avoid NFS for production database

### Scenario 4: Investigating high variance in benchmark results
**Given**: Benchmarks show p99 = 2ms but stddev = 5ms (high variance)
**And**: System may have background I/O interference
**When**: Developer runs benchmarks with system monitoring
**Then**:
- Command: `iostat -x 1` in parallel with `cargo bench`
- High disk utilization correlates with high-latency iterations
- Developer closes web browser/IDE sync and retries
- Second run shows p99 = 1.5ms, stddev = 0.3ms (consistent)

### Scenario 5: Regression test for new batching optimization
**Given**: Team wants to implement fsync batching (10 events per fsync)
**And**: Current baseline: 100 events take 10s with individual fsyncs
**When**: Developer implements batching and runs benchmarks
**Then**:
- New benchmark: `bench_batch_append_with_fsync_batching`
- Throughput improves: 100 events in 2s (5x faster)
- Benchmark report shows side-by-side comparison
- Team decides to merge optimization (meets safety targets)
