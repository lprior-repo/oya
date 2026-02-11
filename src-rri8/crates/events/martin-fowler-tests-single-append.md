# Martin Fowler Test Plan: Single Event Append Benchmark

```jsonl
{"kind":"test_plan","skill":"rust-contract","version":"1.0.0","date":"2026-02-09","bead_id":"src-3k9","framework":"criterion"}
```

## Happy Path Tests

- `test_single_append_with_1kb_payload_meets_latency_target`
  - **Given**: A fresh DurableEventStore and a 1KB BeadEvent
  - **When**: Event is appended with fsync
  - **Then**:
    - p50 latency < 3ms
    - p99 latency < 5ms
    - Event is persisted in WAL
    - Event is queryable from SurrealDB

- `test_single_append_latency_includes_all_operations`
  - **Given**: DurableEventStore and test event
  - **When**: Measuring append latency
  - **Then**:
    - Serialization time is included
    - WAL write time is included
    - fsync time is included
    - Database insert time is included
    - Total matches sum of components

- `test_benchmark_handles_multiple_payload_sizes`
  - **Given**: DurableEventStore and events of varying sizes (100B, 1KB, 10KB)
  - **When**: Running benchmark for each size
  - **Then**:
    - Latency scales linearly with payload size
    - All measurements meet p99 < 5ms target
    - Criterion reports separate statistics per size

- `test_benchmark_provides_percentile_breakdown`
  - **Given**: Completed benchmark run with 100 samples
  - **When**: Analyzing Criterion output
  - **Then**:
    - p50 (median) reported
    - p90 reported
    - p95 reported
    - p99 reported
    - Standard deviation calculated
    - Sample size >= 100

- `test_fresh_database_per_benchmark_iteration`
  - **Given**: Benchmark configured with Criterion
  - **When**: Running multiple iterations
  - **Then**:
    - Each iteration uses isolated temp directory
    - SurrealDB instance is fresh (no cache from previous runs)
    - WAL files are cleaned up between iterations
    - No data leakage between samples

## Error Path Tests

- `test_returns_wal_open_failed_when_directory_not_creatable`
  - **Given**: DurableEventStore with WAL path in /root (no permission)
  - **When**: Attempting to append event
  - **Then**:
    - Returns `Err(AppendError::WalOpenFailed)`
    - Error message contains "permission" or "denied"
    - No partial files created
    - Benchmark records error, not panic

- `test_returns_wal_write_failed_when_disk_full`
  - **Given**: DurableEventStore with full disk (mocked or small partition)
  - **When**: Attempting to append event
  - **Then**:
    - Returns `Err(AppendError::WalWriteFailed)`
    - WAL file is closed cleanly
    - No corruption in existing WAL data

- `test_returns_wal_sync_failed_when_fsync_unsupported`
  - **Given**: File system that doesn't support fsync (e.g., special mount)
  - **When**: Calling sync_all on WAL file
  - **Then**:
    - Returns `Err(AppendError::WalSyncFailed)`
    - Error message indicates fsync failure
    - File handle is released

- `test_returns_serialization_failed_when_event_not_serializable`
  - **Given**: BeadEvent with non-serializable field (e.g., contains Rc)
  - **When**: Calling append_event
  - **Then**:
    - Returns `Err(AppendError::SerializationFailed)`
    - Error mentions bincode failure
    - No WAL write attempted

- `test_returns_database_write_failed_when_connection_lost`
  - **Given**: DurableEventStore with disconnected SurrealDB
  - **When**: WAL append succeeds but database create fails
  - **Then**:
    - Returns `Err(AppendError::DatabaseWriteFailed)`
    - WAL entry exists (written before DB attempt)
    - Error indicates connection issue
    - Transaction is atomic (no partial state)

## Edge Case Tests

- `test_handles_minimum_payload_size_gracefully`
  - **Given**: BeadEvent with empty payload (smallest possible size)
  - **When**: Appending event
  - **Then**:
    - Append succeeds
    - Latency is < 1ms (faster than 1KB payload)
    - fsync still occurs (durable)

- `test_handles_maximum_payload_size_gracefully`
  - **Given**: BeadEvent with 10KB payload (large but realistic)
  - **When**: Appending event
  - **Then**:
    - Append succeeds
    - Latency is < 10ms (scales linearly)
    - No memory allocation failures

- `test_handles_unicode_in_event_fields`
  - **Given**: BeadEvent with emoji and multi-byte characters in title/description
  - **When**: Serializing and appending
  - **Then**:
    - Serialization succeeds
    - Append succeeds
    - Round-trip deserialization preserves unicode

- `test_handles_consecutive_rapid_appends`
  - **Given**: DurableEventStore
  - **When**: Appending 100 events as fast as possible
  - **Then**:
    - All appends succeed
    - No fsync errors
    - Latency remains stable (no degradation)
    - WAL file grows correctly

- `test_handles_event_with_all_optional_fields_populated`
  - **Given**: BeadEvent with all Option/Vec fields populated (maximal size)
  - **When**: Appending event
  - **Then**:
    - Serialization succeeds
    - Append succeeds
    - Serialized size < 1KB (system constraint)

## Contract Verification Tests

- `test_precondition_database_connection_is_valid`
  - **Given**: DurableEventStore with valid SurrealDB connection
  - **When**: Checking connection before append
  - **Then**:
    - Connection is active
    - Namespace is selected
    - Database is selected
    - If precondition violated, append returns error before writing

- `test_postcondition_wal_precedes_database_insert`
  - **Given**: DurableEventStore and test event
  - **When**: Appending event with instrumentation
  - **Then**:
    - WAL write completes before DB insert
    - fsync completes before DB insert
    - Order: serialize → WAL write → fsync → DB insert

