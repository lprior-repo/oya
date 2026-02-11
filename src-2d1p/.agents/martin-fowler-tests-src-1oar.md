# Martin Fowler Test Plan: Supervisor Checkpoint on Graceful Shutdown

## Test Philosophy

Tests are executable specifications that describe behavior unambiguously. Each test name should read like a requirement document. Tests use Given-When-Then structure to make intent explicit.

## Happy Path Tests

### test_checkpoint_created_when_shutdown_signal_received

**Given**:
- A running supervisor with 3 active children
- CheckpointManager is available and initialized
- ShutdownCoordinator is configured with checkpoint result channel

**When**:
- SIGTERM signal is received
- Supervisor enters SavingCheckpoints phase

**Then**:
- Checkpoint is created with unique checkpoint_id
- Checkpoint contains serialized supervisor state with all 3 children
- Checkpoint includes correct total_restarts count
- Checkpoint includes failure times within window
- CheckpointResult::success is sent to ShutdownCoordinator
- CheckpointResult includes component name "supervisor"
- CheckpointResult includes duration_ms < 25000 (25 second timeout)
- Shutdown continues to StoppingActors phase

### test_checkpoint_includes_all_children_metadata

**Given**:
- Supervisor with 2 children: "worker-1" (restart_count: 0) and "worker-2" (restart_count: 2)
- Child "worker-2" has args: `{"queue": "high_priority"}`
- Total restarts: 2

**When**:
- Graceful shutdown is initiated
- Checkpoint is created

**Then**:
- Serialized state includes both children
- Child snapshot for "worker-1" has restart_count: 0
- Child snapshot for "worker-2" has restart_count: 2
- Child snapshot for "worker-2" includes serialized args
- Total restarts in checkpoint equals 2
- All child names are preserved exactly

### test_checkpoint_serialization_includes_all_required_fields

**Given**:
- Supervisor in running state with any valid configuration

**When**:
- Checkpoint is created during shutdown
- Serialized JSON is extracted from CheckpointRecord

**Then**:
- JSON contains "config" field matching SupervisorConfig
- JSON contains "active_children" field (integer)
- JSON contains "total_restarts" field (integer)
- JSON contains "children" field (array)
- JSON contains "child_id_counter" field (integer)
- JSON contains "snapshot_time" field (ISO 8601 timestamp)
- JSON contains "shutdown_reason" field (string or null)

### test_checkpoint_id_is_unique_per_shutdown

**Given**:
- Supervisor performs first shutdown and creates checkpoint
- Supervisor restarts and performs second shutdown

**When**:
- Checkpoints are created during both shutdowns

**Then**:
- First checkpoint has format `cp-{timestamp1}-{sequence}`
- Second checkpoint has format `cp-{timestamp2}-{sequence}`
- timestamp1 != timestamp2 (at least 1ms apart)
- Both checkpoint IDs are unique strings
- Checkpoint IDs parse correctly to timestamp and sequence

### test_checkpoint_result_sent_to_coordinator

**Given**:
- ShutdownCoordinator with active checkpoint result channel
- Supervisor with CheckpointManager available

**When**:
- Checkpoint is created successfully
- CheckpointResult is sent

**Then**:
- ShutdownCoordinator receives exactly one CheckpointResult
- CheckpointResult.component equals "supervisor"
- CheckpointResult.success equals true
- CheckpointResult.duration_ms is a reasonable value (1-25000)
- CheckpointResult.error is None
- Channel remains open after sending

### test_shutdown_continues_after_successful_checkpoint

**Given**:
- Supervisor in shutdown sequence
- Checkpoint created successfully

**When**:
- CheckpointResult is sent to coordinator

**Then**:
- Supervisor state transitions to StoppingActors
- Child actors receive stop signals
- Supervisor completes shutdown within 30 seconds
- No errors are logged during transition

## Error Path Tests

### test_returns_error_when_checkpoint_manager_unavailable

**Given**:
- Supervisor with CheckpointManager set to None
- ShutdownCoordinator is available

**When**:
- Graceful shutdown is initiated
- Checkpoint creation is attempted

