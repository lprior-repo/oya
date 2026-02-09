# Martin Fowler Test Plan

## Happy Path Tests

### State Creation
- **test_default_state**: ReplayState::default() returns Uninitialized
- **test_uninitialized_description**: Uninitialized state description is "Not started"

### Valid State Transitions
- **test_start_loading_from_uninitialized**: Transition Uninitialized → Loading succeeds
- **test_start_replaying_from_loading**: Transition Loading → Replaying succeeds with events_total
- **test_update_progress_while_replaying**: Update events_processed in Replaying state
- **test_complete_from_replaying**: Transition Replaying → Complete succeeds
- **test_fail_from_any_state**: Fail transition works from Uninitialized, Loading, and Replaying

### Full Lifecycle
- **test_successful_replay_lifecycle**: Complete happy path through all states
  - Given: Uninitialized state
  - When: Start loading → start replaying → update progress → complete
  - Then: All transitions succeed, final state is Complete and terminal

## Error Path Tests

### Invalid Transitions from Uninitialized
- **test_cannot_start_loading_from_loading**: Loading → Loading fails (already loading)
- **test_cannot_start_replaying_from_uninitialized**: Uninitialized → Replaying fails (skipped loading)
- **test_cannot_complete_from_uninitialized**: Uninitialized → Complete fails (skipped replaying)

### Invalid Transitions from Loading
- **test_cannot_start_loading_from_complete**: Complete → Loading fails (terminal state)
- **test_cannot_start_loading_from_failed**: Failed → Loading fails (terminal state)
- **test_cannot_complete_from_loading**: Loading → Complete fails (not replaying)

### Invalid Transitions from Replaying
- **test_cannot_start_loading_from_replaying**: Replaying → Loading fails (wrong direction)
- **test_cannot_start_replaying_from_replaying**: Replaying → Replaying fails (already replaying)
- **test_cannot_update_progress_from_loading**: Loading → update_progress fails (wrong state)

### Invalid Transitions from Terminal States
- **test_cannot_complete_from_complete**: Complete → Complete fails (already terminal)
- **test_cannot_complete_from_failed**: Failed → Complete fails (already terminal)
- **test_cannot_update_progress_from_complete**: Complete → update_progress fails (terminal)

### Error Content Verification
- **test_invalid_state_error_contains_context**: InvalidState error includes current and attempted state names

## Edge Case Tests

### Zero and Boundary Values
- **test_replay_progress_zero_total**: ReplayProgress with 0 events_total shows 100% complete
- **test_events_processed_equals_total**: Boundaries where events_processed == events_total
- **test_events_processed_exceeds_total**: Edge case where processed might exceed total (graceful handling)

### State Queries
- **test_is_terminal**: Correctly identifies Complete and Failed as terminal
- **test_is_active**: Correctly identifies Loading and Replaying as active
- **test_description**: All state variants return correct descriptions

### Progress Tracking
- **test_update_progress_preserves_total**: Updating progress maintains events_total
- **test_progress_percentage_calculation**: Percentage is correctly calculated as (processed/total) * 100

## Contract Verification Tests

### Precondition: start_loading requires Uninitialized
- **test_precondition_start_loading_from_uninitialized**: Only Uninitialized can start loading
- **test_precondition_start_loading_fails_from_other_states**: All other states fail

### Precondition: start_replaying requires Loading
- **test_precondition_start_replaying_from_loading**: Only Loading can start replaying
- **test_precondition_start_replaying_requires_events_total**: Must provide events_total

### Precondition: update_progress requires Replaying
- **test_precondition_update_progress_from_replaying**: Only Replaying can update progress
- **test_precondition_update_progress_rejects_other_states**: All other states fail

### Precondition: complete requires Replaying
- **test_precondition_complete_from_replaying**: Only Replaying can complete
- **test_precondition_complete_fails_from_non_replaying**: All other states fail

### Postcondition: Successful transitions return new state
- **test_postcondition_transition_returns_new_state**: All successful transitions return Ok with new state
- **test_postcondition_new_state_has_correct_variant**: State variant matches expected transition

### Invariant: Terminal states cannot transition
- **test_invariant_complete_is_terminal**: Complete state rejects all transitions except fail
- **test_invariant_failed_is_terminal**: Failed state rejects all transitions except fail

### Invariant: Active states allow specific transitions
- **test_invariant_loading_allows_replaying**: Loading allows transition to Replaying
- **test_invariant_replaying_allows_progress_and_complete**: Replaying allows update_progress and complete

## Given-When-Then Scenarios

### Scenario 1: Successful Replay Lifecycle
**Given**: Uninitialized state
**When**:
  1. Call start_loading()
  2. Call start_replaying(100)
  3. Call update_progress(50)
  4. Call update_progress(100)
  5. Call complete()
**Then**:
  - All transitions succeed (return Ok)
  - State progresses: Uninitialized → Loading → Replaying → Replaying → Complete
  - Final state is Complete { events_processed: 100 }
  - is_terminal() returns true on final state

### Scenario 2: Failure During Loading
**Given**: Uninitialized state
**When**:
  1. Call start_loading()
  2. Call fail("disk full")
**Then**:
  - State becomes Failed { error: "disk full" }
  - is_terminal() returns true
  - Any subsequent transition (except fail) returns Error

### Scenario 3: Failure During Replaying
**Given**: Replaying state with 50/100 events processed
**When**:
  1. Call fail("projection error")
**Then**:
  - State becomes Failed { error: "projection error" }
  - Progress (50 events) is lost (state does not preserve it)
  - is_terminal() returns true

### Scenario 4: Invalid Transition Attempt
**Given**: Loading state
**When**: Call complete()
**Then**:
  - Transition fails with Error::InvalidState
  - Error message contains "Loading" and "Complete"
  - State remains Loading (no mutation)

### Scenario 5: Progress Updates
**Given**: Replaying { events_processed: 0, events_total: 100 }
**When**:
  1. Call update_progress(25)
  2. Call update_progress(50)
  3. Call update_progress(75)
  4. Call update_progress(100)
**Then**:
  - All updates succeed
  - Each update preserves events_total = 100
  - Final state is Replaying { events_processed: 100, events_total: 100 }
  - Can still call complete() after final update

## State Query Scenarios

### Scenario 6: State Classification
**Given**: Each state variant
**When**: Call is_terminal() and is_active()
**Then**:
  - Uninitialized: is_terminal=false, is_active=false
  - Loading: is_terminal=false, is_active=true
  - Replaying: is_terminal=false, is_active=true
  - Complete: is_terminal=true, is_active=false
  - Failed: is_terminal=true, is_active=false

## Error Recovery Scenarios

### Scenario 7: Multiple Failure Attempts
**Given**: Failed state
**When**: Call fail() again with different error
**Then**:
  - Creates new Failed state with new error message
  - Previous error is lost (state replacement, not accumulation)
  - Remains terminal

## Implementation Notes

- **Test Coverage**: Current implementation has 680+ lines of tests
- **All tests pass**: Verified in implementation phase
- **Zero unwraps/panics**: All fallible operations use Result<T, Error>
- **Type safety**: Enum variants prevent invalid states at compile time
- **Comprehensive edge cases**: Zero values, boundary conditions, and error paths covered