- `test_postcondition_length_prefix_encoding_is_correct`
  - **Given**: WAL file after append
  - **When**: Reading WAL file directly
  - **Then**:
    - First 4 bytes = serialized length (big-endian u32)
    - Following N bytes = bincode serialized event
    - Length matches actual byte count
    - Event can be deserialized from WAL

- `test_invariant_event_ids_are_unique`
  - **Given**: DurableEventStore and 1000 appends
  - **When**: Querying all events from database
  - **Then**:
    - All event_id values are unique
    - No duplicates in state_transition table
    - Duplicate append returns error

- `test_invariant_timestamps_are_monotonic_for_same_bead`
  - **Given**: Single bead_id and 100 appends
  - **When**: Reading events in order
  - **Then**:
    - Timestamps are non-decreasing
    - Each timestamp >= previous timestamp
    - ULID event_id maintains ordering

- `test_invariant_resource_cleanup_on_error`
  - **Given**: DurableEventStore configured to fail on write
  - **When**: Append fails with WalWriteFailed
  - **Then**:
    - WAL file handle is closed
    - Temporary files are cleaned up
    - No file descriptors leaked
    - Benchmark can run again immediately

## Given-When-Then Scenarios

### Scenario 1: Successful append with 1KB payload meets performance targets

**Given**:
- A fresh DurableEventStore instance
- A BeadEvent with 1KB payload (realistic size)
- Criterion benchmark configured with:
  - warm_up_time: 3 seconds
  - measurement_time: 10 seconds
  - sample_size: 100

**When**:
- Benchmark runs 100 iterations
- Each iteration measures `append_event` latency
- Criterion calculates statistics

**Then**:
- p50 (median) latency < 3ms
- p90 latency < 4ms
- p95 latency < 4.5ms
- p99 latency < 5ms
- Standard deviation < 1ms
- All events are persisted in SurrealDB
- All WAL files are fsync'd
- Temporary directories are cleaned up

### Scenario 2: Benchmark handles serialization failure gracefully

**Given**:
- A DurableEventStore instance
- A BeadEvent with non-serializable data (e.g., contains Rc<u8>)
- Benchmark error handling configured

**When**:
- Benchmark attempts to append the event
- Serialization fails in bincode

**Then**:
- Function returns `Err(AppendError::SerializationFailed)`
- Error message contains "bincode" and "serialize"
- No WAL file is created
- No database insert attempted
- Benchmark records failure, continues to next iteration
- No panic occurs

### Scenario 3: Latency breakdown shows fsync is dominant operation

**Given**:
- A DurableEventStore instance
- Instrumented append_event with per-operation timing
- Standard 1KB test event

**When**:
- Running single append with timing breakdown
- Measuring: serialize, write, fsync, db_insert

**Then**:
- Serialization time: < 0.1ms (fast)
- Write time: < 0.5ms (buffered)
- fsync time: 2-4ms (dominant, expected)
- DB insert time: < 0.5ms (fast)
- Total: < 5ms (meets target)
- fsync accounts for > 60% of total latency
- Output includes breakdown in Criterion JSON

### Scenario 4: Fresh database per iteration prevents cache warming

**Given**:
- Benchmark configured to run 50 iterations
- Each iteration creates new TempDir
- Each iteration initializes new SurrealDB instance

**When**:
- Running full benchmark
- Checking database state between iterations

**Then**:
- Each iteration uses different temp directory path
- SurrealDB data directory is empty at start of iteration
- No cache data from previous iterations
- Cold start performance measured consistently
- Temp dirs are cleaned up after benchmark completes

### Scenario 5: Benchmark scales linearly with payload size

**Given**:
- DurableEventStore instance
- Test events with sizes: 100B, 1KB, 10KB, 100KB
- Criterion configured with input size parameterization

**When**:
- Running benchmark for each payload size
- Recording latency percentiles per size

**Then**:
- 100B: p99 < 1ms
- 1KB: p99 < 5ms
- 10KB: p99 < 20ms
- 100KB: p99 < 100ms
- Latency vs size is approximately linear
- Criterion plots show linear relationship
- No unexpected spikes at certain sizes

## Performance Regression Tests

- `test_regression_latency_does_not_exceed_baseline`
  - **Given**: Historical baseline p99 = 4.2ms for 1KB payload
  - **When**: Running current benchmark
  - **Then**:
    - Current p99 <= baseline * 1.1 (10% tolerance)
    - If regression detected, test fails with message
    - Suggested action: investigate recent changes

- `test_regression_fsync_overhead_stable`
  - **Given**: Historical fsync time = 3.0ms ± 0.5ms
  - **When**: Measuring fsync time in current run
  - **Then**:
    - fsync time within historical range
    - If degraded, check file system health

## Infrastructure Tests

- `test_criterion_configuration_is_correct`
  - **Given**: Benchmark binary
  - **When**: Inspecting Criterion config
  - **Then**:
    - warm_up_time >= 3 seconds
    - measurement_time >= 10 seconds
    - sample_size >= 100
    - confidence_level = 0.95

- `test_tempfile_cleanup_guaranteed`
  - **Given**: Benchmark that panics mid-execution
  - **When**: Process terminates abnormally
  - **Then**:
    - TempDir destructor runs (RAII)
    - Temporary files are deleted
    - No orphaned files in /tmp

- `test_tokio_runtime_available_in_benchmark`
  - **Given**: Criterion benchmark function
  - **When**: Creating tokio runtime
  - **Then**:
    - Runtime creation succeeds
    - Async functions can be blocked on
    - Runtime is shut down cleanly after benchmark