**Then**:
- Function returns `Err(SupervisorCheckpointError::CheckpointManagerUnavailable)`
- CheckpointResult::failure is sent to coordinator with error message
- Error message includes "CheckpointManager not available"
- Shutdown continues despite checkpoint failure
- Supervisor transitions to StoppingActors phase

### test_returns_error_when_state_serialization_fails

**Given**:
- Supervisor with corrupt state (e.g., invalid child args)
- CheckpointManager is available

**When**:
- Checkpoint serialization is attempted

**Then**:
- Function returns `Err(SupervisorCheckpointError::SerializationFailed)`
- Error.source contains details about serialization failure
- CheckpointResult::failure is sent to coordinator
- CheckpointResult.error includes "serialization" or "JSON" context
- Shutdown continues without aborting

### test_returns_error_when_checkpoint_persistence_fails

**Given**:
- Supervisor with valid state
- CheckpointManager available but database is unreachable

**When**:
- Checkpoint persistence is attempted

**Then**:
- Function returns `Err(SupervisorCheckpointError::CheckpointPersistenceFailed)`
- Error.source contains underlying PersistenceError
- CheckpointResult::failure is sent to coordinator
- Error is logged with database connection details
- Shutdown continues without checkpoint

### test_returns_error_when_checkpoint_timeout_exceeded

**Given**:
- Supervisor with valid state
- CheckpointManager available but very slow (>25 seconds)

**When**:
- Checkpoint creation is attempted
- 25 second timeout elapses

**Then**:
- Function returns `Err(SupervisorCheckpointError::CheckpointTimeout)`
- Error.duration_ms reflects time attempted (>=25000)
- CheckpointResult::failure is sent to coordinator
- Error message includes "timeout" and "25 seconds"
- Shutdown is forced to continue (does not hang)

### test_returns_error_when_result_channel_closed

**Given**:
- Supervisor with valid state and CheckpointManager
- ShutdownCoordinator checkpoint result channel is closed/dropped

**When**:
- Checkpoint is created successfully
- Attempting to send CheckpointResult

**Then**:
- Function returns `Err(SupervisorCheckpointError::ResultChannelClosed)`
- Checkpoint is still persisted to database
- Error is logged but does not cause panic
- Shutdown continues (cannot report but must proceed)

### test_handles_corrupt_supervisor_state_gracefully

**Given**:
- Supervisor with invalid state (e.g., negative restart counts, invalid child refs)

**When**:
- Checkpoint creation is attempted

**Then**:
- Function returns `Err(SupervisorCheckpointError::InvalidState)`
- Error.reason describes the invalid state
- CheckpointResult::failure is sent to coordinator
- No panic or unwrap occurs
- Shutdown continues safely

## Edge Case Tests

### test_handles_empty_supervisor_no_children

**Given**:
- Supervisor with zero children (empty children HashMap)
- CheckpointManager available

**When**:
- Graceful shutdown is initiated
- Checkpoint is created

**Then**:
- Checkpoint is created successfully
- Serialized state shows active_children: 0
- Serialized state shows empty children array: []
- CheckpointResult::success is sent
- No errors occur

### test_handles_supervisor_with_max_children

**Given**:
- Supervisor with maximum expected children (e.g., 1000 children)
- CheckpointManager available

**When**:
- Checkpoint is created during shutdown

**Then**:
- Checkpoint is created successfully
- All 1000 children are included in snapshot
- Serialization completes within timeout
- CheckpointResult::success is sent
- Memory usage remains reasonable

### test_handles_rapid_shutdown_after_restart

**Given**:
- Supervisor that just restarted a child (restart_count incremented)
- Shutdown is triggered immediately after restart

**When**:
- Checkpoint is created

**Then**:
- Checkpoint includes updated restart_count
- Checkpoint includes last_restart timestamp
- State is consistent (no race conditions)
- CheckpointResult::success is sent

### test_handles_shutdown_with_failure_times_in_window

**Given**:
- Supervisor with 5 failures within the last 60 seconds
- failure_times vector has 5 entries

