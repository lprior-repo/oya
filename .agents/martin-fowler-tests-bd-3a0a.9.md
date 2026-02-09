# Martin Fowler Test Plan

**Bead ID**: bd-3a0a.9
**Feature**: cli: add oya storm orchestration command
**Generated**: 2026-02-09

## Happy Path Tests

### test_storm_command_executes_simple_linear_dag_successfully
**Given**: A config file with 2 slots and timeout of 300s
**And**: A beads database with 3 open beads in linear dependency (A→B→C)
**When**: `storm_command(StormArgs { config: path, dry_run: false, .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with `beads_completed = 3`
- Returns `beads_failed = 0`
- Returns `duration_ms > 0`
- Results array contains all 3 beads with `ExecutionStatus::Completed`
- Beads executed in topological order (A, then B, then C)

### test_storm_command_executes_parallel_branches_successfully
**Given**: A config file with 4 slots and timeout of 600s
**And**: A beads database with 6 open beads in diamond DAG (A→[B,C]→D→[E,F])
**When**: `storm_command(StormArgs { config: path, dry_run: false, .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with `beads_completed = 6`
- Returns `beads_failed = 0`
- B and C execute in parallel (same time window)
- E and F execute in parallel (same time window)
- D executes after both B and C complete

### test_storm_command_dry_run_validates_without_execution
**Given**: A config file with valid settings
**And**: A beads database with 5 open beads in complex DAG
**When**: `storm_command(StormArgs { config: path, dry_run: true, .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with `beads_completed = 0`
- `results` field is `None`
- `planned_order` is `Some(Vec)` with 5 bead IDs in topological order
- No beads are actually executed
- Returns immediately (minimal duration_ms)

### test_storm_command_respects_timeout
**Given**: A config file with timeout of 5s
**And**: A beads database with slow-executing beads
**When**: `storm_command(StormArgs { config: path, timeout: Some(5s), .. })` is called
**Then**:
- Returns `Err(StormError::OrchestratorExecutionFailed)`
- Error reason contains "timeout" or "timed out"
- Partial results contain completed beads
- Orchestrator is terminated cleanly

### test_storm_command_loads_custom_config_path
**Given**: A config file at non-default path `/tmp/custom-orchestrator.yml`
**And**: Config contains slots: 8, timeout: 900s
**When**: `storm_command(StormArgs { config: PathBuf("/tmp/custom-orchestrator.yml"), .. })` is called
**Then**:
- Loads config from custom path successfully
- Orchestrator uses 8 slots
- Orchestrator respects 900s timeout
- Returns `Ok(StormOutput)`

## Error Path Tests

### test_returns_error_when_config_file_not_found
**Given**: No config file exists at specified path
**When**: `storm_command(StormArgs { config: PathBuf("/nonexistent/config.yml"), .. })` is called
**Then**:
- Returns `Err(StormError::ConfigFileNotFound)`
- Error path equals `/nonexistent/config.yml`
- `exit_code()` returns 3
- `hint()` suggests creating config or using --config flag

### test_returns_error_when_config_file_is_malformed_yaml
**Given**: A config file with invalid YAML syntax (unclosed bracket)
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::ConfigParseFailed)`
- Error source contains YAML parse error
- `exit_code()` returns 4
- `hint()` suggests validating YAML syntax

### test_returns_error_when_config_missing_required_fields
**Given**: A config file with valid YAML but missing `slots` field
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::ConfigParseFailed)`
- Error message indicates missing required field
- `exit_code()` returns 4

### test_returns_error_when_beads_database_not_found
**Given**: A valid config file
**And**: No `.beads/beads.db` file exists
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::DatabaseNotFound)`
- Error path points to `.beads/beads.db`
- `exit_code()` returns 5
- `hint()` suggests running `oya init`

### test_returns_error_when_database_corrupted
**Given**: A valid config file
**And**: A beads database file with corrupted SQLite data
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::DatabaseQueryFailed)`
- Error query field indicates failed operation
- `exit_code()` returns 6
- `hint()` suggests checking database integrity

### test_returns_error_when_dag_contains_cycle
**Given**: A beads database with beads A→B→C→A (circular dependency)
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::DagBuildFailed)`
- Error reason mentions "cycle" or "circular"
- `exit_code()` returns 7
- `hint()` suggests reviewing dependencies

