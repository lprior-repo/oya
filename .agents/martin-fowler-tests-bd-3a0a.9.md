# Martin Fowler Test Plan

**Bead ID**: bd-3a0a.9
**Feature**: cli: add oya storm orchestration command
**Generated**: 2026-02-09
**Contract Reference**: contract-bd-3a0a.9.md

## Overview

This test plan specifies executable tests for the `oya storm` CLI command using Martin Fowler's Given-When-Then approach. Tests verify contract preconditions, postconditions, invariants, and all error modes.

## Happy Path Tests

### test_storm_command_completes_successfully_with_valid_config_and_beads
**Given**: A valid config file at `.oya/orchestrator.yml` with slots=4, timeout=300s
**And**: A beads database at `.beads/beads.db` with 5 open beads in valid DAG order
**When**: User runs `oya storm` without --dry-run
**Then**:
- Command returns exit code 0
- StormOutput contains beads_completed=5, beads_failed=0
- Duration is between 0ms and timeout_ms
- Results array contains 5 entries, all with ExecutionStatus::Completed
- No error messages printed to stderr
- Orchestrator actors are stopped after completion

### test_storm_command_with_dry_run_preview_execution_plan
**Given**: A valid config file with slots=2
**And**: A beads database with 8 open beads
**When**: User runs `oya storm --dry-run`
**Then**:
- Command returns exit code 0
- StormOutput contains planned_order with 8 bead IDs in topological order
- beads_completed=0, beads_failed=0
- No beads are actually executed (no side effects in database)
- Output displays planned execution order

### test_storm_command_with_custom_config_path
**Given**: A valid config file at `/tmp/custom-orchestrator.yml`
**And**: A beads database with open beads
**When**: User runs `oya storm --config /tmp/custom-orchestrator.yml`
**Then**:
- Command loads config from specified path (not default)
- Command returns exit code 0
- Config settings (slots, timeout) are applied from custom file

### test_storm_command_with_json_output_format
**Given**: Valid config and beads database
**When**: User runs `oya storm --output json`
**Then**:
- Command outputs valid JSON to stdout
- JSON contains StormOutput fields: beads_completed, beads_failed, duration_ms
- Exit code is 0
- No non-JSON text mixed in output

### test_storm_command_respects_cli_timeout_override
**Given**: Config file specifies timeout=300s
**When**: User runs `oya storm --timeout 60`
**Then**:
- Orchestrator uses 60s timeout (not 300s)
- If execution exceeds 60s, command returns OrchestratorExecutionFailed
- Exit code is 10 (OrchestratorExecutionFailed)

### test_storm_command_respects_cli_slots_override
**Given**: Config file specifies slots=8
**When**: User runs `oya storm --slots 2`
**Then**:
- Orchestrator uses 2 parallel slots (not 8)
- Execution respects 2-slot limit
- Command completes successfully

## Error Path Tests

### test_config_file_not_found_returns_exit_code_3
**Given**: No config file exists at `.oya/orchestrator.yml`
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 3
- Error message contains "Config file not found"
- Hint message suggests creating config or using --config
- No orchestrator is initialized

### test_config_file_invalid_yaml_returns_exit_code_4
**Given**: Config file exists with malformed YAML
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 4
- Error message contains "Failed to parse config"
- YAML parse details included in error context
- Hint suggests validating YAML syntax

### test_config_file_missing_required_fields_returns_exit_code_4
**Given**: Config file exists but missing required fields (empty file)
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 4
- Error indicates which required fields are missing
- Config parsing fails at validation stage

### test_beads_database_not_found_returns_exit_code_5
**Given**: Valid config file exists
**And**: No `.beads/beads.db` file exists
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 5
- Error message contains "Beads database not found"
- Hint suggests running `oya init`
- No orchestrator execution attempted

### test_beads_database_corrupted_returns_exit_code_6
**Given**: Valid config file
**And**: `.beads/beads.db` exists but is corrupted (invalid SQLite)
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 6
- Error message contains "Database query failed"
- SQLite error details included in context
- Hint suggests checking database integrity

