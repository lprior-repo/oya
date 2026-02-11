# Martin Fowler Test Plan
## Checkpoint-Based Resume for Event Replay

**Bead ID**: src-hrzw
**Based on Contract**: `agent_12_contract.md`
**Testing Philosophy**: Tests are executable specifications

---

## Happy Path Tests

### test_returns_success_when_valid_checkpoint_exists
**Given**: A checkpoint store with valid checkpoint ID "cp-123"
**And**: Checkpoint timestamp is 2024-01-01T12:00:00Z
**And**: Event log has 3 events after checkpoint timestamp
**When**: `resume_from_checkpoint("cp-123", store, log)` is called
**Then**:
  - Returns `Ok(ReplayState)`
  - `ReplayState.checkpoint_id` is "cp-123"
  - `ReplayState.checkpoint_timestamp` is 2024-01-01T12:00:00Z
  - `ReplayState.events_replayed` is 3
  - `ReplayState.last_event_timestamp` is timestamp of 3rd event

---

### test_creates_replay_state_with_zero_events_when_log_is_empty_after_checkpoint
**Given**: A checkpoint store with valid checkpoint ID "cp-empty"
**And**: Checkpoint timestamp is 2024-01-01T12:00:00Z
**And**: Event log has no events after checkpoint timestamp
**When**: `resume_from_checkpoint("cp-empty", store, log)` is called
**Then**:
  - Returns `Ok(ReplayState)`
  - `ReplayState.events_replayed` is 0
  - `ReplayState.last_event_timestamp` is `None`

---

### test_replay_state_records_events_in_chronological_order
**Given**: A checkpoint with timestamp T0
**And**: Event log has events at timestamps [T1, T2, T3] where T0 < T1 < T2 < T3
**When**: `resume_from_checkpoint` is called
**And**: Events are recorded in replay state
**Then**:
  - `events_replayed` increments from 0 to 3
  - `last_event_timestamp` updates to T1, then T2, then T3
  - Final `last_event_timestamp` is T3 (most recent)

---

### test_handles_compressed_checkpoint_data_correctly
**Given**: A checkpoint with `compressed: true` flag
**And**: Compressed state data is valid
**When**: `resume_from_checkpoint` is called
**Then**:
  - Checkpoint loads successfully
  - Returns `Ok(ReplayState)`
  - No error indicates compression issue

---

### test_preserves_sequence_number_from_checkpoint
**Given**: A checkpoint with `sequence_number: 100`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Checkpoint data includes sequence_number: 100
  - Sequence number is available for application use

---

## Error Path Tests

### test_returns_checkpoint_not_found_when_checkpoint_id_does_not_exist
**Given**: A checkpoint store with no checkpoints
**When**: `resume_from_checkpoint("missing-cp", store, log)` is called
**Then**:
  - Returns `Err(ResumeError::CheckpointNotFound)`
  - Error message contains "missing-cp"
  - Error message contains "not found"

---

### test_returns_timestamp_mismatch_when_validation_fails
**Given**: A checkpoint with timestamp 2024-01-01T12:00:00Z
**And**: Event log at that position has timestamp 2024-01-01T12:05:00Z
**And**: `validate_timestamp` returns `Ok(false)`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err` (timestamp validation failure)
  - Error message indicates timestamp mismatch
  - No replay state is created

---

### test_returns_invalid_checkpoint_when_data_is_corrupted
**Given**: A checkpoint store that returns corrupted checkpoint data
**And**: `load_checkpoint` returns `Err(Error::Internal("corrupted data"))`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err(ResumeError::InvalidCheckpoint)`
  - Error message contains "corrupted data" or "invalid checkpoint"

---

### test_returns_event_load_failed_when_event_log_is_unavailable
**Given**: A valid checkpoint exists
**And**: Event log backend is down or returns error
**And**: `load_events_after` returns `Err(Error::Internal("connection timeout"))`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err(ResumeError::EventLoadFailed)`
  - Error message contains "connection timeout"
  - No replay state is created

---

### test_propagates_store_errors_as_invalid_checkpoint
**Given**: Checkpoint store returns `Err(Error::Internal("disk full"))`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err(ResumeError::InvalidCheckpoint { reason: "disk full" })`

