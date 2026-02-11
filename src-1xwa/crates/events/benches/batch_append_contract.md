# Contract Specification: Batch Append Benchmark

## Context
- **Feature**: Batch append benchmark for event-sourcing system
- **Bead ID**: src-ufu
- **Domain terms**:
  - *Single append*: Individual `append_event()` call with one BeadEvent
  - *Batch append*: Hypothetical `append_batch()` call that accepts multiple BeadEvents
  - *Throughput*: Events processed per second (events/sec)
  - *fsync amortization*: Single fsync operation for entire batch vs per-event fsync
  - *Batch size*: Number of events in a single batch operation

- **Assumptions**:
  1. Batch append API (`append_batch()`) needs to be designed as part of implementation
  2. DurableEventStore will be extended with batch append functionality
  3. Benchmark should compare single append (baseline) vs batch append (improved)
  4. Expected performance improvement: 10x+ throughput gain for batches of 100+ events
  5. Fsync amortization is the primary optimization (1 fsync per batch, not per event)

- **Open questions**:
  1. Should batch append be transactional (all-or-nothing)? **Decision**: Yes, maintain atomicity guarantees
  2. What is the maximum batch size before performance degrades? **To be measured**: Test up to 1000 events
  3. Should batch append use a single WAL write or multiple? **Decision**: Single contiguous WAL write with length prefix per event
  4. How to handle partial failures (some events succeed, others fail)? **Decision**: Rollback entire batch, return detailed error

## Preconditions
- DurableEventStore is initialized with valid database connection
- WAL directory (`.wal`) exists or can be created
- All events in batch have valid BeadEvent structure (non-empty bead_id, valid event_id)
- Batch size is within reasonable bounds (1 to 1000 events)
- Database connection is active and not at capacity

## Postconditions
- **Success path**:
  - All events from batch are written to WAL in a single contiguous write
  - Exactly ONE fsync operation is performed for the entire batch
  - All events are persisted to SurrealDB
  - All event IDs are sequentially assigned and returned
  - WAL file contains all events with proper length prefixes
  - Database query can retrieve all events by their bead_ids

- **Error path**:
  - If any event serialization fails: No events written to WAL, no fsync performed
  - If WAL write fails: No partial data in WAL, WAL file is in consistent state
  - If fsync fails: WAL write is rolled back, database unchanged
  - If database write fails: WAL is written (durable), detailed error returned
  - Error message identifies which event failed and why

## Invariants
- **Atomicity**: Either all events in batch are persisted, or none are
- **Ordering**: Events are written to WAL in the order provided in the batch
- **Fsync cardinality**: Exactly ONE fsync per successful batch append operation
- **ID uniqueness**: Each event in batch receives a unique EventId
- **WAL consistency**: WAL file can be read and parsed after any successful batch append
- **Timestamp monotonicity**: Event timestamps are non-decreasing within a batch
- **Database-WAL alignment**: Database and WAL contain identical event data after successful append

## Error Taxonomy

### AppendBatchError (new error type)
- **AppendBatchError::EmptyBatch** - when batch contains zero events
- **AppendBatchError::BatchTooLarge** - when batch size exceeds maximum (1000)
- **AppendBatchError::SerializationFailed { index: usize, event_id: String }** - when specific event cannot be serialized
- **AppendBatchError::WalOpenFailed(String)** - when WAL file cannot be opened/created
- **AppendBatchError::WalWriteFailed(String)** - when write to WAL fails
- **AppendBatchError::WalSyncFailed(String)** - when fsync fails
- **AppendBatchError::WalCloseFailed(String)** - when WAL file cannot be closed
- **AppendBatchError::DatabaseWriteFailed(String)** - when SurrealDB batch insert fails
- **AppendBatchError::PreconditionViolation(String)** - when input validation fails (invalid event IDs, etc.)

### Error conversion
- `AppendBatchError` implements `From<AppendError>` for single-event errors
- `AppendBatchError` implements `Display` and `Error` traits
- `AppendBatchError` converts to `crate::error::Error::store_failed("append_batch", ...)`

