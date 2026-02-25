# Martin Fowler Test Plan

## Happy Path Tests

### Run Lifecycle Tests
- `test_insert_new_run_persists_all_fields`
  - Given: A valid Run with Pending state, unique RunId, existing BeadId
  - When: insert_run is called
  - Then:
    - Run is stored in 'bead_runs' tree
    - All fields (id, bead_id, state, timestamps) are persisted
    - Returns Ok(())

- `test_get_run_retrieves_complete_run`
  - Given: A Run was inserted with ID "test-run-123"
  - When: get_run is called with that RunId
  - Then:
    - Returns Ok(Run) with matching id, bead_id, state
    - created_at and updated_at are preserved
    - history Vec is empty (no attempts yet)

- `test_update_run_state_transitions_from_pending_to_running`
  - Given: A Run exists in Pending state
  - When: update_run_state is called with Running { current_stage: Contract }
  - Then:
    - Run state is updated to Running
    - updated_at timestamp is more recent than created_at
    - Returns Ok(())

### StageAttempt Tests
- `test_insert_stage_attempt_persists_attempt_details`
  - Given: A Run exists and a StageAttempt with attempt=1, stage=Contract
  - When: insert_stage_attempt is called
  - Then:
    - Attempt is stored in 'stage_attempts' tree
    - Key format is "{run_id}:contract:001"
    - All fields (session_id, timestamps, state) are persisted
    - Returns Ok(())

- `test_get_stage_attempts_returns_ordered_attempts`
  - Given: Three attempts exist for same run: (contract, 1), (contract, 2), (tdd15, 1)
  - When: get_stage_attempts is called
  - Then:
    - Returns Vec with 3 attempts
    - Attempts are ordered by (stage, attempt number)
    - contract:1 appears before contract:2 before tdd15:1

### Artifact Tests
- `test_insert_artifact_persists_with_checksum`
  - Given: A Run exists and an Artifact with checksum="abc123"
  - When: insert_artifact is called
  - Then:
    - Artifact is stored in 'artifacts' tree
    - Checksum is preserved
    - produced_by_stage is serialized correctly
    - Returns Ok(())

- `test_get_artifacts_filters_by_stage`
  - Given: Three artifacts exist for a run (2 from contract stage, 1 from tdd15 stage)
  - When: get_artifacts is called with Some(StageName::Contract)
  - Then:
    - Returns Vec with 2 artifacts
    - All artifacts have produced_by_stage == Contract

### Replayability Tests
- `test_replayability_restores_run_after_database_restart`
  - Given: A Run with 3 StageAttempts and 5 Artifacts was persisted
  - When: Database is closed, reopened, and get_run is called
  - Then:
    - Returns Ok(Run) with complete state
    - history.len() == 3
    - All attempts have correct session_id, timestamps, state
    - get_artifacts returns all 5 artifacts

- `test_replayability_preserves_attempt_ordering`
  - Given: Attempts inserted out of order (attempt 2, then attempt 1, then attempt 3)
  - When: get_stage_attempts is called
  - Then:
    - Returns attempts ordered by attempt number: 1, 2, 3
    - Ordering is preserved across database restart

### Idempotency Tests
- `test_insert_run_is_idempotent`
  - Given: A Run was already inserted with ID "duplicate-test"
  - When: insert_run is called again with same Run
  - Then:
    - Returns Ok(()) (does not error)
    - Only one record exists in database
    - Second write overwrites first (upsert semantics)

- `test_insert_stage_attempt_is_idempotent`
  - Given: A StageAttempt (run_123, contract, 1) was already inserted
  - When: insert_stage_attempt is called again with identical attempt
  - Then:
    - Returns Ok(()) (does not error)
    - Only one record exists with key "run_123:contract:001"

---

## Error Path Tests