---

### test_propagates_validation_errors_as_invalid_checkpoint
**Given**: Checkpoint loads successfully
**And**: `validate_timestamp` returns `Err(Error::Internal("log corrupted"))`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err(ResumeError::InvalidCheckpoint { reason: "log corrupted" })`

---

## Edge Case Tests

### test_handles_empty_checkpoint_id_gracefully
**Given**: A checkpoint store
**When**: `resume_from_checkpoint(CheckpointId::new(""), store, log)` is called
**Then**:
  - Returns `Err(ResumeError::CheckpointNotFound)`
  - Error message indicates empty or invalid ID

---

### test_handles_boundary_timestamp_exactly_matching_first_event
**Given**: Checkpoint timestamp T0 exactly matches event E0 timestamp
**And**: Event log has events [E0 at T0, E1 at T1, E2 at T2]
**When**: `resume_from_checkpoint` is called
**Then**:
  - Events replayed may include or exclude E0 (implementation-defined)
  - Behavior is consistent and documented
  - Test verifies actual behavior matches contract

---

### test_handles_events_with_identical_timestamps
**Given**: Checkpoint timestamp T0
**And**: Event log has 3 events all at timestamp T1 where T1 > T0
**When**: `resume_from_checkpoint` is called
**Then**:
  - All 3 events are replayed
  - `events_replayed` is 3
  - `last_event_timestamp` is T1

---

### test_handles_very_large_event_count_after_checkpoint
**Given**: Checkpoint timestamp is very old
**And**: Event log has 10,000 events after checkpoint
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Ok(ReplayState)`
  - `events_replayed` is 10000
  - Operation completes in <5s (performance requirement)

---

### test_handles_single_event_after_checkpoint
**Given**: Checkpoint timestamp T0
**And**: Event log has exactly 1 event at T1 > T0
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Ok(ReplayState)`
  - `events_replayed` is 1
  - `last_event_timestamp` is T1

---

### test_handles_future_checkpoint_timestamp
**Given**: Checkpoint timestamp is in the future (relative to current time)
**And**: Event log has no events after future timestamp
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Ok(ReplayState)`
  - `events_replayed` is 0
  - `last_event_timestamp` is `None`

---

## Contract Verification Tests

### test_precondition_checkpoint_must_exist
**Given**: A checkpoint store
**And**: No checkpoint with ID "nonexistent" exists
**When**: `resume_from_checkpoint("nonexistent", store, log)` is called
**Then**:
  - Returns `Err(ResumeError::CheckpointNotFound)`
  - precondition violated → error returned

---

