# Martin Fowler Test Plan

## Happy Path Tests
- `test_compensation_runs_jj_workspace_forget_and_removes_directory`
  Given: Valid WorkspaceName with existing jj workspace and filesystem directory
  When: run_compensation is called with Compensation::ForgetWorkspace
  Then: jj workspace forget command is executed, directory is removed, result is Ok

- `test_compensation_succeeds_when_directory_already_missing`
  Given: Valid WorkspaceName with jj workspace but no filesystem directory
  When: run_compensation is called
  Then: jj workspace forget succeeds, no error for missing directory, result is Ok

## Error Path Tests
- `test_compensation_fails_when_jj_workspace_forget_fails`
  Given: Valid WorkspaceName but jj workspace forget fails (workspace doesn't exist)
  When: run_compensation is called
  Then: Returns Err with diagnostic showing jj failure

- `test_compensation_fails_when_directory_removal_fails`
  Given: Valid WorkspaceName with jj workspace but directory removal fails (permission denied)
  When: run_compensation is called
  Then: Returns Err with diagnostic showing removal failure

- `test_compensation_fails_when_jj_not_available`
  Given: Valid WorkspaceName but jj command not found
  When: run_compensation is called
  Then: Returns Err with diagnostic showing command not found

## Edge Case Tests
- `test_compensation_handles_permission_error_gracefully`
  Given: Directory exists but cannot be removed due to permissions
  When: run_compensation is called
  Then: Returns Err with specific permission error in diagnostic

- `test_compensation_handles_concurrent_access`
  Given: Directory is locked by another process
  When: run_compensation is called
  Then: Returns Err or retries appropriately

## Contract Verification Tests
- `test_postcondition_jj_forget_executed`
  Given: Valid compensation
  When: run_compensation completes
  Then: jj workspace forget was called (verify in journal)

- `test_postcondition_directory_removed`
  Given: Valid compensation with directory present
  When: run_compensation completes successfully
  Then: Directory no longer exists

- `test_invariant_no_partial_state_on_success`
  Given: run_compensation returns Ok
  Then: Both jj forget succeeded AND directory removed (check journal + filesystem)

- `test_invariant_no_partial_state_on_failure`
  Given: run_compensation returns Err
  Then: Diagnostic captures which operation failed

## Contract Violation Tests
- `test_invalid_workspace_returns_error`
  Given: Compensation::ForgetWorkspace with non-existent workspace
  When: run_compensation is called
  Then: Returns Err (not panic) with diagnostic

- `test_null_workspace_handled`
  Given: Compensation::ForgetWorkspace with empty workspace name
  When: run_compensation is called
  Then: Returns Err (not panic) - precondition violation

## Given-When-Then Scenarios

### Scenario 1: Successful Workspace Cleanup
Given: A workspace "test-workspace" exists in jj and has directory at ../test-workspace
When: Lifecycle completes successfully and finalize_success is called
Then:
- jj workspace forget test-workspace is executed
- Directory ../test-workspace is removed
- Compensation journal shows both operations succeeded
- LifecycleRunOutcome contains compensation_diagnostics with success=true

### Scenario 2: Cleanup After Failed Lifecycle
Given: A workspace exists but lifecycle failed at some step
When: finalize_failure is called
Then:
- workspace_cleanup is still called
- jj workspace forget is executed
- Directory is removed
- Failure diagnostic includes cleanup results
