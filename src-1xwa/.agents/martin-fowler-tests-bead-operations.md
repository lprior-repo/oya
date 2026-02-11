# Martin Fowler Test Plan: IPC Worker Bead Operations

## Happy Path Tests

### execute_start_bead
- `test_start_bead_succeeds_when_bead_in_pending_state`
- `test_start_bead_succeeds_when_bead_in_ready_state`
- `test_start_bead_is_idempotent_when_bead_already_running`
- `test_start_bead_sets_started_at_timestamp`
- `test_start_bead_returns_ack_message_on_success`

### execute_cancel_bead
- `test_cancel_bead_succeeds_when_bead_is_running`
- `test_cancel_bead_succeeds_when_bead_is_pending`
- `test_cancel_bead_is_idempotent_when_bead_already_cancelled`
- `test_cancel_bead_sets_completed_at_timestamp`
- `test_cancel_bead_returns_ack_message_on_success`

### execute_retry_bead
- `test_retry_bead_succeeds_when_bead_in_failed_state`
- `test_retry_bead_increments_retry_count`
- `test_retry_bead_clears_error_message`
- `test_retry_bead_clears_started_at_timestamp`
- `test_retry_bead_clears_completed_at_timestamp`
- `test_retry_bead_returns_ack_message_on_success`

---

## Error Path Tests

### execute_start_bead
- `test_start_bead_returns_not_found_when_bead_does_not_exist`
- `test_start_bead_returns_invalid_state_when_bead_completed`
- `test_start_bead_returns_invalid_state_when_bead_failed`
- `test_start_bead_returns_invalid_state_when_bead_cancelled`
- `test_start_bead_returns_internal_error_when_store_not_initialized`
- `test_start_bead_maps_persistence_errors_to_actor_errors`

### execute_cancel_bead
- `test_cancel_bead_returns_not_found_when_bead_does_not_exist`
- `test_cancel_bead_returns_invalid_state_when_bead_already_completed`
- `test_cancel_bead_returns_invalid_state_when_bead_already_failed`
- `test_cancel_bead_returns_internal_error_when_store_not_initialized`

### execute_retry_bead
- `test_retry_bead_returns_not_found_when_bead_does_not_exist`
- `test_retry_bead_returns_invalid_state_when_bead_is_pending`
- `test_retry_bead_returns_invalid_state_when_bead_is_running`
- `test_retry_bead_returns_invalid_state_when_bead_is_completed`
- `test_retry_bead_returns_invalid_state_when_bead_is_cancelled`
- `test_retry_bead_returns_internal_error_when_store_not_initialized`

---

## Edge Case Tests

### State Transition Boundaries
- `test_all_non_terminal_states_can_transition_to_running`
- `test_all_non_terminal_states_can_transition_to_cancelled`
- `test_only_failed_state_can_transition_to_ready_via_retry`
- `test_terminal_states_block_running_transition`
- `test_terminal_states_block_cancel_transition`

### Timestamp Semantics
- `test_start_bead_preserves_existing_started_at_on_idempotent_call`
- `test_cancel_bead_preserves_existing_completed_at_on_idempotent_call`
- `test_retry_bead_resets_all_execution_timestamps`

### Empty/Invalid Input
- `test_start_bead_rejects_empty_bead_id`
- `test_cancel_bead_rejects_empty_bead_id`
- `test_retry_bead_rejects_empty_bead_id`

### Retry Count Tracking
- `test_retry_bead_increments_count_on_multiple_retries`
- `test_retry_bead_preserves_count_on_start_after_retry`

---

## Contract Verification Tests

### Preconditions
- `test_precondition_store_required_for_start_bead`
- `test_precondition_store_required_for_cancel_bead`
- `test_precondition_store_required_for_retry_bead`

### Postconditions
- `test_postcondition_running_state_set_after_start`
- `test_postcondition_cancelled_state_set_after_cancel`
- `test_postcondition_ready_state_set_after_retry`
- `test_postcondition_timestamps_updated_correctly`

### Invariants
- `test_invariant_state_transitions_are_valid`
- `test_invariant_updated_at_always_changes`
- `test_invariant_retry_count_never_decrements`