### test_precondition_checkpoint_data_must_be_valid
**Given**: Checkpoint store returns `Some((corrupted_data, timestamp))`
**And**: Checkpoint data cannot be deserialized
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err(ResumeError::InvalidCheckpoint)`
  - precondition violated → error returned

---

### test_precondition_timestamp_must_be_valid
**Given**: Checkpoint timestamp is invalid (e.g., NaN, far future)
**And**: `validate_timestamp` returns `Ok(false)` or `Err`
**When**: `resume_from_checkpoint` is called
**Then**:
  - Returns `Err` (timestamp validation failure)
  - precondition violated → error returned

---

### test_postcondition_checkpoint_id_preserved
**Given**: Input checkpoint_id is "test-cp-456"
**When**: `resume_from_checkpoint` returns `Ok(state)`
**Then**:
  - `state.checkpoint_id.as_str()` equals "test-cp-456"

---

### test_postcondition_checkpoint_timestamp_preserved
**Given**: Input checkpoint has timestamp 2024-02-08T10:30:00Z
**When**: `resume_from_checkpoint` returns `Ok(state)`
**Then**:
  - `state.checkpoint_timestamp` equals 2024-02-08T10:30:00Z

---

### test_postcondition_events_counted_correctly
**Given**: Event log has N events with timestamp > checkpoint_timestamp
**When**: `resume_from_checkpoint` returns `Ok(state)`
**Then**:
  - `state.events_replayed` equals N

---

### test_postcondition_last_event_timestamp_set_correctly
**Given**: Event log has events at [T1, T2, T3] where T3 is most recent
**When**: `resume_from_checkpoint` returns `Ok(state)`
**Then**:
  - `state.last_event_timestamp` equals `Some(T3)`

---

### test_postcondition_no_state_mutation
**Given**: Mock checkpoint store and event log
**When**: `resume_from_checkpoint` is called
**Then**:
  - Checkpoint store `load_checkpoint` called exactly once
  - Event log `load_events_after` called exactly once
  - No mutations to store or log state

---

### test_invariant_events_replayed_is_monotonically_increasing
**Given**: `ReplayState` with initial `events_replayed: 0`
**When**: `record_event` is called 3 times
**Then**:
  - `events_replayed` sequence is [0, 1, 2, 3]
  - Values never decrease

---

### test_invariant_last_event_timestamp_advances
**Given**: `ReplayState` with events at [T1, T2, T3]
**And**: T1 < T2 < T3
**When**: Events are recorded
**Then**:
  - `last_event_timestamp` sequence is [None, Some(T1), Some(T2), Some(T3)]
  - Timestamps never move backward

---

### test_invariant_events_replayed_does_not_exceed_available
**Given**: Event log has exactly 5 events after checkpoint
**When**: `resume_from_checkpoint` returns `Ok(state)`
**Then**:
  - `state.events_replayed` <= 5

---

### test_invariant_checkpoint_timestamp_is_immutable
**Given**: `ReplayState` created with checkpoint_timestamp T0
**When**: Any operations are performed on state
**Then**:
  - `checkpoint_timestamp` never changes from T0

---

## Given-When-Then Scenarios

### Scenario 1: Successful Resume with Active Event Stream
**Given**:
  - Checkpoint "prod-2024-02-08" exists with timestamp 2024-02-08T00:00:00Z
  - Event log contains 100 events since checkpoint
  - Last event timestamp is 2024-02-08T12:34:56Z

**When**:
  - User calls `resume_from_checkpoint("prod-2024-02-08", store, log)`

**Then**:
  - Replay state is created successfully
  - `events_replayed` is 100
  - `last_event_timestamp` is 2024-02-08T12:34:56Z
  - Operation completes in <1s (checkpoint resume optimization)

---

### Scenario 2: Checkpoint Expired and Removed
**Given**:
  - Checkpoint "prod-2024-01-01" has expired per retention policy
  - Checkpoint store returns `None` for this ID

**When**:
  - User calls `resume_from_checkpoint("prod-2024-01-01", store, log)`

**Then**:
  - Returns `Err(ResumeError::CheckpointNotFound)`
  - Error message indicates checkpoint not found
  - User can create new checkpoint from current state

---

### Scenario 3: Event Log Corruption Detected
**Given**:
  - Checkpoint exists with timestamp 2024-02-08T10:00:00Z
  - Event log at that position has timestamp 2024-02-08T09:00:00Z (goes backward)
  - Timestamp validation detects mismatch

**When**:
  - User calls `resume_from_checkpoint("corrupted-log-cp", store, log)`

**Then**:
  - Returns `Err` with timestamp mismatch
  - Error message includes both timestamps
  - System prevents replay with corrupted log
  - Admin can restore event log from backup

---

### Scenario 4: Network Partition During Event Load
**Given**:
  - Checkpoint loads successfully
  - Event log backend becomes unavailable
  - `load_events_after` returns `Err(Error::Internal("connection timeout"))`

**When**:
  - User calls `resume_from_checkpoint("valid-cp", store, log)`

**Then**:
  - Returns `Err(ResumeError::EventLoadFailed)`
  - Error message includes "connection timeout"
  - No partial replay state is created
  - User can retry when network is restored

---

### Scenario 5: First Checkpoint in System (No Previous Events)
**Given**:
  - This is the first checkpoint ever created
  - Checkpoint timestamp is system start time
  - No events exist before this timestamp

**When**:
  - User calls `resume_from_checkpoint("first-cp", store, log)`

**Then**:
  - Replay state is created successfully
  - `events_replayed` is 0
  - `last_event_timestamp` is `None`
  - System is ready to process new events

---

## Integration Scenarios

### Scenario 6: Resume Across Checkpoint Versions
**Given**:
  - Old checkpoint exists with deprecated format
  - System has been upgraded to new checkpoint format
  - Migration layer handles format conversion

**When**:
  - User calls `resume_from_checkpoint` with old checkpoint ID

**Then**:
  - Migration layer converts old format to new format
  - Returns `Ok(ReplayState)` with converted data
  - No error indicates format mismatch

**Note**: This scenario is a **non-goal** for current implementation but should be documented for future work.

---

### Scenario 7: Resume with Compressed Large Checkpoint
**Given**:
  - Checkpoint contains 100MB of uncompressed state
  - Checkpoint data is compressed to 10MB
  - `compressed: true` flag is set

**When**:
  - User calls `resume_from_checkpoint("large-cp", store, log)`

**Then**:
  - Checkpoint loads successfully
  - Decompression happens transparently
  - Returns `Ok(ReplayState)` in <1s
  - Memory usage remains bounded

---

## Performance Tests

### test_performance_replay_1000_events_in_under_5_seconds
**Given**: Checkpoint with timestamp T0
**And**: Event log has 1000 events distributed over 1 hour after T0
**When**: `resume_from_checkpoint` is called
**Then**:
  - Operation completes in <5s
  - `events_replayed` is 1000
  - Memory usage is reasonable (<100MB)

---

### test_performance_checkpoint_load_time
**Given**: Checkpoint with 10MB compressed state data
**When**: `load_checkpoint` is called
**Then**:
  - Load completes in <100ms
  - Returns `Ok(Some(data, timestamp))`

---

### test_performance_event_log_query_time
**Given**: Event log with 10,000 total events
**And**: Query requests events after midpoint timestamp
**When**: `load_events_after` is called
**Then**:
  - Query completes in <500ms
  - Returns ~5000 events

---

## Mock Implementations for Testing

### MockCheckpointStore
```rust
struct MockCheckpointStore {
    checkpoint: Option<(CheckpointData, DateTime<Utc>)>,
    timestamp_valid: bool,
    load_error: Option<Error>,
    validate_error: Option<Error>,
}