**When**:
- Checkpoint is created during shutdown

**Then**:
- Checkpoint includes failure_count_in_window: 5
- Old failures (outside window) are excluded
- Recent failures (within window) are included
- CheckpointResult::success is sent

### test_handles_multiple_rapid_shutdown_signals

**Given**:
- Supervisor running normally
- SIGTERM received

**When**:
- Second SIGTERM received during checkpoint creation

**Then**:
- Second shutdown signal is ignored (no-op)
- Checkpoint creation continues uninterrupted
- Only one checkpoint is created
- Shutdown proceeds normally

### test_handles_checkpoint_with_all_child_stopped

**Given**:
- Supervisor where all children are already stopped (exited)
- Shutdown is triggered

**When**:
- Checkpoint is created

**Then**:
- Checkpoint includes all children (even stopped ones)
- Child snapshots include restart history
- active_children count reflects stopped state (0)
- CheckpointResult::success is sent

### test_handles_serialization_of_complex_child_args

**Given**:
- Supervisor with child having complex nested args:
  ```json
  {
    "config": {
      "nested": {
        "deep": {
          "value": [1, 2, 3]
        }
      }
    },
    "metadata": {"key": "value"}
  }
  ```

**When**:
- Checkpoint is created

**Then**:
- Serialization succeeds
- Child args are preserved exactly
- Nested structure is intact in checkpoint
- CheckpointResult::success is sent

### test_handles_zero_restart_count

**Given**:
- Supervisor that has never restarted any child
- total_restarts: 0, all child restart_counts: 0

**When**:
- Checkpoint is created

**Then**:
- Checkpoint shows total_restarts: 0
- All children have restart_count: 0
- No last_restart timestamps (all None)
- CheckpointResult::success is sent

### test_handles_maximum_restart_count

**Given**:
- Supervisor with u32::MAX restarts (edge case)

**When**:
- Checkpoint is created

**Then**:
- Serialization succeeds (no overflow)
- Restart count is preserved as u32::MAX
- CheckpointResult::success is sent
- No saturating_add overflow occurs

## Contract Verification Tests

### test_precondition_shutdown_signal_received

**Given**:
- Supervisor in Running state
- No shutdown signal received

**When**:
- Attempt to create shutdown checkpoint

**Then**:
- Function returns error or panics (precondition violation)
- Error message indicates "shutdown not initiated"
- No checkpoint is created

### test_precondition_checkpoint_manager_available

**Given**:
- Supervisor in ShuttingDown state
- CheckpointManager is None

**When**:
- Checkpoint creation is attempted

**Then**:
- Function returns `CheckpointManagerUnavailable` error
- CheckpointResult::failure is sent
- Error is logged appropriately

### test_postcondition_checkpoint_id_unique

**Given**:
- Supervisor creates checkpoint

**When**:
- Checkpoint is created

**Then**:
- checkpoint_id is unique string
- Format matches `cp-{timestamp}-{sequence}`
- checkpoint_id is not empty
- checkpoint_id does not contain invalid characters

### test_postcondition_checkpoint_persisted

**Given**:
- Checkpoint is created successfully

**When**:
- Query CheckpointManager for the checkpoint

**Then**:
- Checkpoint exists in database
- Checkpoint has matching checkpoint_id
- Checkpoint has matching scheduler_state JSON
- Checkpoint has correct event_sequence
- Checkpoint has valid created_at timestamp

### test_postcondition_result_sent_to_coordinator

**Given**:
- Checkpoint creation completes (success or failure)

**When**:
- ShutdownCoordinator checkpoint result channel is checked

**Then**:
- Exactly one CheckpointResult was received
- CheckpointResult.component is "supervisor"
- CheckpointResult.success matches actual outcome
- CheckpointResult.error is Some if failed, None if succeeded
- CheckpointResult.duration_ms is positive

### test_invariant_no_spawning_during_shutdown

**Given**:
- Supervisor in SavingCheckpoints phase
- Checkpoint creation in progress

**When**:
- SpawnChild message is received

