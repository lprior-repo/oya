# Martin Fowler Test Plan

## Happy Path Tests

### Workflow Lifecycle Tests
- `test_start_run_workflow_creates_new_run_in_pending_state`
  - Given: A valid RunId and existing BeadId
  - When: start_run_workflow is called
  - Then:
    - Run is persisted with Pending state
    - Workflow is initialized in Restate
    - Returns Ok(WorkflowId)
    - created_at and updated_at are set to now

- `test_advance_to_next_stage_transitions_from_contract_to_tdd15`
  - Given: A Run in Running(Contract) state with completed Contract stage
  - When: advance_to_next_stage is called with Contract completion
  - Then:
    - Run state transitions to Running(Tdd15)
    - StageAttempt(Contract, attempt=1) is persisted
    - Returns Ok(Some(Tdd15))
    - updated_at is more recent than previous timestamp

- `test_advance_to_next_stage_progresses_through_all_stages`
  - Given: A Run at each stage in sequence
  - When: advance_to_next_stage is called repeatedly
  - Then:
    - Contract → Tdd15 → Qa → RedQueen → GptReview → ShipGate
    - Each transition creates a StageAttempt record
    - Final transition (ShipGate) returns Ok(None) (terminal)

- `test_complete_workflow_transitions_to_shipped_state`
  - Given: A Run in Running(ShipGate) with all gates passed
  - When: complete_workflow is called with Shipped terminal state
  - Then:
    - Run state is set to Shipped
    - shipped_at timestamp is recorded
    - No further transitions are possible
    - Returns Ok(())

### Retry Logic Tests
- `test_handle_stage_failure_routes_back_to_tdd15`
  - Given: A Run at Qa stage with failed gate (retryable failure)
  - When: handle_stage_failure is called
  - Then:
    - Run state transitions back to Running(Tdd15)
    - Failure context is preserved
    - Backoff timer is scheduled (2^attempt seconds)
    - Returns Ok(RetryScheduled)

- `test_handle_stage_failure_increments_attempt_count`
  - Given: A Run failing Contract stage for the second time
  - When: handle_stage_failure is called with attempt=2
  - Then:
    - Attempt count for Contract is preserved as 2
    - Backoff is 4 seconds (2^2)
    - Next retry starts from Tdd15

- `test_calculate_backoff_returns_exponential_backoff`
  - Given: Attempt numbers 1, 2, 3, 4
  - When: calculate_backoff is called
  - Then:
    - attempt=1 → 2 seconds
    - attempt=2 → 4 seconds
    - attempt=3 → 8 seconds
    - attempt=4 → 16 seconds
    - Max is bounded at 300 seconds

### Replayability Tests
- `test_replay_workflow_restores_state_from_persistence`
  - Given: A Run was persisted with Running(Qa) state
  - When: replay_workflow is called after Restate restart
  - Then:
    - Returns Ok(Run) with correct state
    - history contains all previous StageAttempts
    - Workflow can be resumed from Qa stage
    - Idempotent: multiple calls return same state

- `test_replay_workflow_handles_orphaned_restate_state`
  - Given: Restate workflow state exists but Run was deleted from Sled
  - When: replay_workflow is called
  - Then:
    - Returns Err(WorkflowError::StateCorruption)
    - Error includes both states for debugging

### Pure Function Tests
- `test_get_next_canonical_stage_returns_correct_sequence`
  - Given: All stages in canonical order
  - When: get_next_canonical_stage is called
  - Then:
    - Contract → Some(Tdd15)
    - Tdd15 → Some(Qa)
    - Qa → Some(RedQueen)
    - RedQueen → Some(GptReview)
    - GptReview → Some(ShipGate)
    - ShipGate → None (terminal)

