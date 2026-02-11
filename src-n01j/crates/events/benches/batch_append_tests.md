# Martin Fowler Test Plan: Batch Append Benchmark

## Happy Path Tests
- **test_batch_append_with_ten_events_succeeds**
  - Given: A batch of 10 valid BeadEvents with unique IDs
  - When: append_batch() is called with the batch
  - Then:
    - All 10 events are persisted to WAL
    - Exactly ONE fsync is performed
    - All 10 events are inserted into SurrealDB
    - Vector of 10 EventIds is returned in the same order
    - Database query returns all 10 events

- **test_batch_append_with_maximum_size_succeeds**
  - Given: A batch of 1000 valid BeadEvents (maximum allowed)
  - When: append_batch() is called with the batch
  - Then:
    - All 1000 events are persisted
    - Single fsync is performed
    - All 1000 EventIds are returned
    - Throughput is measured and reported

- **test_batch_append_amortizes_fsync_overhead**
  - Given: A batch of 100 events
  - When: append_batch() is called
  - Then:
    - Exactly 1 fsync operation is performed (verified via tracing/counting)
    - Fsync count is NOT equal to batch size (100)
    - Performance is significantly faster than 100 single appends

- **test_batch_append_preserves_event_order**
  - Given: A batch of 5 events with timestamps T1 < T2 < T3 < T4 < T5
  - When: append_batch() is called
  - Then:
    - Events are written to WAL in order T1, T2, T3, T4, T5
    - Database query returns events in order T1, T2, T3, T4, T5
    - Returned EventIds are in same order as input

- **test_batch_append_writes_contiguous_wal**
  - Given: A batch of 3 events
  - When: append_batch() is called
  - Then:
    - WAL file contains all 3 events in single contiguous block
    - Each event has proper length prefix
    - WAL can be parsed and all events deserialized

- **test_batch_append_with_single_event_matches_single_append**
  - Given: A batch containing exactly 1 event
  - When: append_batch() is called
  - Then:
    - Result is identical to single append_event()
    - Same WAL format, same DB records
    - Performance may be slightly worse due to batch overhead

## Error Path Tests
- **test_batch_append_returns_error_when_batch_is_empty**
  - Given: An empty batch (zero events)
  - When: append_batch() is called
  - Then:
    - Returns `Err(AppendBatchError::EmptyBatch)`
    - No WAL files are created
    - No database writes occur
    - No fsync is performed

- **test_batch_append_returns_error_when_batch_exceeds_maximum**
  - Given: A batch of 1001 events (exceeds maximum of 1000)
  - When: append_batch() is called
  - Then:
    - Returns `Err(AppendBatchError::BatchTooLarge)`
    - No writes occur
    - Error message indicates maximum allowed size

- **test_batch_append_returns_error_when_event_serialization_fails**
  - Given: A batch where one event contains invalid data that cannot be serialized
  - When: append_batch() is called
  - Then:
    - Returns `Err(AppendBatchError::SerializationFailed { index, event_id })`
    - Error identifies which event failed
    - No events are written to WAL
    - No fsync is performed
    - Database is unchanged

- **test_batch_append_returns_error_when_wal_write_fails**
  - Given: A batch of valid events, but WAL directory is read-only (simulated failure)
  - When: append_batch() is called
  - Then:
    - Returns `Err(AppendBatchError::WalWriteFailed)`
    - Partial data is not written to WAL (atomicity)
    - No fsync is performed
    - Database is unchanged

- **test_batch_append_returns_error_when_fsync_fails**
  - Given: A batch of valid events, but file system simulates fsync failure
  - When: append_batch() is called
  - Then:
    - Returns `Err(AppendBatchError::WalSyncFailed)`
    - WAL write is rolled back if possible
    - Database is unchanged

- **test_batch_append_returns_error_when_database_write_fails**
  - Given: A batch of valid events, but database connection is closed
  - When: append_batch() is called
  - Then:
    - Returns `Err(AppendBatchError::DatabaseWriteFailed)`
    - WAL is written (data is durable)
    - Error indicates database failure specifically

## Edge Case Tests
- **test_batch_append_handles_batch_size_of_one**
  - Given: A batch containing exactly 1 event
  - When: append_batch() is called
  - Then:
    - Succeeds and returns single EventId
    - Performs exactly 1 fsync
    - WAL is written correctly

- **test_batch_append_handles_batch_size_of_two**
  - Given: A batch containing exactly 2 events
  - When: append_batch() is called
  - Then:
    - Succeeds and returns 2 EventIds
    - Both events in same WAL file
    - Single fsync for both

- **test_batch_append_handles_events_with_same_bead_id**
  - Given: A batch of 5 events, all with the same bead_id
  - When: append_batch() is called
  - Then:
    - All 5 events are persisted
    - Database query for that bead_id returns all 5 events
    - Events are ordered by timestamp

- **test_batch_append_handles_events_with_different_bead_ids**
  - Given: A batch of 5 events, each with a different bead_id
  - When: append_batch() is called
  - Then:
    - All 5 events are persisted
    - Each bead_id query returns its respective event
    - Database contains all 5 events

- **test_batch_append_handles_zero_length_event_data**
  - Given: A batch containing an event with empty payload (valid but minimal)
  - When: append_batch() is called
  - Then:
    - Succeeds and persists the event
    - WAL contains length prefix of 0 for that event
    - Database record has empty data field

