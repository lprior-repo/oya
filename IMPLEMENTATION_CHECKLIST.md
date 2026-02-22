# IMPLEMENTATION CHECKLIST: Bead src-20y

## Pre-Implementation Checklist
- [x] Contracts written (contract-spec.md)
- [x] Test plan written (martin-fowler-tests.md)
- [x] Failing tests written (tests/zjj_removal.rs)
- [x] RED GATE confirmed (6 tests failing)
- [ ] Ready for LOGIC_AGENT to implement

## Implementation Tasks

### 1. Remove ZjjMergeQueue from Gate enum (src/types/pipeline.rs)
- [ ] Remove `Gate::ZjjMergeQueue` variant (line 138)
- [ ] Remove `"zjj_merge_queue"` from `Gate::as_str()` match (line 149)
- [ ] Remove `"zjj_merge_queue"` from `Gate::try_from()` match (line 164)
- [ ] Update `StageName::ShipGate.gates()` to return `vec![Gate::CueArtifactGenerated]` (line 74)

### 2. Remove MergeQueuePolicy from RuntimeConfig (src/pipeline/mod.rs)
- [ ] Remove `MergeQueuePolicy` enum (lines 46-50)
- [ ] Remove `merge_queue_policy` field from `RuntimeConfig` (line 22)
- [ ] Remove `OYA_DISABLE_ZJJ`, `OYA_SKIP_ZJJ_GATE`, `OYA_SKIP_ZJJ_WORKSPACE` env var reading
- [ ] Update `RuntimeConfig::load()` to:
  - Remove `read_flag(ctx, "OYA_DISABLE_ZJJ")`
  - Remove `read_zjj_skip_flags()` call
  - Remove `merge_queue_policy` field initialization
  - Simplify `workspace_policy` or remove if unused

### 3. Remove ZjjSyncStatus from GateCommand (src/runtime_tools/gates.rs)
- [ ] Remove `ZjjSyncStatus` variant from `GateCommand` enum (line 56)
- [ ] Remove ZJJ timeout handling from `execute_gate()`:
  - Remove `match gate { Gate::ZjjMergeQueue => ZJJ_TIMEOUT_SECONDS, _ => ... }`
  - Use `MOON_TIMEOUT_SECONDS` for all gates
- [ ] Remove `zjj` command parsing from `parse_gate_command_parts()`:
  - Remove `("zjj", zjj_args) if zjj_args == ["sync", "--status"] => Ok(GateCommand::ZjjSyncStatus)`
- [ ] Remove ZjjSyncStatus case from `GateCommand::command_parts()`:
  - Remove `GateCommand::ZjjSyncStatus => ("zjj".to_string(), vec!["sync".to_string(), "--status".to_string()])`
- [ ] Remove `(&Stage::ShipGate, &Gate::ZjjMergeQueue)` case from `gate_failure_mapping()` (lines 295-297)
- [ ] Consider removing `ZJJ_TIMEOUT_SECONDS` constant (line 8)

### 4. Update Existing Tests

#### tests/contract_verify.rs
- [ ] Remove or update `test_verify_zjj_exit_codes`
- [ ] Update `contract_gate_definitions` (lines 178-183):
  - Change `assert_eq!(ship_gates.len(), 2)` to `assert_eq!(ship_gates.len(), 1)`
  - Remove `assert!(ship_gates.contains(&Gate::ZjjMergeQueue))`

#### tests/gates.rs
- [ ] Update `given_shipgate_when_gates_run_then_runs_ci_and_merge_checks`:
  - Update test name to reflect only CueArtifactGenerated
  - Change `assert_eq!(gates.len(), 2)` to `assert_eq!(gates.len(), 1)`
  - Remove `assert!(gates.contains(&Gate::ZjjMergeQueue))`
  - Update doc comment to remove ZjjMergeQueue reference
- [ ] Update `given_all_stages_when_gates_checked_then_appropriate_for_stage`:
  - Remove `assert!(shipgate_gates.contains(&Gate::ZjjMergeQueue))` (line 136)

#### tests/jj_br_coordination.rs
- [ ] Review and update tests that reference ZjjMergeQueue
- [ ] Consider if these tests are still relevant after ZJJ removal

#### tests/src_15v_zjj_removal.rs
- [ ] Review if this file is from a previous bead or TODO
- [ ] Update or remove if outdated

## Post-Implementation Verification

### Test Execution
- [ ] All 6 tests in `tests/zjj_removal.rs` pass:
  - test_postcondition_ship_gate_has_only_cue_artifact_generated_gate
  - test_precondition_no_stage_references_zjj_merge_queue
  - test_precondition_gate_enum_has_five_variants
  - test_error_path_gate_try_from_zjj_merge_queue_returns_error
  - test_backward_incompatibility_zjj_merge_queue_string_fails
  - test_backward_incompatibility_ship_gate_no_longer_has_two_gates
- [ ] All other existing tests pass (or are properly updated)
- [ ] `moon run :test` passes with no test failures

### Code Quality
- [ ] `moon run :clippy` passes with no warnings
- [ ] `moon run :check` passes
- [ ] No unwrap/panic/expect violations
- [ ] No unused imports or dead code

### Code Review
- [ ] No references to `ZjjMergeQueue` remain in codebase
- [ ] No references to `MergeQueuePolicy` remain
- [ ] No references to `ZjjSyncStatus` remain
- [ ] No references to `OYA_DISABLE_ZJJ`, `OYA_SKIP_ZJJ_GATE`, `OYA_SKIP_ZJJ_WORKSPACE` remain
- [ ] No references to `ZJJ_TIMEOUT_SECONDS` remain (or is properly documented as unused)

### Documentation
- [ ] Contract documents updated if needed
- [ ] Code comments updated to reflect changes
- [ ] No TODO comments related to ZJJ removal

## Exit Criteria

The bead is COMPLETE when:

- [x] RED GATE confirmed (6 tests failing before implementation)
- [ ] All tests GREEN after implementation
- [ ] `moon run :ci` passes (includes clippy, tests)
- [ ] Code review approves
- [ ] No ZJJ-related code remains

## Files Changed

1. `src/types/pipeline.rs` - Remove ZjjMergeQueue enum variant and usage
2. `src/pipeline/mod.rs` - Remove MergeQueuePolicy and related config
3. `src/runtime_tools/gates.rs` - Remove ZjjSyncStatus and ZJJ-specific logic
4. `tests/contract_verify.rs` - Update existing tests
5. `tests/gates.rs` - Update existing tests
6. `tests/jj_br_coordination.rs` - Review and update if needed
7. `tests/src_15v_zjj_removal.rs` - Review and update if needed

## Notes

- This is a **breaking change** - backward compatibility with ZJJ-based workflows is NOT maintained
- The ZjjMergeQueue gate is being **deprecated/removed** - not replaced
- All tests in `tests/zjj_removal.rs` must pass to confirm removal is complete
- Existing tests that fail after removal are expected and should be updated