### test_workflow_dag_with_cycle_returns_exit_code_7
**Given**: Valid config and database
**And**: Database contains beads with circular dependency: A->B->C->A
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 7
- Error message contains "Failed to build workflow DAG"
- Error reason indicates cycle detected
- Hint suggests reviewing bead dependencies
- No orchestrator execution attempted

### test_workflow_dag_with_missing_dependency_returns_exit_code_7
**Given**: Valid config and database
**And**: Bead A depends on bead X, but X does not exist in database
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 7
- Error message indicates missing dependency
- Missing bead ID (X) mentioned in error
- DAG construction fails before execution

### test_no_open_beads_returns_exit_code_8
**Given**: Valid config and database
**And**: All beads in database have status != 'open' (all closed/completed)
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 8
- Error message contains "No beads to execute"
- Hint suggests creating beads or updating status
- DAG is not built (no nodes)

### test_orchestrator_initialization_failure_returns_exit_code_9
**Given**: Valid config and database with open beads
**And**: System cannot allocate required resources (e.g., thread spawn fails)
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 9
- Error message contains "Orchestrator initialization failed"
- Reason indicates resource or configuration issue
- Hint suggests checking system resources

### test_orchestrator_execution_timeout_returns_exit_code_10
**Given**: Valid config with timeout=5s
**And**: Database with beads that take > 5s to execute
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 10
- Error message contains "Orchestrator execution failed"
- Reason indicates timeout exceeded
- Partial execution details included (completed beads before timeout)
- Hint suggests checking logs for bead-specific failures

### test_orchestrator_crash_returns_exit_code_10
**Given**: Valid config and database
**And**: Bead execution causes orchestrator actor to panic/crash
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 10
- Error indicates orchestrator execution failed
- Crash details captured if available
- Resources are cleaned up despite crash

### test_invalid_slot_count_zero_returns_exit_code_11
**Given**: Config file specifies slots=0
**When**: User runs `oya storm --slots 0`
**Then**:
- Command returns exit code 11
- Error message contains "Invalid slot count: 0"
- Hint suggests slots must be >= 1
- No orchestrator execution attempted

### test_invalid_timeout_zero_returns_exit_code_12
**Given**: Config file or CLI specifies timeout=0s
**When**: User runs `oya storm --timeout 0`
**Then**:
- Command returns exit code 12
- Error message contains "Invalid timeout: 0s"
- Hint suggests timeout must be >= 1s
- No orchestrator execution attempted

## Edge Case Tests

### test_storm_command_with_single_bead_executes_successfully
**Given**: Database contains exactly 1 open bead with no dependencies
**When**: User runs `oya storm`
**Then**:
- Command completes successfully (exit code 0)
- beads_completed=1, beads_failed=0
- Single bead is executed

### test_storm_command_with_empty_database_returns_exit_code_8
**Given**: Database exists but contains 0 beads
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 8 (NoBeadsToExecute)
- Error message indicates no open beads found

### test_storm_command_with_linear_dependency_chain_executes_in_order
**Given**: Database with beads: A->B->C->D (linear chain)
**When**: User runs `oya storm --dry-run`
**Then**:
- planned_order shows [A, B, C, D] (topological order)
- DAG validation passes
- All beads in chain are included

### test_storm_command_with_diamond_dependency_pattern
**Given**: Database with diamond pattern: A -> (B, C) -> D
**When**: User runs `oya storm --dry-run`
**Then**:
- planned_order shows valid topological sort (A before B/C, B/C before D)
- DAG is acyclic
- All 4 beads included

### test_storm_command_with_parallel_independent_beads
**Given**: Database with 10 independent beads (no dependencies)
**And**: Config specifies slots=4
**When**: User runs `oya storm`
**Then**:
- Execution uses up to 4 parallel slots
- All 10 beads complete successfully
- Duration reflects parallel execution (< sequential time)