### NotFound Errors
- `test_get_run_returns_error_when_run_does_not_exist`
  - Given: No run with ID "nonexistent-run" exists
  - When: get_run is called with that ID
  - Then:
    - Returns Err(OyaDbError::NotFound("nonexistent-run"))
    - Error message includes the run ID

- `test_update_run_state_returns_error_when_run_not_found`
  - Given: No run with ID "ghost-run" exists
  - When: update_run_state is called with that ID
  - Then:
    - Returns Err(OyaDbError::NotFound("ghost-run"))
    - No state mutation occurs

- `test_insert_stage_attempt_returns_error_when_run_not_found`
  - Given: A StageAttempt references run_id "orphan-run"
  - When: insert_stage_attempt is called but run does not exist
  - Then:
    - Returns Err(OyaDbError::RunNotFound("orphan-run"))
    - No attempt record is created

### Serialization Errors
- `test_insert_run_returns_error_on_invalid_json`
  - Given: Mock Sled tree that fails serialization
  - When: insert_run is called with unserializable data
  - Then:
    - Returns Err(OyaDbError::Serialization(_))
    - Error message describes serialization failure
    - No partial write occurs

- `test_get_run_returns_error_on_corrupted_data`
  - Given: Run data in Sled is corrupted (invalid JSON)
  - When: get_run is called
  - Then:
    - Returns Err(OyaDbError::Serialization(_))
    - Error includes details of parse failure

### Precondition Violations
- `test_insert_stage_attempt_returns_error_when_attempt_limit_exceeded`
  - Given: A StageAttempt with attempt=4 for stage Contract (max=3)
  - When: insert_stage_attempt is called
  - Then:
    - Returns Err(OyaDbError::AttemptLimitExceeded { stage: "contract", attempt: 4, max: 3 })
    - No record is created

- `test_update_run_state_returns_error_on_invalid_transition`
  - Given: A Run in Shipped state
  - When: update_run_state is called with Running
  - Then:
    - Returns Err(OyaDbError::InvalidStateTransition { from: "Shipped", to: "Running" })
    - State remains Shipped

- `test_insert_stage_attempt_returns_error_when_timestamp_order_invalid`
  - Given: A StageAttempt with completed_at < started_at
  - When: insert_stage_attempt is called
  - Then:
    - Returns Err(OyaDbError::InvalidTimestampOrder)
    - No record is created

### Database Errors
- `test_insert_run_returns_error_when_disk_full`
  - Given: Mock Sled that returns disk-full error
  - When: insert_run is called
  - Then:
    - Returns Err(OyaDbError::Database(sled::Error::Io(_)))
    - No partial write occurs

- `test_get_run_returns_error_on_database_corruption`
  - Given: Sled database files are corrupted
  - When: get_run is called
  - Then:
    - Returns Err(OyaDbError::Database(_))
    - Error is propagated from Sled

---

## Edge Case Tests

### Boundary Values
- `test_handles_empty_history_gracefully`
  - Given: A Run exists with no StageAttempts
  - When: get_run is called
  - Then:
    - Returns Ok(Run) with history.len() == 0
    - No error is returned

- `test_handles_run_with_max_attempts_per_stage`
  - Given: A Run with 3 attempts for Contract stage (max allowed)
  - When: insert_stage_attempt is called with attempt=3
  - Then:
    - Returns Ok(())
    - All 3 attempts are persisted
    - Fourth attempt would return AttemptLimitExceeded

- `test_handles_single_artifact_correctly`
  - Given: A Run with exactly one Artifact
  - When: get_artifacts is called
  - Then:
    - Returns Vec with len() == 1
    - Artifact fields are correct

### Empty and None Handling
- `test_handles_empty_string_bead_id`
  - Given: A Run with bead_id = ""
  - When: insert_run is called
  - Then:
    - Returns Err(OyaDbError::Serialization(_)) or appropriate validation error
    - No record with empty bead_id is created

- `test_handles_optional_checksum_none`
  - Given: An Artifact with checksum = None
  - When: insert_artifact is called
  - Then:
    - Returns Ok(())
    - Artifact is persisted with null/empty checksum

