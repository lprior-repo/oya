# Contract Specification
## Checkpoint-Based Resume for Event Replay

**Bead ID**: src-hrzw
**Feature**: EventSourcingReplay: Implement checkpoint-based resume
**Date**: 2026-02-08

---

## Context

### Feature Description
Resume event replay from a checkpoint instead of replaying all events from the beginning. This optimization allows event-sourced systems to quickly restore state by loading a checkpoint and replaying only events that occurred after the checkpoint was created.

### Domain Terms
- **Checkpoint**: A serialized snapshot of application state at a specific point in time, identified by a unique `CheckpointId`
- **ReplayState**: The state object tracking replay progress, including checkpoint ID, timestamp, and events replayed
- **EventLog**: Append-only log of domain events with timestamps and sequence numbers
- **CheckpointStore**: Storage backend for persisting and retrieving checkpoints
- **TimestampMismatch**: Validation failure when checkpoint timestamp doesn't match event log consistency

### Assumptions
1. Event timestamps are monotonic and strictly increasing
2. Checkpoint IDs are unique within the store
3. Event log is immutable (append-only)
4. Checkpoint data includes serialized state that can be deserialized by the application
5. Timestamp validation ensures causal consistency between checkpoint and event log

### Open Questions
- **Q1**: Should checkpoint validation verify that checkpoint timestamp corresponds to an actual event in the log?
  - **A1**: No, validation only checks that the timestamp is within valid log bounds
- **Q2**: What happens if events are missing between checkpoint timestamp and current log head?
  - **A2**: System loads all available events after checkpoint; gaps may indicate log corruption but are not explicitly detected
- **Q3**: Should compression of checkpoint data be mandatory?
  - **A3**: No, compression is optional and tracked via `CheckpointData.compressed` flag

---

## Preconditions

### For `resume_from_checkpoint`
1. **Checkpoint must exist**: Checkpoint ID must reference a valid checkpoint in the store
2. **Checkpoint data must be valid**: Checkpoint data must be deserializable and not corrupted
3. **Timestamp must be valid**: Checkpoint timestamp must be within the valid range of the event log
4. **Event log must be accessible**: Event log backend must be available for querying
5. **Checkpoint store must be accessible**: Checkpoint store backend must be available for loading

### For `CheckpointStore::load_checkpoint`
1. Store backend must be initialized and connected
2. Checkpoint ID must be non-empty string

### For `CheckpointStore::validate_timestamp`
1. Checkpoint must exist (verified before validation call)
2. Event log must have at least one event for comparison

### For `EventLog::load_events_after`
1. Timestamp must be valid UTC datetime
2. Event log must be readable

---

## Postconditions

### For `resume_from_checkpoint` (Success Path)
1. **Returns `Ok(ReplayState)`**: Result contains valid replay state
2. **Checkpoint ID preserved**: `ReplayState.checkpoint_id` matches input `checkpoint_id`
3. **Checkpoint timestamp preserved**: `ReplayState.checkpoint_timestamp` matches loaded checkpoint
4. **Events counted**: `ReplayState.events_replayed` equals number of events in log with `timestamp > checkpoint_timestamp`
5. **Last event timestamp set**: `ReplayState.last_event_timestamp` is `Some(timestamp)` of most recent event, or `None` if no events after checkpoint
6. **No state mutation**: Original checkpoint store and event log are unchanged

### For `resume_from_checkpoint` (Failure Path)
1. **Returns `Err(ResumeError)`**: Error type semantically describes failure mode
2. **No partial state**: No `ReplayState` is created or returned
3. **Error is actionable**: Error message provides enough context to diagnose and fix the issue

### For `CheckpointStore::load_checkpoint`
1. **Returns `Ok(Some(data, timestamp))`**: If checkpoint exists
2. **Returns `Ok(None)`**: If checkpoint doesn't exist (not an error)
3. **Returns `Err(Error)`**: If store access fails

### For `CheckpointStore::validate_timestamp`
1. **Returns `Ok(true)`**: If timestamp is valid and matches event log
2. **Returns `Ok(false)`**: If timestamp doesn't match event log
3. **Returns `Err(Error)`**: If validation operation fails

### For `EventLog::load_events_after`
1. **Returns `Ok(events)`**: Vector of events with `timestamp > input_timestamp`
2. **Events are ordered**: Events returned in chronological order by timestamp
3. **Returns `Err(Error)`**: If event log access fails

---

## Invariants

### CheckpointStore Invariants
1. **Checkpoint ID uniqueness**: Each ID maps to at most one checkpoint
2. **Timestamp consistency**: Checkpoint timestamp is immutable once created
3. **Data integrity**: Checkpoint data, if present, is valid and deserializable

### EventLog Invariants
1. **Event immutability**: Once written, events never change
2. **Monotonic timestamps**: Event timestamps are non-decreasing
3. **Sequential ordering**: Events can be ordered by timestamp or sequence number

### ReplayState Invariants
1. **Events replayed is monotonically increasing**: `events_replayed` only increases
2. **Last event timestamp advances**: `last_event_timestamp` only moves forward in time
3. **Checkpoint timestamp is immutable**: Never changes after state creation
4. **Events replayed <= total events available**: Cannot replay more events than exist in log

### System-wide Invariants
1. **Causal consistency**: All events after checkpoint timestamp are replayed
2. **No event loss**: Every event with `timestamp > checkpoint_timestamp` is counted
3. **No event duplication**: Each event is counted exactly once

---

## Error Taxonomy

### `ResumeError::CheckpointNotFound`
**When**: Checkpoint ID does not exist in checkpoint store
**Context**: User typo, checkpoint expired, or checkpoint never created
**Recovery**: Verify checkpoint ID, check checkpoint lifecycle policy, create new checkpoint
**Example**: `checkpoint 'prod-2024-01-01' not found`