### test_storm_command_with_mixed_success_and_failure
**Given**: Database with 5 beads, where beads #2 and #4 will fail
**When**: User runs `oya storm`
**Then**:
- Command returns exit code 10 (OrchestratorExecutionFailed)
- beads_completed=3, beads_failed=2
- Results array shows status for each bead
- Failed beads include error messages
- Dependent beads of failed beads are skipped

### test_storm_command_preserves_database_immutability
**Given**: Database with open beads
**And**: Initial database checksum recorded
**When**: User runs `oya storm` (even with failures)
**Then**:
- Database checksum unchanged after command
- No bead statuses modified in database
- No new records added
- Database is read-only

### test_storm_command_with_unicode_bead_ids
**Given**: Database contains beads with Unicode IDs (e.g., "feature-日本語-テスト")
**When**: User runs `oya storm --dry-run`
**Then**:
- Command handles Unicode bead IDs correctly
- planned_order includes Unicode IDs
- No encoding errors

### test_storm_command_with_very_long_bead_id
**Given**: Database contains bead with ID = 1000 characters
**When**: User runs `oya storm --dry-run`
**Then**:
- Command handles long bead IDs without truncation
- planned_order includes full bead ID
- No buffer overflow or string slicing issues

### test_storm_command_config_path_with_spaces
**Given**: Config file at path with spaces: "/tmp/my config/orchestrator.yml"
**When**: User runs `oya storm --config "/tmp/my config/orchestrator.yml"`
**Then**:
- Config file is loaded correctly
- Spaces in path do not cause parsing errors
- Command executes successfully

## Contract Verification Tests

### test_precondition_config_file_must_exist
**Given**: No config file exists
**When**: storm_command() is called
**Then**: Returns Err(ConfigFileNotFound) - precondition violated

### test_precondition_config_must_be_valid_yaml
**Given**: Config file with invalid YAML syntax
**When**: storm_command() is called
**Then**: Returns Err(ConfigParseFailed) - precondition violated

### test_precondition_database_must_exist
**Given**: Config exists, but database file missing
**When**: storm_command() is called
**Then**: Returns Err(DatabaseNotFound) - precondition violated

### test_precondition_database_must_contain_open_beads
**Given**: Database exists but 0 open beads
**When**: storm_command() is called
**Then**: Returns Err(NoBeadsToExecute) - precondition violated

### test_precondition_slots_must_be_positive
**Given**: Config with slots=0 or CLI flag --slots 0
**When**: storm_command() is called
**Then**: Returns Err(InvalidSlotCount) - precondition violated

### test_precondition_timeout_must_be_positive
**Given**: Config with timeout=0s or CLI flag --timeout 0
**When**: storm_command() is called
**Then**: Returns Err(InvalidTimeout) - precondition violated

### test_postcondition_success_returns_ok_with_results
**Given**: Valid config, database, and DAG
**When**: storm_command() completes successfully
**Then**:
- Returns Ok(StormOutput)
- beads_completed > 0
- beads_failed = 0
- duration_ms >= 0
- results is Some with length matching executed beads

### test_postcondition_dry_run_returns_ok_with_planned_order
**Given**: Valid config and database with --dry-run flag
**When**: storm_command() completes
**Then**:
- Returns Ok(StormOutput)
- beads_completed = 0, beads_failed = 0
- planned_order is Some with bead IDs in topological order
- results is None

### test_postcondition_failure_returns_err_with_exit_code
**Given**: Invalid config (missing file)
**When**: storm_command() is called
**Then**:
- Returns Err(StormError::ConfigFileNotFound)
- error.exit_code() returns deterministic value (always 3)
- No partial execution state

### test_invariant_dag_acyclicity_always_enforced
**Given**: Database with circular dependencies
**When**: build_workflow_dag() is called
**Then**:
- Returns Err(DagBuildFailed) with cycle reason
- DAG is never returned with cycles
- Invariant holds: DAG is always acyclic

### test_invariant_exit_code_determinism
**Given**: Same error condition (e.g., ConfigFileNotFound)
**When**: Error occurs multiple times
**Then**: exit_code() always returns same value (3)
- Exit code mapping is deterministic

