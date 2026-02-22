# TEST_AGENT DELIVERABLES: Bead src-20y

## Overview

All acceptance tests and contracts have been written for the ZJJ gate removal feature. The RED GATE is confirmed.

## Deliverables

### 1. Contract Specifications
**File**: `contract-spec.md`
- Design-by-contract specification
- Preconditions, postconditions, invariants
- Error taxonomy
- Exact function signatures (before/after)
- Non-goals and assumptions

### 2. Test Plan
**File**: `martin-fowler-tests.md`
- Martin Fowler style test plan
- Happy path tests
- Error path tests
- Edge case tests
- Contract verification tests
- Given-When-Then scenarios
- Integration tests

### 3. Executable Tests
**File**: `tests/zjj_removal.rs`
- 16 executable tests
- 6 failing (RED GATE) - what needs to be implemented
- 10 passing - baseline verification

### 4. Implementation Guidance
**File**: `TEST_AGENT_SUMMARY.md`
- Summary of test results
- Failing tests explanation
- Passing tests baseline
- Implementation guidance
- Exit criteria

### 5. Implementation Checklist
**File**: `IMPLEMENTATION_CHECKLIST.md`
- Step-by-step implementation tasks
- Files to modify
- Pre/post-implementation verification
- Code quality checks
- Exit criteria

### 6. Status Document
**File**: `RED_GATE_STATUS.md`
- Current test status
- Test results table
- Next steps for implementing agent
- Summary of all deliverables

## Test Status

### Failing Tests (6) - RED GATE
1. test_postcondition_ship_gate_has_only_cue_artifact_generated_gate
2. test_precondition_no_stage_references_zjj_merge_queue
3. test_precondition_gate_enum_has_five_variants
4. test_error_path_gate_try_from_zjj_merge_queue_returns_error
5. test_backward_incompatibility_zjj_merge_queue_string_fails
6. test_backward_incompatibility_ship_gate_no_longer_has_two_gates

### Passing Tests (10) - GREEN
1. test_backward_incompatibility_zjj_merge_queue_removed
2. test_postcondition_all_stages_have_valid_gate_configurations
3. test_postcondition_contract_stage_has_only_compiles_gate
4. test_postcondition_explore_stage_has_no_gates
5. test_postcondition_gate_as_str_no_zjj_merge_queue
6. test_postcondition_gate_serialization_without_zjj
7. test_postcondition_implementation_stage_has_correct_gates
8. test_postcondition_pipeline_state_machine_works_without_zjj
9. test_postcondition_red_stage_has_only_compiles_gate
10. test_postcondition_witness_stage_has_correct_gates

## Verification Command

Run this command to verify the RED GATE:
```bash
cargo test --test zjj_removal
```

Expected output: `test result: FAILED. 10 passed; 6 failed; 0 ignored;`

## What the Implementing Agent Needs to Do

1. Read all documentation files (contract-spec.md, martin-fowler-tests.md, etc.)
2. Implement changes to make the 6 failing tests pass
3. Update existing tests that broke due to ZJJ removal
4. Run `moon run :ci` to verify
5. Ensure no ZJJ-related code remains

## Key Implementation Changes

1. Remove `Gate::ZjjMergeQueue` enum variant
2. Remove `MergeQueuePolicy` from RuntimeConfig
3. Remove `ZjjSyncStatus` from GateCommand
4. Update `StageName::ShipGate.gates()` to return only `CueArtifactGenerated`
5. Remove ZJJ env var reading from `RuntimeConfig::load()`

## Files to Modify

1. `src/types/pipeline.rs`
2. `src/pipeline/mod.rs`
3. `src/runtime_tools/gates.rs`
4. `tests/contract_verify.rs`
5. `tests/gates.rs`
6. `tests/jj_br_coordination.rs` (if needed)

## Test Coverage

The tests cover:
- ✅ Type system (Gate enum removal)
- ✅ Stage configuration (ShipGate gates)
- ✅ String parsing (TryFrom implementation)
- ✅ Error paths (invalid gate parsing)
- ✅ Backward incompatibility (breaking changes)
- ✅ Integration (pipeline state machine)
- ✅ Serialization (JSON round-trip)

## Success Criteria

All 6 failing tests pass AND `moon run :ci` succeeds with no warnings.

---
**TEST_AGENT work complete. Ready for implementation.**