**Then**:
- SpawnChild is rejected or ignored
- No new child is created
- Error is returned to SpawnChild caller
- Checkpoint creation continues unaffected

### test_invariant_checkpoint_atomicity

**Given**:
- Checkpoint creation in progress
- Database connection fails mid-operation

**When**:
- Checkpoint creation is observed

**Then**:
- Either checkpoint is fully saved OR not saved at all
- No partial/corrupt checkpoint exists
- CheckpointResult reflects actual outcome
- No inconsistent state in database

### test_invariant_timeout_compliance

**Given**:
- Supervisor initiates shutdown
- Checkpoint creation starts

**When**:
- 25 seconds elapse

**Then**:
- Checkpoint creation is terminated
- Timeout error is returned
- CheckpointResult::failure is sent
- Shutdown continues (does not hang)

### test_invariant_state_consistency

**Given**:
- Supervisor with specific state (children, restarts, failures)

**When**:
- Checkpoint is created
- State is inspected immediately after

**Then**:
- No children were added or removed during checkpoint
- Restart counts unchanged
- Child ID counter unchanged
- State is identical to pre-checkpoint state

## Integration Tests

### test_end_to_end_shutdown_with_checkpoint

**Given**:
- Full system running (supervisor, children, checkpoint manager, shutdown coordinator)
- 3 active children with various states

**When**:
- SIGTERM is received
- Full shutdown sequence executes

**Then**:
1. Shutdown signal received by supervisor
2. Supervisor transitions to Initiating phase
3. Checkpoint is created with all state
4. CheckpointResult sent to coordinator
5. Supervisor transitions to StoppingActors
6. All children receive stop signals
7. Children exit cleanly
8. Supervisor transitions to Complete
9. Entire sequence completes within 30 seconds
10. Checkpoint is queryable from database

### test_checkpoint_restoration_integration

**Given**:
- Supervisor created checkpoint during shutdown
- System is restarted

**When**:
- Supervisor queries for latest checkpoint
- Checkpoint is restored

**Then**:
- Checkpoint contains all supervisor state
- Children can be respawned from checkpoint data
- Restart counts are preserved
- Child ID counter is restored
- State matches pre-shutdown state

**Note**: This test may be a placeholder for future restoration feature.

## Performance Tests

### test_checkpoint_creation_performance_within_bounds

**Given**:
- Supervisor with 100 children
- Typical state size (~10KB JSON)

**When**:
- Checkpoint is created 100 times

**Then**:
- Median checkpoint time < 100ms
- 95th percentile < 500ms
- 99th percentile < 1000ms
- Maximum < 5000ms (well below 25s timeout)

### test_checkpoint_size_reasonable

**Given**:
- Supervisor with 100 children

**When**:
- Checkpoint is created
- Serialized JSON size is measured

**Then**:
- JSON size < 1MB
- Size scales linearly with child count
- No excessive metadata bloat

## Stress Tests

### test_concurrent_shutdown_and_checkpoint

**Given**:
- Supervisor under load (processing many messages)
- Shutdown signal received

**When**:
- Checkpoint creation competes with message processing

**Then**:
- Checkpoint creation succeeds
- No race conditions
- State is consistent
- Shutdown completes within timeout

### test_checkpoint_during_high_failure_rate

**Given**:
- Supervisor with children failing rapidly (meltdown imminent)
- Shutdown triggered during meltdown

**When**:
- Checkpoint is created

**Then**:
- Checkpoint captures failure_times accurately
- Checkpoint includes meltdown context
- CheckpointResult::success or ::failure (either acceptable)
- Shutdown completes without hanging

## Test Coverage Requirements

### Code Coverage
- **Line Coverage**: >= 95% for checkpoint creation logic
- **Branch Coverage**: 100% for all error paths
- **Function Coverage**: 100% of public checkpoint functions

### Scenario Coverage
- All happy paths covered
- All error variants tested
- All edge cases explored
- All invariants verified
- At least one end-to-end integration test

### Error Path Coverage
- Every `SupervisorCheckpointError` variant has at least one test
- Every error condition has a corresponding test
- Error recovery is verified for each error type
