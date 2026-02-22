# Martin Fowler Test Plan: JJ-BR Coordination

## Happy Path Tests

### Workspace Name Generation

- `test_build_workspace_name_with_valid_inputs_produces_valid_jj_name`
- `test_build_workspace_name_normalizes_case_and_special_characters`
- `test_build_workspace_name_handles_max_length_inputs`
- `test_build_workspace_name_deterministic_output_for_same_inputs`
- `test_build_workspace_name_includes_all_components_in_correct_order`

### Gate Execution

- `test_zjj_sync_status_gate_passes_when_database_clean`
- `test_zjj_merge_queue_gate_passes_when_no_conflicts`
- `test_gate_evidence_contains_all_fields_on_success`

### Stage-Gate Coordination

- `test_shipgate_stage_includes_zjj_merge_queue_gate`
- `test_stage_gates_return_consistent_results`
- `test_explore_stage_has_no_gates`

## Error Path Tests

### Workspace Name Generation Errors

- `test_build_workspace_name_rejects_empty_run_id`
- `test_build_workspace_name_rejects_whitespace_only_run_id`
- `test_build_workspace_name_rejects_empty_stage`
- `test_build_workspace_name_rejects_whitespace_only_stage`
- `test_build_workspace_name_rejects_zero_attempt`
- `test_build_workspace_name_rejects_control_characters_in_run_id`
- `test_build_workspace_name_rejects_control_characters_in_stage`
- `test_build_workspace_name_rejects_oversized_workspace_name`

### Gate Execution Errors

- `test_zjj_sync_status_gate_fails_on_non_zero_exit_code`
- `test_zjj_merge_queue_gate_fails_on_command_timeout`
- `test_gate_execution_rejects_unsupported_gate_type`
- `test_gate_execution_handles_zjj_not_found`
- `test_invalid_gate_command_returns_error`

## Edge Case Tests

### Workspace Name Generation Edges

- `test_build_workspace_name_handles_very_long_valid_run_id`
- `test_build_workspace_name_handles_very_long_valid_stage`
- `test_build_workspace_name_handles_max_attempt_value`
- `test_build_workspace_name_handles_special_characters_in_run_id`
- `test_build_workspace_name_handles_special_characters_in_stage`
- `test_build_workspace_name_collapses_consecutive_special_characters`

### Gate Execution Edges

- `test_gate_timeout_respects_configured_limits`
- `test_zjj_gate_has_shorter_timeout_than_moon_gates`
- `test_gate_preserves_stdout_and_stderr_on_failure`

### Stage-Gate Edges

- `test_all_stages_return_valid_gate_lists`
- `test_witness_stage_has_single_holdout_gate`
- `test_implementation_stage_has_compiles_and_tests_pass_gates`

## Contract Verification Tests

### Workspace Naming Invariants

- `test_workspace_name_always_starts_with_oya_prefix`
- `test_workspace_name_contains_normalized_run_id_segment`
- `test_workspace_name_contains_normalized_stage_segment`
- `test_workspace_name_ends_with_attempt_suffix`
- `test_workspace_name_length_never_exceeds_64_chars`
- `test_workspace_name_contains_only_valid_ascii_chars`

### Gate Execution Invariants

- `test_gate_evidence_always_contains_command_field`
- `test_gate_evidence_always_contains_passed_field`
- `test_gate_evidence_always_contains_exit_code_field`
- `test_gate_evidence_always_contains_output_field`
- `test_non_zero_exit_code_implies_passed_false`
- `test_gate_timeout_is_enforced`

### Stage-Gate Invariants

- `test_shipgate_stage_always_includes_zjj_merge_queue`
- `test_zjj_merge_queue_is_unique_to_shipgate_stage`
- `test_all_gates_have_valid_string_representations`

## Given-When-Then Scenarios

### Scenario 1: Valid workspace name generation

**Given**:
- run_id = "RUN-123"
- stage = "Implementation"
- attempt = 1

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Ok(workspace_name)`
- workspace_name starts with "oya-"
- workspace_name contains "run-123"
- workspace_name contains "implementation"
- workspace_name ends with "-a1"
- workspace_name length <= 64

---

### Scenario 2: Workspace name normalizes special characters

**Given**:
- run_id = "  Test@Run#ID  "
- stage = "QA_Stage"
- attempt = 2

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Ok(workspace_name)`
- workspace_name = "oya-test-run-id-qa-stage-a2"
- All special characters replaced with hyphens
- Consecutive hyphens collapsed to single hyphen

---

### Scenario 3: Empty run_id is rejected

**Given**:
- run_id = "   " (whitespace only)
- stage = "contract"
- attempt = 1

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Err(CoordinationError::EmptyWorkspaceField("run_id"))`

---

### Scenario 4: Zero attempt is rejected

**Given**:
- run_id = "valid-run"
- stage = "tdd15"
- attempt = 0

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Err(CoordinationError::InvalidAttempt(0))`