### test_invariant_database_immutability_preserved
**Given**: Database with open beads
**When**: storm_command() executes (success or failure)
**Then**:
- Database file modification time unchanged
- No new records in database
- No status updates in database
- Database is read-only

### test_invariant_resource_cleanup_on_success
**Given**: Valid config and database
**When**: storm_command() completes successfully
**Then**:
- All orchestrator actors are stopped
- All threads spawned are terminated
- No orphan processes remain
- Memory freed

### test_invariant_resource_cleanup_on_failure
**Given**: Valid config but orchestrator crashes
**When**: storm_command() returns Err
**Then**:
- All orchestrator actors are stopped (best effort)
- Resources cleanup attempted
- No resource leaks

## Integration Tests (End-to-End Scenarios)

### Scenario 1: Full workflow execution with 10 beads
**Given**:
- Config file: `.oya/orchestrator.yml` with slots=3, timeout=120s
- Database with 10 open beads in complex DAG with dependencies
- All beads are designed to complete successfully

**When**: User runs `oya storm`

**Then**:
- Command returns exit code 0
- Output shows: "Completed 10 beads in Xms"
- StormOutput contains:
  - beads_completed=10, beads_failed=0
  - results array with 10 entries
  - All ExecutionStatus::Completed
- DAG dependencies were respected (B executed after A if A->B)
- Parallel execution used up to 3 slots
- Total duration < 120s (timeout)
- Database unchanged

### Scenario 2: Dry run validation of complex DAG
**Given**:
- Config file exists
- Database with 20 beads in diamond pattern dependencies

**When**: User runs `oya storm --dry-run`

**Then**:
- Command returns exit code 0
- No beads executed (verified by checking database timestamps)
- planned_order contains 20 bead IDs
- planned_order is in valid topological sort (dependencies before dependents)
- Output displays: "Planned 20 beads for execution"
- Execution estimated duration shown
- Command exits quickly (< 1s)

### Scenario 3: Timeout during execution
**Given**:
- Config with timeout=5s
- Database with beads that simulate 10s execution each

**When**: User runs `oya storm`

**Then**:
- Command returns exit code 10
- Error message: "Orchestrator execution failed: timeout exceeded"
- Partial execution details shown:
  - "Completed N beads before timeout"
  - List of completed beads
- Orphan beads not started (those dependent on incomplete beads)
- Resources cleaned up despite timeout
- Total duration ~5s (timeout value)

### Scenario 4: Config file overrides via CLI
**Given**:
- Default config at `.oya/orchestrator.yml`: slots=2, timeout=60s
- Custom config at `/tmp/fast.yml`: slots=8, timeout=10s
- Database with 20 beads

**When**: User runs `oya storm --config /tmp/fast.yml --slots 4 --timeout 30`

**Then**:
- Config loaded from /tmp/fast.yml (not default)
- slots=4 (CLI override of custom config)
- timeout=30s (CLI override of custom config)
- Execution uses 4 slots, 30s timeout
- Command completes successfully

### Scenario 5: Database query failure mid-execution
**Given**:
- Valid config and database
- Bead #5 queries database and finds corruption
- Database becomes unreadable after bead #5 completes

**When**: User runs `oya storm`

**Then**:
- Command returns exit code 6 or 10 (depending on when error surfaces)
- Error message indicates database query failure
- beads_completed >= 5 (beads before failure)
- beads_failed > 0
- Results array shows success for beads 1-5, failure for rest
- Database corruption details in error context
- Hint suggests checking database integrity

## Test Count Summary

- Happy path tests: 6
- Error path tests: 13 (all StormError variants covered)
- Edge case tests: 10
- Contract verification tests: 13
- Integration tests: 5

**Total: 47 tests**

## Coverage Goals

- Line coverage: >= 90%
- Branch coverage: >= 85%
- All error variants tested: 100% (13/13)
- All preconditions verified: 100% (6/6)
- All postconditions verified: 100% (3/3)
- All invariants verified: 100% (5/5)