impl CheckpointStore for MockCheckpointStore {
    fn load_checkpoint(&self, id: &CheckpointId) -> Result<Option<...>> {
        if let Some(err) = &self.load_error {
            return Err(err.clone());
        }
        Ok(self.checkpoint.clone())
    }

    fn validate_timestamp(&self, _id: &CheckpointId, _ts: DateTime<Utc>) -> Result<bool> {
        if let Some(err) = &self.validate_error {
            return Err(err.clone());
        }
        Ok(self.timestamp_valid)
    }
}
```

### MockEventLog
```rust
struct MockEventLog {
    events: Vec<EventMetadata>,
    load_error: Option<Error>,
}

impl EventLog for MockEventLog {
    fn load_events_after(&self, timestamp: DateTime<Utc>) -> Result<Vec<EventMetadata>> {
        if let Some(err) = &self.load_error {
            return Err(err.clone());
        }
        Ok(self.events.iter()
            .filter(|e| e.timestamp > timestamp)
            .cloned()
            .collect())
    }
}
```

---

## Test Organization

### Module Structure
```
tests/
  ├── happy_path/
  │   ├── test_returns_success_when_valid_checkpoint_exists.rs
  │   ├── test_creates_replay_state_with_zero_events.rs
  │   └── ...
  ├── error_paths/
  │   ├── test_returns_checkpoint_not_found.rs
  │   ├── test_returns_timestamp_mismatch.rs
  │   └── ...
  ├── edge_cases/
  │   ├── test_handles_empty_checkpoint_id.rs
  │   ├── test_handles_boundary_timestamp.rs
  │   └── ...
  ├── contract_verification/
  │   ├── test_preconditions.rs
  │   ├── test_postconditions.rs
  │   └── test_invariants.rs
  ├── integration/
  │   ├── scenario_1_successful_resume.rs
  │   ├── scenario_2_checkpoint_expired.rs
  │   └── ...
  └── performance/
      ├── test_performance_1000_events.rs
      └── test_performance_checkpoint_load.rs