---

## Given-When-Then Scenarios

### Scenario 1: Start Pending Bead
**Given** an IPC worker with initialized store
**And** a bead exists in Pending state
**When** execute_start_bead is called with the bead ID
**Then**:
- The bead state transitions to Running
- The started_at timestamp is set to current time
- The updated_at timestamp is updated
- A StateChanged event is published to EventBus
- An Ack HostMessage is returned

### Scenario 2: Cancel Running Bead
**Given** an IPC worker with initialized store
**And** a bead exists in Running state
**When** execute_cancel_bead is called with the bead ID
**Then**:
- The bead state transitions to Cancelled
- The completed_at timestamp is set
- The updated_at timestamp is updated
- A StateChanged event is published to EventBus
- An Ack HostMessage is returned

### Scenario 3: Retry Failed Bead
**Given** an IPC worker with initialized store
**And** a bead exists in Failed state with retry_count=0
**And** the bead has error_message="division by zero"
**When** execute_retry_bead is called with the bead ID
**Then**:
- The bead state transitions to Ready
- The retry_count is incremented to 1
- The error_message is cleared to None
- The started_at timestamp is cleared
- The completed_at timestamp is cleared
- The updated_at timestamp is updated
- A StateChanged event is published to EventBus
- An Ack HostMessage is returned

### Scenario 4: Idempotent Start on Running Bead
**Given** an IPC worker with initialized store
**And** a bead exists in Running state with started_at=T1
**When** execute_start_bead is called with the bead ID
**Then**:
- The bead state remains Running
- The started_at timestamp remains T1 (unchanged)
- No error is returned
- An Ack HostMessage is returned

### Scenario 5: Invalid Transition from Completed
**Given** an IPC worker with initialized store
**And** a bead exists in Completed state
**When** execute_start_bead is called with the bead ID
**Then**:
- The bead state remains Completed
- An InvalidStateTransition error is returned
- The error message indicates current and requested states

### Scenario 6: Store Not Initialized
**Given** an IPC worker without store (store is None)
**When** execute_start_bead is called with any bead ID
**Then**:
- An Internal error is returned
- The error message indicates "Store not initialized"

### Scenario 7: Multiple Retries Increment Count
**Given** an IPC worker with initialized store
**And** a bead exists in Failed state with retry_count=2
**When** execute_retry_bead is called with the bead ID
**Then**:
- The bead state transitions to Ready
- The retry_count is incremented to 3
- An Ack HostMessage is returned

---

## Test Helpers

### Mock Store Expectations
```rust
// For not found scenarios
store.get_bead("non-existent") → Err(PersistenceError::NotFound)

// For pending bead scenarios
store.get_bead("bead-1") → Ok(BeadRecord {
    bead_id: "bead-1",
    state: BeadState::Pending,
    started_at: None,
    ...
})

// For state update scenarios
store.update_bead_state("bead-1", BeadState::Running) → Ok(updated_record)
```

### Assertion Helpers
```rust
fn assert_ack_message(result: &Result<HostMessage, ActorError>) {
    match result {
        Ok(HostMessage::Ack { .. }) => (),
        other => panic!("Expected Ack, got {:?}", other),
    }
}

fn assert_bead_not_found_error(result: &Result<HostMessage, ActorError>, bead_id: &str) {
    match result {
        Err(ActorError::BeadNotFound(id)) => assert_eq!(id, bead_id),
        other => panic!("Expected BeadNotFound, got {:?}", other),
    }
}

fn assert_invalid_state_error(result: &Result<HostMessage, ActorError>) {
    match result {
        Err(ActorError::InvalidStateTransition(_)) => (),
        other => panic!("Expected InvalidStateTransition, got {:?}", other),
    }
}
```

---

## Coverage Targets

| Category | Target Tests | Minimum Pass |
|----------|-------------|--------------|
| Happy Path | 15 | 15 |
| Error Path | 18 | 18 |
| Edge Cases | 15 | 15 |
| Contract Verification | 10 | 10 |
| Integration Scenarios | 7 | 7 |
| **Total** | **65** | **65** |