### `ResumeError::TimestampMismatch`
**When**: Checkpoint timestamp does not match event log at that position
**Context**: Checkpoint corrupted, event log truncated, or clock skew
**Recovery**: Restore event log from backup, recreate checkpoint from valid state
**Example**: `checkpoint 'cp-123' timestamp 2024-01-01T12:00:00Z does not match event log 2024-01-01T12:05:00Z`

### `ResumeError::InvalidCheckpoint`
**When**: Checkpoint data is corrupted, malformed, or cannot be deserialized
**Context**: Disk corruption, serialization format change, or incomplete write
**Recovery**: Restore from backup, recreate checkpoint, implement version tolerance
**Example**: `invalid checkpoint: corrupted state data`

### `ResumeError::EventLoadFailed`
**When**: Event log backend fails to load events after checkpoint timestamp
**Context**: Network failure, disk error, permission denied, or log corrupted
**Recovery**: Check event log backend health, verify permissions, repair log
**Example**: `failed to load events after checkpoint: connection timeout`

### Error Conversion Hierarchy
```
ResumeError
  └─> Error::Internal (via From trait)
      └─> propagates to calling code
```

---

## Contract Signatures

### Core Function
```rust
pub fn resume_from_checkpoint<S, L>(
    checkpoint_id: &CheckpointId,
    checkpoints: &S,
    event_log: &L,
) -> Result<ReplayState, ResumeError>
where
    S: CheckpointStore,
    L: EventLog
```

**Railway-Oriented Programming**: Function returns `Result`, forcing caller to handle both success and failure paths explicitly.

### Trait: CheckpointStore
```rust
pub trait CheckpointStore: Send + Sync {
    fn load_checkpoint(
        &self,
        checkpoint_id: &CheckpointId,
    ) -> Result<Option<(CheckpointData, DateTime<Utc>)>>;

    fn validate_timestamp(
        &self,
        checkpoint_id: &CheckpointId,
        checkpoint_timestamp: DateTime<Utc>,
    ) -> Result<bool>;
}
```

### Trait: EventLog
```rust
pub trait EventLog: Send + Sync {
    fn load_events_after(
        &self,
        timestamp: DateTime<Utc>,
    ) -> Result<Vec<EventMetadata>>;
}
```

### Supporting Types
```rust
pub struct CheckpointId(String);
pub struct CheckpointData { pub state: Vec<u8>, pub sequence_number: u64, pub compressed: bool }
pub struct ReplayState { pub checkpoint_id: CheckpointId, pub checkpoint_timestamp: DateTime<Utc>, pub events_replayed: u64, pub last_event_timestamp: Option<DateTime<Utc>> }
pub struct EventMetadata { pub event_id: String, pub timestamp: DateTime<Utc>, pub sequence_number: u64 }

impl ReplayState {
    pub fn new(checkpoint_id: CheckpointId, checkpoint_timestamp: DateTime<Utc>) -> Self;
    pub fn record_event(&mut self, timestamp: DateTime<Utc>);
}
```

---

## Performance Requirements

### Success Criterion
- **Replay 1000 events in <5s** with checkpoint resume
- **Checkpoint load**: <100ms for typical checkpoint (1-10MB compressed)
- **Event log query**: <500ms for 1000 events after timestamp

### Optimization Strategy
1. Checkpoint provides fast initial state (avoid replaying all historical events)
2. Only replay events after checkpoint timestamp
3. Event log query is indexed by timestamp
4. Checkpoint data may be compressed to reduce I/O

---

## Non-goals

1. **Checkpoint creation**: This contract only covers resume, not checkpoint creation
2. **Checkpoint deletion/lifecycle**: Not in scope for resume functionality
3. **Event replay execution**: This contract only counts events, doesn't execute replay logic
4. **Distributed consensus**: No coordination across multiple nodes
5. **Checkpoint versioning/migration**: Assumes checkpoint format is stable
6. **Real-time resume**: Not optimized for hot-reload during active processing
7. **Compression algorithm**: Not specified; implementation choice

---

## Safety and Correctness Guarantees

### Zero Unwraps
- All fallible operations return `Result<T, E>`
- No `.unwrap()` or `.expect()` calls in production code
- Errors are propagated via `?` operator

### Zero Panics
- No `panic!()`, `todo!()`, `unimplemented!()` in production code
- All error conditions return `Err`

### Railway-Oriented Programming
- Function composition: `load_checkpoint.and_then(validate).and_then(replay)`
- Error handling is explicit, not hidden
- Callers must handle both success and failure paths

### Thread Safety
- `CheckpointStore` and `EventLog` traits require `Send + Sync`
- Safe to share references across threads
- No interior mutability in core types

---

## Testing Strategy

See `martin-fowler-tests.md` for comprehensive test plan covering:
- Happy path scenarios
- Error path for each `ResumeError` variant
- Edge cases (empty logs, boundary timestamps, missing checkpoints)
- Contract verification (preconditions, postconditions, invariants)
- Integration scenarios with mock implementations

---

## Verification Checklist

Before implementation is considered complete:

- [ ] All error variants have corresponding tests
- [ ] All preconditions have validation tests
- [ ] All postconditions have assertion tests
- [ ] All invariants have property-based tests
- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- [ ] All functions return `Result<T, E>` for fallible operations
- [ ] Error messages are actionable and informative
- [ ] Performance requirement: 1000 events in <5s
- [ ] Thread safety: `Send + Sync` constraints documented
- [ ] Integration test with realistic checkpoint and event log sizes