---

### Scenario 5: Control characters are rejected

**Given**:
- run_id = "run\u{0007}id" (contains ASCII 7 - bell)
- stage = "plan"
- attempt = 1

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Err(CoordinationError::WorkspaceInvalidContent("run_id"))`

---

### Scenario 6: Oversized workspace name is rejected

**Given**:
- run_id = "r".repeat(45)
- stage = "gpt_review"
- attempt = 10

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Err(CoordinationError::WorkspaceNameTooLong(..., 64))`

---

### Scenario 7: ZjjSyncStatus gate executes successfully

**Given**:
- zjj CLI is installed
- Current directory is within jj repository
- Database is clean (no pending changes)

**When**:
- `execute_zjj_sync_status_gate(repo_root)` is called

**Then**:
- Result is `Ok(evidence)`
- evidence.command = "zjj sync --status"
- evidence.passed = true
- evidence.exit_code = 0
- evidence.output contains sync status information

---

### Scenario 8: ZjjSyncStatus gate fails on dirty state

**Given**:
- zjj CLI is installed
- Database is stale (changes not flushed)

**When**:
- `execute_zjj_sync_status_gate(repo_root)` is called

**Then**:
- Result is `Ok(evidence)`
- evidence.passed = false
- evidence.exit_code != 0
- evidence.output describes sync issue

---

### Scenario 9: ZjjMergeQueue gate included in ShipGate stage

**Given**:
- Stage = StageName::ShipGate

**When**:
- `stage.gates()` is called

**Then**:
- Result contains `Gate::ZjjMergeQueue`
- Result contains `Gate::CueArtifactGenerated`
- Result length = 2

---

### Scenario 10: Explore stage has no gates

**Given**:
- Stage = StageName::Explore

**When**:
- `stage.gates()` is called

**Then**:
- Result is empty vector

---

### Scenario 11: All stages have valid string representations

**Given**:
- All stages in StageName enum

**When**:
- `stage.as_str()` is called for each

**Then**:
- All results are non-empty strings
- All results are snake_case
- No result contains spaces

---

### Scenario 12: Stage transitions follow correct sequence

**Given**:
- Stage = StageName::Contract

**When**:
- `stage.next()` is called

**Then**:
- Result is `Some(StageName::Red)`

**Given**:
- Stage = StageName::ShipGate

**When**:
- `stage.next()` is called

**Then**:
- Result is `None`

---

### Scenario 13: Gate command parsing recognizes zjj sync status

**Given**:
- command = "zjj sync --status"

**When**:
- `parse_gate_command(command)` is called

**Then**:
- Result is `Ok(GateCommand::ZjjSyncStatus)`

---

### Scenario 14: Invalid gate command is rejected

**Given**:
- command = "zjj invalid-command"

**When**:
- `parse_gate_command(command)` is called

**Then**:
- Result is `Err(CoordinationError::InvalidGateCommand { command })`

---

### Scenario 15: Gate timeout respects zjj limits

**Given**:
- Gate = Gate::ZjjMergeQueue

**When**:
- Gate timeout is checked

**Then**:
- Timeout = 60 seconds (ZJJ_TIMEOUT_SECONDS)
- Moon gates use 900 seconds (MOON_TIMEOUT_SECONDS)

---

### Scenario 16: Max attempts is always 2

**Given**:
- Any stage from StageName enum

**When**:
- `stage.max_attempts()` is called

**Then**:
- Result is always 2

---

### Scenario 17: Model tier varies by stage

**Given**:
- Stage = StageName::Explore

**When**:
- `stage.model_for_stage()` is called

**Then**:
- Result is `ModelTier::Fast`

**Given**:
- Stage = StageName::ShipGate

**When**:
- `stage.model_for_stage()` is called

**Then**:
- Result is `ModelTier::Best`

---

### Scenario 18: Workspace name is deterministic

**Given**:
- Same inputs: run_id="test-run", stage="plan", attempt=1

**When**:
- `build_zjj_workspace_name()` called twice

**Then**:
- Both calls return identical `Ok(workspace_name)`

---

### Scenario 19: Gate evidence preserves command output

**Given**:
- Gate execution produces stdout/stderr

**When**:
- Gate completes (pass or fail)

**Then**:
- `GateEvidence.output` contains combined stdout and stderr
- `GateEvidence.command` contains executed command

---

### Scenario 20: Empty after normalization is rejected

**Given**:
- run_id = "---" (only special characters)
- stage = "plan"
- attempt = 1

**When**:
- `build_zjj_workspace_name(run_id, stage, attempt)` is called

**Then**:
- Result is `Err(CoordinationError::WorkspaceInvalidFormat("run_id"))`