### test_returns_error_when_dag_references_missing_bead
**Given**: A beads database with bead A depending on non-existent bead X
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::DagBuildFailed)`
- Error reason mentions "missing" or "not found"
- Error reason includes bead ID "X"
- `exit_code()` returns 7

### test_returns_error_when_no_open_beads_found
**Given**: A valid config file
**And**: A beads database with all beads in status `closed` or `blocked`
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::NoBeadsToExecute)`
- `exit_code()` returns 8
- `hint()` suggests creating beads or updating status

### test_returns_error_when_slot_count_is_zero
**Given**: A config file with `slots: 0`
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::InvalidSlotCount)`
- Error slots field equals 0
- `exit_code()` returns 11
- `hint()` suggests setting slots >= 1

### test_returns_error_when_slot_count_is_negative
**Given**: Command line argument `--slots -5`
**When**: `StormArgs::parse_from(["--slots", "-5"])` is called
**Then**:
- Clap validation fails or returns error
- Error message indicates invalid value

### test_returns_error_when_timeout_is_zero
**Given**: A config file with `timeout: 0s`
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::InvalidTimeout)`
- Error secs field equals 0
- `exit_code()` returns 12
- `hint()` suggests setting timeout >= 1s

### test_returns_error_when_orchestrator_init_fails
**Given**: A valid config and DAG
**And**: System resources insufficient (e.g., out of memory)
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::OrchestratorInitFailed)`
- Error reason describes resource issue
- `exit_code()` returns 9
- No orphaned processes or actors remain

### test_returns_error_when_bead_execution_fails
**Given**: A DAG with bead B that will fail during execution
**And**: Bead C depends on B
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::OrchestratorExecutionFailed)`
- Error reason mentions bead B failure
- Bead C is not executed (dependency failed)
- Bead A (no dependency on B) completes successfully
- `exit_code()` returns 10

## Edge Case Tests