- `test_is_canonical_transition_validates_stage_sequence`
  - Given: Various stage transitions
  - When: is_canonical_transition is called
  - Then:
    - (Contract, Tdd15) → true
    - (Tdd15, Qa) → true
    - (Contract, Qa) → false (skips Tdd15)
    - (ShipGate, Contract) → false (can't go backwards)

- `test_is_retryable_failure_categorizes_failures`
  - Given: Various failure categories
  - When: is_retryable_failure is called
  - Then:
    - TestFailed → true
    - TestInfraFailed → true
    - CompileFailed → true
    - LintFailed → true
    - AuthFailed → false
    - ContextOverflow → false

---

## Error Path Tests

### Not Found Errors
- `test_start_run_workflow_returns_error_when_bead_not_found`
  - Given: A BeadId that doesn't exist in tracker
  - When: start_run_workflow is called
  - Then:
    - Returns Err(WorkflowError::BeadNotFound)
    - No Run is created
    - No workflow is initialized

- `test_advance_to_next_stage_returns_error_when_run_not_found`
  - Given: A RunId that doesn't exist
  - When: advance_to_next_stage is called
  - Then:
    - Returns Err(WorkflowError::RunNotFound)
    - No state transition occurs

### Invalid Transitions
- `test_advance_to_next_stage_returns_error_for_invalid_transition`
  - Given: A Run in Shipped state (terminal)
  - When: advance_to_next_stage is called
  - Then:
    - Returns Err(WorkflowError::InvalidTransition)
    - Error includes from="Shipped", to="Running"
    - Run remains in Shipped state

- `test_complete_workflow_returns_error_for_invalid_terminal_state`
  - Given: A Run in Pending state (not ready to complete)
  - When: complete_workflow is called with Shipped
  - Then:
    - Returns Err(WorkflowError::InvalidTransition)
    - No state change occurs

### Non-Canonical Transitions
- `test_advance_to_next_stage_blocks_skipping_stages`
  - Given: A Run in Contract stage
  - When: advance_to_next_stage attempts to skip to Qa
  - Then:
    - Returns Err(WorkflowError::NonCanonicalTransition)
    - Error includes from="Contract", to="Qa"
    - Transition is blocked

### Attempt Limit Exceeded
- `test_handle_stage_failure_returns_error_when_max_attempts_exceeded`
  - Given: A Run that has failed Contract stage 3 times
  - When: handle_stage_failure is called with attempt=4
  - Then:
    - Returns Err(WorkflowError::AttemptLimitExceeded)
    - Error includes stage="Contract", attempt=4, max=3
    - Run transitions to Failed state

### Non-Retryable Failures
- `test_handle_stage_failure_returns_error_for_non_retryable_failure`
  - Given: A Run failed with AuthFailed category
  - When: handle_stage_failure is called
  - Then:
    - Returns Err(WorkflowError::NonRetryableFailure)
    - Error includes category="AuthFailed"
    - Run transitions to Failed state immediately

### Context Overflow
- `test_advance_to_next_stage_returns_error_on_context_overflow`
  - Given: A Run with accumulated context exceeding size limit
  - When: advance_to_next_stage is called
  - Then:
    - Returns Err(WorkflowError::ContextOverflow)
    - Error includes size_bytes and max_bytes
    - Stage transition is blocked

### Concurrent Modifications
- `test_advance_to_next_stage_returns_error_on_concurrent_modification`
  - Given: Two workflows trying to advance the same Run
  - When: Second workflow commits after first
  - Then:
    - One returns Ok(())
    - Other returns Err(WorkflowError::ConcurrentModification)
    - Error includes expected_version and actual_version

---

## Edge Case Tests

### Boundary Values
- `test_handle_stage_failure_handles_first_attempt_retry`
  - Given: A Run failing on first attempt (attempt=1)
  - When: handle_stage_failure is called
  - Then:
    - Returns Ok(RetryScheduled) with backoff=2 seconds
    - Retry is allowed (1 < max_attempts)

- `test_calculate_backoff_handles_max_attempt`
  - Given: attempt_number=10 (hypothetically high)
  - When: calculate_backoff is called
  - Then:
    - Returns Duration::from_secs(300) (bounded max)
    - No overflow occurs

- `test_get_next_canonical_stage_handles_terminal_stage`
  - Given: ShipGate stage (last stage)
  - When: get_next_canonical_stage is called
  - Then:
    - Returns None
    - No panic or error

### Empty and None Handling
- `test_complete_workflow_handles_none_rationale_for_aborted`
  - Given: A Run being aborted without explicit rationale
  - When: complete_workflow is called with Aborted and None rationale
  - Then:
    - Returns Ok(())
    - aborted_at timestamp is recorded
    - rationale field is None or empty

- `test_replay_workflow_handles_run_with_empty_history`
  - Given: A Run in Pending state with no attempts yet
  - When: replay_workflow is called
  - Then:
    - Returns Ok(Run) with empty history Vec
    - state is Pending
    - No error occurs

### Special Scenarios
- `test_start_run_workflow_is_idempotent`
  - Given: A workflow already started for run_id
  - When: start_run_workflow is called again with same run_id
  - Then:
    - Returns Ok(WorkflowId) (same ID)
    - No duplicate Run is created
    - No error occurs

- `test_advance_to_next_stage_handles_duplicate_completion`
  - Given: A stage completion already recorded
  - When: advance_to_next_stage is called again for same stage
  - Then:
    - Returns Ok(next_stage) but is idempotent
    - No duplicate StageAttempt record created
    - Or returns error if already transitioned (implementation choice)

---

## Contract Verification Tests

### Precondition Tests
- `test_precondition_run_must_exist_for_advance`
  - Given: A RunId that was never created
  - When: advance_to_next_stage is called
  - Then:
    - Returns Err(WorkflowError::RunNotFound)
    - Precondition is enforced

- `test_precondition_stage_must_be_completed_for_advance`
  - Given: A Run in Running(Contract) but Contract stage still running
  - When: advance_to_next_stage is called
  - Then:
    - Returns Err(WorkflowError::InvalidTransition)
    - Precondition "stage completed" is enforced

- `test_precondition_attempt_within_limits_for_handle_failure`
  - Given: A Run with attempt=3 (max attempts)
  - When: handle_stage_failure is called
  - Then:
    - Returns Err(WorkflowError::AttemptLimitExceeded)
    - Precondition "attempt < max_attempts" is enforced

### Postcondition Tests
- `test_postcondition_state_persisted_after_transition`
  - Given: A Run advancing through stages
  - When: advance_to_next_stage completes
  - Then:
    - Run state is persisted to Sled
    - StageAttempt record exists in persistence
    - Postcondition "persistence before return" is verified

- `test_postcondition_workflow_state_updated_in_restate`
  - Given: A Run workflow in Restate
  - When: advance_to_next_stage completes
  - Then:
    - Restate workflow state is updated
    - Next workflow continuation is scheduled
    - Postcondition "workflow progressed" is verified

- `test_postcondition_terminal_state_absorbing`
  - Given: A Run in Shipped state
  - When: Any transition is attempted
  - Then:
    - Returns Err(WorkflowError::InvalidTransition)
    - State remains Shipped
    - Postcondition "no outgoing transitions" is verified

### Invariant Tests
- `test_invariant_stage_transitions_are_monotonic`
  - Given: A Run progressing through pipeline
  - When: All transitions are logged
  - Then:
    - No stage is visited twice (except via retry lane)
    - Stage indices only increase
    - Invariant "forward progress" is maintained

- `test_invariant_attempts_are_sequential_per_stage`
  - Given: A Run with multiple attempts for same stage
  - When: Attempts are queried
  - Then:
    - Attempt numbers are 1, 2, 3 (strictly increasing)
    - No gaps in sequence
    - Invariant "sequential attempts" is maintained

- `test_invariant_retry_routes_through_tdd15`
  - Given: A Run failing at Qa, RedQueen, or GptReview
  - When: handle_stage_failure is called
  - Then:
    - Next stage is always Tdd15 (retry lane entry)
    - Never routes to other stages
    - Invariant "single retry entry point" is maintained

---

## Given-When-Then Scenarios

### Scenario 1: Full Pipeline Execution (Happy Path)

**Given:**
- A Bead "oya-test-002" exists in tracker
- RunId is generated via RunId::new()
- All stages have valid gate checks configured

**When:**
1. start_run_workflow is called with run_id and bead_id
2. advance_to_next_stage is called for Contract (passed)
3. advance_to_next_stage is called for Tdd15 (passed)
4. advance_to_next_stage is called for Qa (passed)
5. advance_to_next_stage is called for RedQueen (passed)
6. advance_to_next_stage is called for GptReview (passed)
7. advance_to_next_stage is called for ShipGate (passed)
8. complete_workflow is called with Shipped state

**Then:**
- Step 1: Run created in Pending state, workflow initialized
- Step 2: Run → Running(Contract), StageAttempt(Contract, 1) persisted
- Step 3: Run → Running(Tdd15), StageAttempt(Tdd15, 1) persisted
- Step 4: Run → Running(Qa), StageAttempt(Qa, 1) persisted
- Step 5: Run → Running(RedQueen), StageAttempt(RedQueen, 1) persisted
- Step 6: Run → Running(GptReview), StageAttempt(GptReview, 1) persisted
- Step 7: Run → Running(ShipGate), StageAttempt(ShipGate, 1) persisted
- Step 8: Run → Shipped, shipped_at timestamp set, workflow completed
- **Postcondition**: 6 StageAttempts in history, run passed all gates

### Scenario 2: Retry Lane with Recovery

**Given:**
- A Run in Running(Qa) state
- Qa stage failed with TestFailed (retryable)
- This is attempt 1 for Qa

**When:**
1. handle_stage_failure is called for Qa stage
2. Workflow waits for backoff (2 seconds)
3. advance_to_next_stage is called for Tdd15 (retry lane entry)
4. Tdd15 stage passes (attempt 2)
5. Qa stage is retried (attempt 2) and passes

**Then:**
- Step 1: Returns Ok(RetryScheduled) with backoff=2 seconds
- Step 2: Run state → Running(Tdd15), failure context preserved
- Step 3: Tdd15 (attempt 2) completed successfully
- Step 4: Qa (attempt 2) completed successfully
- Step 5: Run advances to RedQueen (next stage)
- **Invariant**: Retry routing through Tdd15 preserved
- **Postcondition**: Total attempts per stage <= 3

### Scenario 3: Max Attempts Exceeded (Terminal Failure)

**Given:**
- A Run in Running(Contract) state
- Contract stage failed 3 times already (attempts 1, 2, 3)
- Fourth attempt is attempted

**When:**
1. handle_stage_failure is called with attempt=4
2. complete_workflow is called with Failed state

**Then:**
- Step 1: Returns Err(WorkflowError::AttemptLimitExceeded { stage: "Contract", attempt: 4, max: 3 })
- Step 2: Run → Failed state, failed_at timestamp recorded
- **Postcondition**: Run is terminal (no further transitions)
- **Invariant**: Max attempts enforced

### Scenario 4: Non-Retryable Failure (Immediate Termination)

**Given:**
- A Run in Running(Tdd15) state
- Tdd15 stage failed with AuthFailed category

**When:**
1. handle_stage_failure is called with AuthFailed
2. Error is checked

**Then:**
- Step 1: Returns Err(WorkflowError::NonRetryableFailure { category: "AuthFailed", reason: "..." })
- Run should transition to Failed (or requires explicit complete_workflow call)
- **Postcondition**: No retry attempted for non-retryable failures

### Scenario 5: Idempotency and Replayability

**Given:**
- A Run in Running(Qa) state with full history
- Restate service crashes and restarts

**When:**
1. replay_workflow is called after restart
2. advance_to_next_stage is called for Qa completion

**Then:**
- Step 1: Returns Ok(Run) with Running(Qa) state, history intact
- Step 2: Transition succeeds as if no restart occurred
- **Postcondition**: Workflow state fully recovered from persistence
- **Invariant**: Idempotency - multiple replay calls return same state

---

## Test Organization

### File Structure
```
src/workflow/tests/
  ├── mod.rs                      # Test module
  ├── lifecycle_tests.rs          # Workflow lifecycle tests
  ├── transition_tests.rs         # Stage transition tests
  ├── retry_tests.rs              # Retry logic tests
  ├── replayability_tests.rs     # Replay and recovery tests
  ├── pure_function_tests.rs      # Property tests for pure functions
  ├── error_tests.rs              # Error path tests
  └── edge_case_tests.rs          # Boundary and special case tests
```

### Test Naming Convention
- `test_<domain>_<action>_<outcome>`
- Examples:
  - `test_workflow_start_returns_ok`
  - `test_stage_advance_returns_error_when_not_found`
  - `test_retry_failure_enforces_max_attempts`

---

## Coverage Requirements

- [ ] All happy paths covered
- [ ] All error variants tested at least once
- [ ] All preconditions have validation tests
- [ ] All postconditions have assertion tests
- [ ] All invariants have violation tests
- [ ] Pure functions have property tests (proptest)
- [ ] Retry logic verified with state machine tests
- [ ] Idempotency verified for start and replay
- [ ] Replayability verified with restart scenarios
- [ ] E2E scenario covers full pipeline
- [ ] Edge cases (boundaries, empty, special values) covered