- `test_handles_optional_completed_at_none`
  - Given: A StageAttempt with completed_at = None (still running)
  - When: insert_stage_attempt is called
  - Then:
    - Returns Ok(())
    - Attempt is persisted with completed_at = None

### Special Characters in IDs
- `test_handles_run_id_with_special_characters`
  - Given: A RunId containing special chars like "test/run-123"
  - When: insert_run and get_run are called
  - Then:
    - Both operations succeed
    - ID is preserved exactly

- `test_handles_artifact_location_with_url`
  - Given: An Artifact with location = "https://example.com/artifact.pdf"
  - When: insert_artifact and get_artifacts are called
  - Then:
    - URL is preserved exactly
    - No URL encoding/decoding issues

---

## Contract Verification Tests

### Precondition Tests
- `test_precondition_run_id_must_be_unique`
  - Given: A Run with ID "clone-123" already exists
  - When: insert_run is called with different Run but same ID
  - Then:
    - Returns Ok(()) (idempotent upsert)
    - Original run is overwritten (verified by field comparison)

- `test_precondition_stage_attempt_must_reference_existing_run`
  - Given: A StageAttempt with run_id that does not exist
  - When: insert_stage_attempt is called
  - Then:
    - Returns Err(OyaDbError::RunNotFound(_))
    - Invariant is enforced

### Postcondition Tests
- `test_postcondition_run_flushed_to_disk`
  - Given: A Run is inserted
  - When: Database is immediately closed without explicit flush
  - Then:
    - Run is still readable after reopening database
    - Sled's flush guarantees durability

- `test_postcondition_artifacts_retrievable_by_stage`
  - Given: Artifacts inserted for multiple stages
  - When: get_artifacts is called with stage filter
  - Then:
    - Only artifacts matching the stage are returned
    - Postcondition: filtering works correctly

### Invariant Tests
- `test_invariant_run_state_exclusivity`
  - Given: A Run in Running state
  - When: State is queried
  - Then:
    - Run is in exactly one state (Running)
    - Not in multiple states simultaneously

- `test_invariant_attempt_monotonicity`
  - Given: Three attempts for same stage: 1, 2, 3
  - When: Attempts are retrieved
  - Then:
    - Attempts are strictly increasing: 1 < 2 < 3
    - No gaps in sequence

- `test_invariant_referential_integrity_no_orphan_attempts`
  - Given: A Run is deleted (hypothetically, for this test)
  - When: StageAttempts are queried for that run
  - Then:
    - Either cascade delete removes attempts OR
    - Attempts are marked as orphaned (future enhancement)

---

## Given-When-Then Scenarios

### Scenario 1: Full Run Lifecycle with Replayability

**Given:**
- A Bead "oya-test-001" exists in the tracker
- Sled database is empty (fresh start)
- Moon gates are available

**When:**
1. Create Run with RunId::new(), bead_id="oya-test-001", state=Pending
2. insert_run is called
3. Create StageAttempt for Contract stage, attempt=1
4. insert_stage_attempt is called
5. Update Run state to Running { current_stage: Contract }
6. update_run_state is called
7. Insert Artifact (contract_document) for Contract stage
8. insert_artifact is called
9. Complete Contract stage: insert StageResult with passed=true
10. Simulate process restart: close and reopen Sled database
11. Call get_run with original RunId

**Then:**
- Step 2: Returns Ok(())
- Step 4: Returns Ok(())
- Step 6: Returns Ok(())
- Step 8: Returns Ok(())
- Step 11: Returns Ok(Run) with:
  - run.id == original RunId
  - run.bead_id == "oya-test-001"
  - run.state == Running { current_stage: Contract }
  - run.history.len() == 1
  - history[0].stage == Contract
  - history[0].attempt == 1
- get_artifacts(run_id, Some(Contract)) returns 1 artifact
- **Postcondition**: Replayability achieved - state fully restored