### test_handles_empty_dag_gracefully
**Given**: A beads database with zero beads
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Err(StormError::NoBeadsToExecute)`
- Not a different error type

### test_handles_single_bead_dag
**Given**: A beads database with 1 open bead and no dependencies
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with `beads_completed = 1`
- Bead executes successfully
- Duration is reasonable (< 5s for fast bead)

### test_handles_max_parallelism
**Given**: A config file with `slots: 1000`
**And**: A DAG with 500 independent beads (no dependencies)
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- System limits are respected (doesn't crash)
- Either: (a) executes with system-limited parallelism, or (b) returns error about resource limits
- If successful, all 500 beads complete

### test_handles_deep_linear_dag
**Given**: A beads database with 100 beads in linear chain (A1→A2→...→A100)
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with `beads_completed = 100`
- Beads execute in strict order (A1, A2, ..., A100)
- Total execution time >= sum of individual bead times

### test_handles_wide_diamond_pattern
**Given**: A DAG with 1 root → 50 parallel branches → 1 convergence node
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with `beads_completed = 52` (1 + 50 + 1)
- All 50 parallel beads execute concurrently (limited by slots)
- Convergence node executes after all 50 complete

### test_handles_config_with_optional_fields_missing
**Given**: A minimal config file with only required fields
**When**: `storm_command(StormArgs { config: path, .. })` is called
**Then**:
- Uses default values for optional fields
- Returns `Ok(StormOutput)` if valid DAG exists

### test_handles_command_line_overriding_config_timeout
**Given**: A config file with `timeout: 600s`
**And**: Command line argument `--timeout 120s`
**When**: `storm_command(StormArgs { config: path, timeout: Some(120s), .. })` is called
**Then**:
- Command line timeout takes precedence (120s)
- Orchestrator times out after 120s if running

### test_handles_command_line_overriding_config_slots
**Given**: A config file with `slots: 4`
**And**: Command line argument `--slots 8`
**When**: `storm_command(StormArgs { config: path, slots: Some(8), .. })` is called
**Then**:
- Command line slots takes precedence (8)
- Orchestrator uses 8 parallel slots

### test_handles_json_output_format
**Given**: Command line argument `--output json`
**And**: A successful execution
**When**: `storm_command(StormArgs { output: "json".to_string(), .. })` is called
**Then**:
- Returns `Ok(StormOutput)` with JSON-serializable data
- All fields in `StormOutput` can be serialized to JSON
- Output is valid JSON when printed

## Contract Verification Tests

### test_precondition_config_file_must_exist
**Given**: No config file exists
**When**: `storm_command` is called
**Then**: Returns `Err(StormError::ConfigFileNotFound)` immediately without attempting other operations

### test_precondition_dag_must_be_acyclic
**Given**: A database with circular dependencies
**When**: `storm_command` is called
**Then**:
- Returns `Err(StormError::DagBuildFailed)`
- No beads are executed
- Error message indicates cycle detected

### test_precondition_slot_count_must_be_positive
**Given**: A config with `slots: 0`
**When**: `storm_command` is called
**Then**:
- Returns `Err(StormError::InvalidSlotCount)`
- Orchestrator is not initialized

### test_postcondition_successful_run_returns_completed_beads
**Given**: A valid DAG with 5 beads
**When**: `storm_command` completes successfully
**Then**:
- `StormOutput.beads_completed = 5`
- `StormOutput.beads_failed = 0`
- `StormOutput.results.len() = 5`
- All results have `ExecutionStatus::Completed`

### test_postcondition_dry_run_does_not_execute_beads
**Given**: A valid DAG
**When**: `storm_command` with `dry_run: true` completes
**Then**:
- `StormOutput.beads_completed = 0`
- `StormOutput.results = None`
- `StormOutput.planned_order.is_some()`

### test_postcondition_failure_returns_non_zero_exit_code
**Given**: Any `StormError` variant
**When**: `error.exit_code()` is called
**Then**: Exit code is always > 0 (never 0)

### test_postcondition_failure_provides_hint
**Given**: Any `StormError` variant
**When**: `error.hint()` is called
**Then**: Returns `Some(String)` with actionable remediation steps

### test_invariant_dag_never_contains_cycles
**Given**: Any successful DAG construction
**When**: `build_workflow_dag` returns
**Then**: Returned DAG passes `is_cyclic_directed()` check (returns false)

### test_invariant_all_bead_ids_in_dag_exist_in_database
**Given**: A successfully built DAG
**When**: Each bead ID in DAG is queried in database
**Then**: All bead IDs exist in beads table

### test_invariant_exit_code_determinism
**Given**: The same error condition occurs twice
**When**: `error.exit_code()` is called on both errors
**Then**: Both exit codes are identical

## Given-When-Then Scenarios

### Scenario 1: Successful execution of linear workflow
**Given**: A developer has created 3 beads with linear dependencies
```
bd-001 (implement auth)
bd-002 (implement login) depends on bd-001
bd-003 (implement logout) depends on bd-002
```
**And**: All beads have status `open`
**And**: Config file exists with `slots: 2, timeout: 300s`

**When**: The developer runs `oya storm`

**Then**:
- Command executes successfully
- Output shows "3 beads completed, 0 failed"
- bd-001 executes first
- After bd-001 completes, bd-002 starts
- After bd-002 completes, bd-003 starts
- All beads complete within 300s
- Exit code is 0

### Scenario 2: Dry run previews execution plan
**Given**: A developer wants to preview execution before running
**And**: Database contains 10 beads with complex dependencies
**And**: Config file exists

**When**: The developer runs `oya storm --dry-run`

**Then**:
- Command completes quickly (< 1s)
- Output shows "Planned execution order: 10 beads"
- Lists bead IDs in topological order
- No beads are actually executed
- Exit code is 0

### Scenario 3: Missing config file with helpful error
**Given**: A new developer cloned the repo
**And**: No config file exists
**And**: No `.oya` directory exists

**When**: The developer runs `oya storm`

**Then**:
- Command fails immediately
- Error message: "Config file not found: .oya/orchestrator.yml"
- Exit code is 3
- Hint: "Create config with `oya init --template orchestrator` or specify --config"

### Scenario 4: Circular dependency detected
**Given**: A developer accidentally created circular dependencies
```
bd-001 depends on bd-002
bd-002 depends on bd-001
```
**And**: Both beads have status `open`

**When**: The developer runs `oya storm`

**Then**:
- Command fails during DAG construction
- Error message: "Failed to build workflow DAG: cycle detected"
- Exit code is 7
- Hint: "Review bead dependencies for circular references"
- No beads are executed

### Scenario 5: Timeout prevents infinite loop
**Given**: A bead has a bug causing it to never complete
**And**: Config has `timeout: 10s`
**And**: The bead is part of a DAG

**When**: The developer runs `oya storm --timeout 10s`

**Then**:
- Orchestrator starts execution
- After 10s, execution is terminated
- Error message: "Orchestrator execution failed: timeout after 10s"
- Exit code is 10
- Partial results show which beads completed before timeout

### Scenario 6: Parallel execution with diamond pattern
**Given**: A workflow with diamond dependency pattern
```
bd-setup
bd-feature-a depends on bd-setup
bd-feature-b depends on bd-setup
bd-integration depends on bd-feature-a and bd-feature-b
```
**And**: Config has `slots: 4`

**When**: The developer runs `oya storm`

**Then**:
- bd-setup executes first
- After bd-setup completes, both bd-feature-a and bd-feature-b start in parallel
- bd-integration waits for both features to complete
- bd-integration executes after both dependencies are done
- Total time < time_a + time_b (due to parallelism)
- Exit code is 0

### Scenario 7: Override config with command line options
**Given**: Config file has `slots: 2, timeout: 600s`
**And**: Developer wants faster feedback with more parallelism

**When**: The developer runs `oya storm --slots 8 --timeout 120s`

**Then**:
- Orchestrator uses 8 slots (not 2)
- Orchestrator times out after 120s (not 600s)
- Execution succeeds with overridden settings
- Config file is not modified

### Scenario 8: No open beads to execute
**Given**: Database has beads but all are status `closed` or `blocked`
**And**: Config file exists

**When**: The developer runs `oya storm`

**Then**:
- Command fails immediately
- Error message: "No beads to execute (no beads with status 'open')"
- Exit code is 8
- Hint: "Create beads or update their status to 'open'"

### Scenario 9: JSON output for CI/CD integration
**Given**: A CI/CD pipeline needs machine-readable output
**And**: Database has valid workflow
**And**: Config file exists

**When**: The pipeline runs `oya storm --output json`

**Then**:
- Command outputs valid JSON
- JSON contains: beads_completed, beads_failed, duration_ms, results array
- Exit code is 0 on success, non-zero on failure
- JSON can be parsed by jq or other tools

### Scenario 10: Bead failure blocks dependent beads
**Given**: A workflow with dependencies
```
bd-infra (completed)
bd-auth (will fail)
bd-api depends on bd-auth
```
**And**: Config file exists

**When**: The developer runs `oya storm`

**Then**:
- bd-infra completes successfully
- bd-auth fails
- bd-api is NOT executed (dependency failed)
- Error message: "Orchestrator execution failed: bead bd-auth failed"
- Exit code is 10
- Results show: bd-infra (Completed), bd-auth (Failed), bd-api (Skipped)
