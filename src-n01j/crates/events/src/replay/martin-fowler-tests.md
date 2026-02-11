# Martin Fowler Test Plan

## Happy Path Tests

- `test_apply_first_event_for_bead_creates_state_entry`
  - Given: Empty state and context
  - When: BeadCreated event is applied
  - Then: State contains the new bead with correct spec
  - And: Context tracks the event as last applied

- `test_apply_state_changed_event_transitions_state`
  - Given: State with bead in Pending state
  - When: StateChanged event (Pending -> Scheduled) is applied
  - Then: Bead state is updated to Scheduled
  - And: Context tracks the new event

- `test_apply_completed_event_marks_bead_terminal`
  - Given: State with active bead
  - When: Completed event is applied
  - Then: Bead is marked complete with result
  - And: No further state changes are allowed

- `test_apply_events_in_sequence_builds_state`
  - Given: Empty state and context
  - When: Multiple events for same bead are applied in order
  - Then: State reflects all applied events
  - And: Context tracks last event
  - And: State version reflects number of applied events

- `test_apply_events_for_multiple_beads_independently`
  - Given: Empty state and context
  - When: Events for different beads are interleaved
  - Then: Each bead's state is correct
  - And: Context tracks last event per bead independently

- `test_apply_phase_completed_event_updates_progress`
  - Given: State with active bead
  - When: PhaseCompleted event is applied
  - Then: Bead's phase progress is updated
  - And: Phase output is stored

## Error Path Tests

- `test_apply_event_out_of_order_returns_error`
  - Given: Context with last event ID "01H..."
  - When: Event with earlier ULID "01H..." is applied
  - Then: Returns ApplyError::OutOfOrder
  - And: State is unchanged (atomicity)
  - And: Context is unchanged

- `test_apply_event_with_earlier_timestamp_returns_error`
  - Given: Context with last event at time T
  - When: Event with timestamp T-1 is applied
  - Then: Returns ApplyError::TimestampInconsistent
  - And: State is unchanged

- `test_apply_event_for_unknown_bead_returns_error`
  - Given: Empty state
  - When: StateChanged event for non-existent bead is applied
  - Then: Returns ApplyError::BeadNotFound
  - And: No bead is created

- `test_apply_invalid_state_transition_returns_error`
  - Given: State with bead in Complete state
  - When: StateChanged event (Complete -> Pending) is applied
  - Then: Returns ApplyError::InvalidTransition
  - And: Bead remains in Complete state

- `test_apply_events_stops_on_first_error`
  - Given: Empty state and context
  - When: Event slice with valid event then invalid event
  - Then: First event is applied
  - And: Second event returns error
  - And: No subsequent events are applied
  - And: State reflects only first event

- `test_apply_event_with_mismatched_from_state_returns_error`
  - Given: State with bead in Scheduled state
  - When: StateChanged event with from=Pending is applied
  - Then: Returns ApplyError::InvalidTransition
  - And: Bead remains in Scheduled state

## Edge Case Tests

- `test_apply_empty_events_slice_returns_success`
  - Given: Any state and context
  - When: Empty event slice is applied
  - Then: Returns Ok(())
  - And: State is unchanged

- `test_apply_event_with_same_ulid_as_last_returns_error`
  - Given: Context with last event ID "01H..."
  - When: Event with identical ULID is applied
  - Then: Returns ApplyError::OutOfOrder (not strictly greater)

- `test_apply_events_with_single_event_succeeds`
  - Given: Empty state and context
  - When: Single event slice is applied
  - Then: Event is applied successfully
  - And: State and context are updated

- `test_apply_multiple_events_same_timestamp_succeeds_if_ulids_ordered`
  - Given: Context with last event at time T
  - When: Multiple events with same timestamp T but increasing ULIDs
  - Then: All events are applied successfully
  - And: No timestamp inconsistency error

- `test_apply_event_to_just_completed_bead_fails`
  - Given: State with bead in Complete state
  - When: Any state change event is applied
  - Then: Returns ApplyError::InvalidTransition
  - And: Bead remains Complete

- `test_apply_created_event_after_other_events_for_same_bead_fails`
  - Given: State with existing bead
  - When: Another Created event for same bead is applied
  - Then: Returns appropriate error (InvalidTransition or BeadNotFound logic)

- `test_context_tracks_multiple_beads_correctly`
  - Given: Context with 1000 tracked beads
  - When: Event is applied for bead #1001
  - Then: Context tracks all 1001 beads
  - And: No cross-bead contamination

- `test_apply_events_with_maximum_boundary_ulid_values`
  - Given: Context with last event near ULID max value
  - When: Event with ULID near max is applied
  - Then: Handles correctly without overflow
  - And: Ordering check succeeds if valid

## Contract Verification Tests

- `test_precondition_event_must_be_wellformed`
  - Verify: Malformed events (missing required fields) are rejected
  - Method: Apply event with null/invalid fields
  - Expected: ApplyError::Internal or appropriate validation error

- `test_precondition_state_must_be_initialized`
  - Verify: Uninitialized state is detected
  - Method: Attempt to apply event to corrupted state
  - Expected: ApplyError::Internal or validation error

- `test_postcondition_state_unchanged_on_error`
  - Verify: Failed application doesn't mutate state
  - Method: Capture state hash before/after failed apply
  - Expected: Hashes identical

- `test_postcondition_context_updated_on_success`
  - Verify: Successful apply updates context
  - Method: Check last_event after successful apply
  - Expected: Context contains applied event

- `test_postcondition_returns_ok_on_success`
  - Verify: Success path returns Ok(())
  - Method: Apply valid event
  - Expected: Ok(())