```

---

## Test Execution Order

1. **Unit tests** (fast, isolated)
   - Happy path tests
   - Error path tests
   - Edge case tests
   - Contract verification tests

2. **Integration tests** (slower, realistic mocks)
   - Given-When-Then scenarios
   - End-to-end workflows

3. **Performance tests** (slowest, benchmarked)
   - 1000 events in <5s
   - Checkpoint load time
   - Event log query time

---

## Coverage Requirements

### Error Coverage
- [ ] `ResumeError::CheckpointNotFound` - 3 tests
- [ ] `ResumeError::TimestampMismatch` - 2 tests
- [ ] `ResumeError::InvalidCheckpoint` - 4 tests
- [ ] `ResumeError::EventLoadFailed` - 2 tests

### Contract Coverage
- [ ] All preconditions - 3 tests
- [ ] All postconditions - 4 tests
- [ ] All invariants - 4 tests

### Edge Case Coverage
- [ ] Empty inputs (empty ID, empty log)
- [ ] Boundary values (timestamp = first event)
- [ ] Extreme values (10,000 events, large checkpoint)
- [ ] Duplicate values (same timestamp)

### Scenario Coverage
- [ ] 7 Given-When-Then scenarios documented

---

## Test Naming Convention

All test names follow the pattern:
```
test_{behavior}_when_{condition}
```

Examples:
- `test_returns_success_when_valid_checkpoint_exists`
- `test_returns_checkpoint_not_found_when_checkpoint_id_does_not_exist`
- `test_handles_empty_event_log_after_checkpoint`

This makes tests self-documenting and easier to discover.

---

## Test Data Builders

Helper functions to create test data:

```rust
fn create_checkpoint(id: &str, ts: DateTime<Utc>) -> (CheckpointData, DateTime<Utc>) {
    (CheckpointData {
        state: vec![1, 2, 3],
        sequence_number: 1,
        compressed: false,
    }, ts)
}

fn create_event(id: &str, ts: DateTime<Utc>, seq: u64) -> EventMetadata {
    EventMetadata {
        event_id: id.to_string(),
        timestamp: ts,
        sequence_number: seq,
    }
}
```

---

## Assertion Helpers

Custom assertions for better error messages:

```rust
fn assert_checkpoint_not_found(result: &Result<ReplayState, ResumeError>, expected_id: &str) {
    assert!(result.is_err(), "Expected error but got success");
    let err = result.as_ref().unwrap_err();
    assert!(matches!(err, ResumeError::CheckpointNotFound { .. }),
        "Expected CheckpointNotFound, got: {:?}", err);
}

fn assert_events_replayed_count(state: &ReplayState, expected: u64) {
    assert_eq!(state.events_replayed, expected,
        "Expected {} events replayed, got {}", expected, state.events_replayed);
}
```

---

## Test Cleanup

All tests must:
- Clean up temporary files
- Reset mock state
- Not leave database connections open
- Be order-independent (can run in parallel)

---

## Notes

- Tests are written in Rust using built-in test framework
- Use `#[test]` attribute for unit tests
- Use `#[tokio::test]` if async operations are added later
- Property-based testing with `proptest` for invariants (optional)
- Benchmark with `criterion` for performance tests (optional)