## Contract Signatures

### Core batch append function (to be implemented)
```rust
impl DurableEventStore {
    /// Append multiple events in a single atomic operation.
    ///
    /// # Preconditions
    /// - Batch is non-empty
    /// - Batch size <= 1000
    /// - All events have valid bead_id and event_id
    ///
    /// # Postconditions
    /// - All events written to WAL in single contiguous write
    /// - Exactly ONE fsync performed for entire batch
    /// - All events persisted to SurrealDB
    /// - Returns vector of EventIds in same order as input
    ///
    /// # Errors
    /// - Returns `AppendBatchError::EmptyBatch` if batch is empty
    /// - Returns `AppendBatchError::BatchTooLarge` if size > 1000
    /// - Returns `AppendBatchError::SerializationFailed` if any event cannot serialize
    /// - Returns `AppendBatchError::WalWriteFailed` if WAL write fails
    /// - Returns `AppendBatchError::WalSyncFailed` if fsync fails
    /// - Returns `AppendBatchError::DatabaseWriteFailed` if DB insert fails
    pub async fn append_batch(
        &self,
        events: &[BeadEvent]
    ) -> Result<Vec<EventId>, AppendBatchError>
    {
        // Implementation must use Result<T, Error> pattern
        // No unwrap(), expect(), or panic!() allowed
    }
}
```

### Batch WAL write helper (internal)
```rust
impl DurableEventStore {
    /// Write multiple events to WAL in single operation with single fsync.
    ///
    /// # Contract
    /// - Preconditions: WAL directory exists, events non-empty
    /// - Postconditions: All events written to WAL, single fsync completed
    /// - Invariants: WAL file is parseable after write
    ///
    /// # Errors
    /// - Returns `AppendBatchError::WalOpenFailed` if file cannot be opened
    /// - Returns `AppendBatchError::WalWriteFailed` if write fails
    /// - Returns `AppendBatchError::WalSyncFailed` if fsync fails
    async fn append_batch_to_wal(
        &self,
        events: &[(BeadEvent, SerializedEvent)]
    ) -> Result<(), AppendBatchError>
    {
        // Implementation must:
        // 1. Serialize all events first (fail fast)
        // 2. Open WAL file once
        // 3. Write all events with length prefixes
        // 4. Call sync_all() exactly once
        // 5. Return Result<(), AppendBatchError>
    }
}
```

### Batch database insert helper (internal)
```rust
impl DurableEventStore {
    /// Insert multiple events into SurrealDB in batch.
    ///
    /// # Contract
    /// - Preconditions: All events serialized successfully
    /// - Postconditions: All events inserted into database
    /// - Invariants: Database contains all events from WAL
    ///
    /// # Errors
    /// - Returns `AppendBatchError::DatabaseWriteFailed` if insert fails
    async fn insert_batch_to_db(
        &self,
        serialized_events: &[SerializedEvent]
    ) -> Result<(), AppendBatchError>
    {
        // Implementation must:
        // 1. Use SurrealDB batch insert API
        // 2. Return detailed error on failure
        // 3. Maintain Result<T, Error> pattern
    }
}
```

## Non-goals
- **Out of scope for this bead**:
  - Implementing batch read/replay operations (separate bead)
  - Implementing batch checkpoint/resume (separate bead)
  - Optimizing batch size dynamically (future enhancement)
  - Cross-batch transaction support (future enhancement)
  - Parallel batch append (future enhancement)
  - Batch append for in-memory store (not needed for benchmark)

## Performance Targets (from bead description)
- **Throughput**: Batch append should achieve 10x+ improvement vs single append
- **Batch sizes to test**: 1, 10, 50, 100, 500, 1000 events
- **Fsync verification**: Exactly ONE fsync per batch (not per event)
- **Metrics to report**:
  - Events/second throughput for each batch size
  - Latency per event (microseconds)
  - Total wall-clock time per batch
  - Fsync count verification (must be 1 per batch)