### Scenario 2: Attempt Retry with State Progression

**Given:**
- A Run in Running state at Contract stage
- Attempt 1 failed with GateResult { passed: false, exit_code: 1 }

**When:**
1. Insert StageResult for attempt 1: failed=false, failure_category=TestFailed
2. Insert StageAttempt for attempt 2
3. Insert StageResult for attempt 2: passed=true
4. Update Run state to Running { current_stage: Tdd15 }
5. Close and reopen database
6. Call get_run and get_stage_attempts

**Then:**
- Step 1: Returns Ok(())
- Step 2: Returns Ok(())
- Step 3: Returns Ok(())
- Step 4: Returns Ok(())
- Step 6: Returns:
  - Run with state = Running { current_stage: Tdd15 }
  - history.len() == 2
  - history[0].attempt == 1, history[0].state == Failed
  - history[1].attempt == 2, history[1].state == Passed
- **Invariant**: Attempt monotonicity preserved (1 < 2)
- **Postcondition**: State transition Contract → Tdd15 is valid

### Scenario 3: Error Recovery from Corrupted Data

**Given:**
- A Run exists with 3 attempts
- Sled database file is manually corrupted (for testing)

**When:**
- get_run is called

**Then:**
- Returns Err(OyaDbError::Database(_)) or Serialization(_)
- Error message indicates corruption
- No panic or unwrap occurs
- **Contract**: Error is propagated, not swallowed

---

## End-to-End Test

### E2E: Full Pipeline Persistence and Replay

**Setup:**
- Start with fresh Sled database at `/tmp/oya-e2e-test`
- Create Run for bead "oya-e2e-001"
- Execute all 6 stages with attempts and artifacts
- Persist after each stage completion

**Execute:**
```bash
# Insert run
oya-persistence insert-run --run-id <ULID> --bead-id oya-e2e-001

# Execute contract stage (attempt 1, passes)
oya-persistence insert-attempt --run-id <ULID> --stage contract --attempt 1
oya-persistence update-state --run-id <ULID> --state running:contract
oya-persistence insert-artifact --run-id <ULID> --type contract_document --location /path/to/contract.md

# Complete contract, advance to tdd15
oya-persistence update-state --run-id <ULID> --state running:tdd15
# ... repeat for all stages

# Simulate restart
kill oya-persistence
oya-persistence start

# Replay
oya-persistence get-run --run-id <ULID> --output /tmp/replay.json
```

**Verify:**
- Exit code is 0
- `/tmp/replay.json` contains:
  - Complete run history (all 6 stages)
  - All artifacts (contract, code, tests, reports)
  - Correct state at each stage
  - Timestamps in correct order
- Moon gate `:ci` passes (no unwrap/panic/expect)
- **Evidence**: Replayability achieved

---

## Test Organization

### File Structure
```
src/persistence/tests/
  ├── mod.rs                       # Test module
  ├── run_tests.rs                # Run lifecycle tests
  ├── attempt_tests.rs            # StageAttempt tests
  ├── artifact_tests.rs           # Artifact tests
  ├── replayability_tests.rs      # Replay and restore tests
  ├── error_tests.rs              # Error path tests
  └── edge_case_tests.rs          # Boundary and special case tests
```

### Test Naming Convention
- `test_<domain>_<action>_<outcome>`
- Examples:
  - `test_run_insert_returns_ok`
  - `test_run_get_returns_error_when_not_found`
  - `test_attempt_insert_enforces_max_attempts`

---

## Coverage Requirements

- [ ] All happy paths covered
- [ ] All error variants tested at least once
- [ ] All preconditions have validation tests
- [ ] All postconditions have assertion tests
- [ ] All invariants have violation tests
- [ ] Edge cases (empty, max, special chars) covered
- [ ] Idempotency verified for insert operations
- [ ] Replayability verified with restart scenarios
- [ ] E2E scenario covers full pipeline
