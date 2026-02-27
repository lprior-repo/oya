# Martin Fowler Test Plan

## Happy Path Tests
- `test_valid_dag_executes_in_dependency_order`
- `test_quality_gate_pass_allows_pr_create`
- `test_status_includes_compensation_summary_after_success`

## Error Path Tests
- `test_cycle_in_dag_fails_before_first_effect`
- `test_missing_dependency_reference_fails_validation`
- `test_unmet_dependency_blocks_step_execution`
- `test_beads_only_diff_fails_quality_gate_and_blocks_pr`
- `test_compensation_failure_is_reported_in_final_status`

## Edge Case Tests
- `test_single_step_dag_validation_passes`
- `test_diff_with_mixed_beads_and_source_changes_passes`
- `test_large_opencode_stdout_is_summarized_without_losing_error_metadata`
- `test_repeated_status_reads_are_stable_for_completed_run`

## Contract Verification Tests
- `test_precondition_unique_step_ids_enforced`
- `test_precondition_all_dependency_ids_exist`
- `test_postcondition_pr_not_created_when_quality_gate_fails`
- `test_invariant_compensation_journal_persisted_separately`

## Given-When-Then Scenarios
### Scenario 1: Prevent empty PR from `.beads`-only changes
Given:
- Lifecycle reached quality gate step
- Diff contains only `.beads/*`
When:
- quality gate evaluates diff
Then:
- lifecycle returns terminal `QualityGateFailed`
- `bookmark_push` and `pr_create` remain pending/not executed
- status includes failure reason and compensation outcomes

### Scenario 2: Reject invalid DAG before execution
Given:
- A lifecycle step graph with a dependency cycle
When:
- run initializes and validates graph
Then:
- no mutating command effect executes
- run ends with `DagInvalid`
- status snapshot records validation failure category

### Scenario 3: Truthful unwind visibility
Given:
- Terminal failure occurs after workspace creation
- one compensation fails
When:
- finalization completes
Then:
- final status success=false
- compensation journal includes failed item with error message
- operator can identify unresolved cleanup action from status alone