- `test_postcondition_returns_err_with_specific_variant_on_failure`
  - Verify: Each error path returns specific ApplyError variant
  - Method: Trigger each error condition
  - Expected: Correct ApplyError variant with context

- `test_invariant_event_ulids_monotonic_for_bead`
  - Verify: Applied event ULIDs strictly increase
  - Method: Apply multiple events, check ULIDs
  - Expected: ULID[n+1] > ULID[n] for all n

- `test_invariant_timestamps_non_decreasing_for_bead`
  - Verify: Applied event timestamps never decrease
  - Method: Apply multiple events, check timestamps
  - Expected: Timestamp[n+1] >= Timestamp[n] for all n

- `test_invariant_state_version_increments`
  - Given: State with version tracking (if implemented)
  - When: Events are applied
  - Then: Version increments by 1 for each event
  - And: Version is always >= number of applied events

## Given-When-Then Scenarios

### Scenario 1: Happy path - bead lifecycle from creation to completion

**Given**:
- Empty state (AllBeadsState)
- Empty context (ApplyContext)
- A sequence of 5 events for bead "src-d02f":
  1. Created event (spec: "Implement state application")
  2. StateChanged (Pending -> InProgress)
  3. PhaseCompleted (rust-contract)
  4. StateChanged (InProgress -> Complete)
  5. Completed event

**When**:
- `apply_events(&mut state, &events, &mut context)` is called

**Then**:
- State contains exactly 1 bead with ID "src-d02f"
- Bead state is Complete
- Bead has 1 phase completed: rust-contract
- Bead has result stored from Completed event
- Context.last_events["src-d02f"].event_id == events[4].event_id
- Context has exactly 1 entry
- Return value is Ok(())
- No errors occurred during application

### Scenario 2: Error path - out-of-order event detection

**Given**:
- State with bead "src-d02f" in Scheduled state
- Context with last event ULID "01HXXXXXXXXX01"
- New event with ULID "01HXXXXXXXXX00" (earlier)

**When**:
- `apply_event(&mut state, &event, &mut context)` is called

**Then**:
- Return value is Err(ApplyError::OutOfOrder {
    bead_id: "src-d02f",
    event_id: "01HXXXXXXXXX00",
    expected: "01HXXXXXXXXX01",
    actual: "01HXXXXXXXXX00"
  })
- State still has bead "src-d02f" in Scheduled state (unchanged)
- Context.last_events["src-d02f"].event_id still equals "01HXXXXXXXXX01" (unchanged)
- No other state mutations occurred
- Atomicity preserved

### Scenario 3: Error path - invalid state transition

**Given**:
- State with bead "src-d02f" in Complete state
- StateChanged event with from=Complete, to=Pending

**When**:
- `apply_event(&mut state, &event, &mut context)` is called

**Then**:
- Return value is Err(ApplyError::InvalidTransition {
    bead_id: "src-d02f",
    from: BeadState::Complete,
    to: BeadState::Pending
  })
- Bead "src-d02f" remains in Complete state
- Context unchanged
- Transition validation prevents state corruption

### Scenario 4: Edge case - multiple beads with independent state

**Given**:
- Empty state and context
- Events for 3 different beads interleaved:
  1. Created bead-A
  2. Created bead-B
  3. StateChanged bead-A (Pending -> InProgress)
  4. Created bead-C
  5. StateChanged bead-B (Pending -> Scheduled)
  6. Completed bead-A
  7. StateChanged bead-C (Pending -> Failed)

**When**:
- `apply_events(&mut state, &events, &mut context)` is called

**Then**:
- State contains exactly 3 beads: A, B, C
- Bead A is Complete
- Bead B is Scheduled
- Bead C is Failed
- Context has 3 entries, one per bead
- Context.last_events[bead-A].event_id == events[5].event_id
- Context.last_events[bead-B].event_id == events[4].event_id
- Context.last_events[bead-C].event_id == events[6].event_id
- No cross-bead state contamination

### Scenario 5: Determinism verification

**Given**:
- Empty state S1 and context C1
- Empty state S2 and context C2
- Same ordered event sequence E (10 events)

**When**:
- `apply_events(&mut S1, &E, &mut C1)` is called
- `apply_events(&mut S2, &E, &mut C2)` is called

**Then**:
- S1 == S2 (states are identical)
- C1 == C2 (contexts are identical)
- Proves: Same events + same order = same state (deterministic)

### Scenario 6: Fail-fast on sequence error

**Given**:
- Empty state and context
- Event sequence with error at position 3:
  1. Valid Created event
  2. Valid StateChanged event
  3. Invalid event (out of order)
  4. Valid StateChanged event
  5. Valid Completed event

**When**:
- `apply_events(&mut state, &events, &mut context)` is called

**Then**:
- Events 1 and 2 are applied successfully
- Event 3 returns Err(ApplyError::OutOfOrder)
- Events 4 and 5 are NOT applied
- State reflects only events 1 and 2
- Processing stops immediately at error
- No partial application of event 3

## Performance Tests (Optional)

- `test_apply_thousand_events_completes_quickly`
  - Verify: 1000 events applied in < 100ms
  - Method: Benchmark with 1000 events
  - Expected: Completes in reasonable time

- `test_context_lookup_scales_linearly`
  - Verify: Context lookup doesn't degrade with many beads
  - Method: Apply events to 10,000 beads, measure lookup time
  - Expected: O(1) lookup (HashMap guarantees)

## Integration Tests

- `test_replay_integration_with_loader`
  - Load events from event store
  - Apply to state
  - Verify state consistency

- `test_replay_with_checkpoint_restore`
  - Load checkpoint
  - Apply events after checkpoint
  - Verify correct state reconstruction
