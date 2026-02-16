# Martin Fowler Test Plan: Oya Run Orchestration

## Happy Path Tests
- `test_starts_new_run_in_pending_state`
- `test_transitions_run_to_next_stage_on_success`
- `test_completes_run_when_all_stages_pass`
- `test_ships_run_after_approval`

## Error Path Tests
- `test_prevents_transition_from_failed_to_running`
- `test_fails_run_when_critical_stage_fails`
- `test_rejects_shipping_incomplete_run`
- `test_handles_missing_artifacts_gracefully`

## Edge Case Tests
- `test_handles_zero_duration_stages`
- `test_handles_retries_exceeding_max_limit`
- `test_idempotency_of_stage_completion`

## State Machine Verification (Property-Based)
- `prop_state_transitions_preserve_invariants` (Proptest)
- `prop_completed_at_always_after_started_at`

## Given-When-Then Scenarios

### Scenario 1: Successful TDD Cycle
**Given**: A Run is in `Tdd15` stage
**When**: The stage completes with `passed=true`
**Then**:
- The Run status updates to `Running` (or similar)
- The current stage advances to `Qa` (or next configured stage)
- An event is recorded

### Scenario 2: Failed QA Gate
**Given**: A Run is in `Qa` stage
**When**: The stage completes with `passed=false`
**Then**:
- The Run status updates to `Failed` (or `Blocked`)
- The failure reason is recorded
- No stage advancement occurs