- **test_batch_append_handles_maximum_size_event_data**
  - Given: A batch containing an event with large payload (1MB)
  - When: append_batch() is called with appropriate batch size limit
  - Then:
    - Succeeds and persists the large event
    - WAL contains correct length prefix
    - Deserialization succeeds

## Contract Verification Tests
- **test_precondition_batch_non_empty**
  - Given: An empty batch
  - When: append_batch() is called
  - Then: Returns `Err(AppendBatchError::EmptyBatch)`

- **test_precondition_batch_size_within_limit**
  - Given: A batch of 1001 events (exceeds limit)
  - When: append_batch() is called
  - Then: Returns `Err(AppendBatchError::BatchTooLarge)`

- **test_postcondition_all_events_persisted**
  - Given: A batch of 10 events
  - When: append_batch() succeeds
  - Then: All 10 events are retrievable from database

- **test_postcondition_single_fsync_performed**
  - Given: A batch of 50 events
  - When: append_batch() succeeds
  - Then: Exactly 1 fsync operation was performed (counted via instrumentation)

- **test_postcondition_event_ids_in_order**
  - Given: A batch of events [E1, E2, E3]
  - When: append_batch() succeeds
  - Then: Returns [ID1, ID2, ID3] where ID1 corresponds to E1, etc.

- **test_postcondition_wal_file_consistent**
  - Given: A batch of 5 events
  - When: append_batch() succeeds
  - Then: WAL file can be opened and parsed, all events deserializable

- **test_invariant_atomicity_on_serialization_failure**
  - Given: A batch where event 3 of 5 cannot be serialized
  - When: append_batch() is called
  - Then: No events are written to WAL or database

- **test_invariant_atomicity_on_wal_failure**
  - Given: A batch where WAL write fails mid-batch
  - When: append_batch() is called
  - Then: No partial data in WAL, database unchanged

- **test_invariant_wal_db_alignment**
  - Given: A batch of 10 events
  - When: append_batch() succeeds
  - Then: WAL and database contain identical event data

## Given-When-Then Scenarios

### Scenario 1: Successful batch append with 100 events
**Given**:
- A DurableEventStore connected to a test database
- A batch of 100 valid BeadEvents with unique IDs
- WAL directory is writable

**When**:
- `append_batch(&events)` is called

**Then**:
- Returns `Ok(Vec<EventId>)` with 100 IDs
- All 100 events are in the database
- WAL file contains 100 events with length prefixes
- Exactly 1 fsync was performed
- Throughput is calculated (events / second)
- Performance is compared to 100 single appends

### Scenario 2: Batch append fails due to invalid event in middle of batch
**Given**:
- A batch of 50 events
- Event #25 contains invalid data that cannot be serialized

**When**:
- `append_batch(&events)` is called

**Then**:
- Returns `Err(AppendBatchError::SerializationFailed { index: 25, event_id: ... })`
- No events are written to WAL
- No events are in the database
- No fsync was performed
- Error message clearly identifies event #25 as the problem

### Scenario 3: Batch append achieves 10x throughput improvement
**Given**:
- Baseline single append: 100 events takes 5 seconds (20 events/sec)
- Batch append with 100 events

**When**:
- `append_batch(&events)` is called with 100 events
- Time is measured

**Then**:
- Batch append completes in < 0.5 seconds (> 200 events/sec)
- Throughput improvement is > 10x
- Single fsync (vs 100 fsyncs for single appends)

### Scenario 4: Fsync amortization verification
**Given**:
- A batch of 500 events
- Instrumentation to count fsync operations

**When**:
- `append_batch(&events)` is called
- Fsync count is recorded

**Then**:
- Exactly 1 fsync operation counted
- If 500 single appends: would be 500 fsyncs
- Amortization factor: 500x reduction in fsync calls

### Scenario 5: Batch append scales with batch size
**Given**:
- Test batches of sizes: 1, 10, 50, 100, 500, 1000
- Timing infrastructure

**When**:
- Each batch size is tested
- Throughput (events/sec) is calculated for each

**Then**:
- Throughput increases with batch size (up to a point)
- Batch size 1000 achieves highest throughput
- Diminishing returns may be observed past optimal size
- Results are reported in benchmark output

## Benchmark-Specific Tests (Criterion Integration)

### Benchmark: Single append baseline
- **bench_single_append_baseline**
  - Measures: Time to append 1 event using existing `append_event()`
  - Purpose: Establish baseline for comparison
  - Batch size: 1 (repeated 100 times)
  - Metrics: Time per event, events/sec

### Benchmark: Batch append throughput
- **bench_batch_append_throughput**
  - Measures: Time to append N events using `append_batch()`
  - Batch sizes: [1, 10, 50, 100, 500, 1000]
  - Metrics: Total time, time per event, events/sec
  - Comparison: vs baseline single append

### Benchmark: Fsync verification
- **bench_verify_single_fsync_per_batch**
  - Measures: Fsync count for batch append
  - Verification: Count must equal 1 for any batch size
  - Comparison: Single append has fsync count = batch size

### Benchmark: Scalability analysis
- **bench_batch_append_scalability**
  - Measures: How throughput scales with batch size
  - Expectation: Throughput increases logarithmically or linearly
  - Analysis: Identify optimal batch size
