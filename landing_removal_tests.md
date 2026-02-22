# Martin Fowler Test Plan: Remove zjj Landing and Workspace Execution Paths

## Test Organization
- **Location**: `src/main/tests/landing_removal_tests.rs` (new file)
- **Test module**: `mod landing_removal_tests;` in `src/main.rs`
- **Fixture**: Test helper functions for creating mock contexts and state

---

## Happy Path Tests

### test_landing_plane_executes_remaining_steps_after_zjj_removal
**Given**: A completed implementation stage artifact with workspace=None
**When**: `run_landing_plane` is called
**Then**:
- moon_ci step executes successfully
- br_close step executes successfully
- br_sync_flush_only step executes successfully
- All step telemetry is persisted
- Run is marked as completed (status="shipped")
- Returns Ok(())

### test_landing_step_executes_with_idempotency
**Given**: A landing step that has not been executed
**When**: `run_landing_step` is called with moon_ci step
**Then**:
- Command executes: `moon run :ci`
- Execution succeeds (exit_code=0)
- Telemetry is persisted with "landing_step_moon_ci" key
- Returns Ok(())

**Given**: A landing step that was already executed
**When**: `run_landing_step` is called again
**Then**:
- Command does NOT execute (skipped)
- Existing telemetry is read
- Returns Ok(())

### test_resolve_landing_run_root_returns_repo_root_without_workspace
**Given**: Runtime config with repo_root="/path/to/repo"
**Given**: Stage artifact with workspace=None
**When**: `resolve_landing_run_root(config, artifact)` is called
**Then**:
- Returns PathBuf("/path/to/repo")
- Does NOT attempt to resolve workspace path

### test_resolve_execution_root_returns_repo_root_when_workspace_none
**Given**: repo_root="/path/to/repo"
**Given**: workspace=None
**When**: `resolve_execution_root(repo_root, workspace)` is called
**Then**:
- Returns PathBuf("/path/to/repo")

### test_prepare_workspace_lifecycle_always_returns_none
**Given**: Valid StageExecutionInput with stage=Implementation
**Given**: RuntimeConfig with workspace_policy=Skip
**When**: `prepare_workspace_lifecycle(ctx, input, config)` is called
**Then**:
- Returns Ok(None)
- Does NOT call zjj commands
- Does NOT create workspace

### test_build_stage_artifact_sets_workspace_to_none
**Given**: StageArtifactData with workspace=None
**When**: `build_stage_artifact(data)` is called
**Then**:
- Returns StageArtifact with workspace=None
- All other fields populated correctly

### test_stage_artifact_serializes_with_null_workspace
**Given**: StageArtifact with workspace=None
**When**: Artifact is serialized to JSON
**Then**:
- JSON contains `"workspace": null`
- All other fields present

### test_stage_artifact_deserializes_with_null_workspace
**Given**: JSON artifact with `"workspace": null`
**When**: JSON is deserialized to StageArtifact
**Then**:
- Artifact.workspace is None
- All other fields populated

---

## Error Path Tests

### test_landing_step_fails_when_command_returns_non_zero_exit
**Given**: A landing step for br_close
**When**: `run_landing_step` is called and br close fails with exit_code=1
**Then**:
- Returns Err(LandingFailure {
    failure_category: FailureCategory::OutputParseFailure,
    next_stage: Stage::ShipGate,
    output: contains command and stderr
  })

### test_landing_plane_returns_landing_failure_on_step_failure
**Given**: moon_ci step fails with exit_code=1
**When**: `run_landing_plane` is called
**Then**:
- Returns Err(LandingFailure)
- failure_category matches moon_ci failure
- next_stage = Stage::Implementation
- output contains failure details

### test_landing_step_fails_on_command_timeout
**Given**: A landing step with timeout_seconds=30
**When**: Command exceeds timeout
**Then**:
- Returns Err(LandingFailure)
- output contains timeout message

---

## Edge Case Tests

### test_landing_step_with_empty_output_still_persists_telemetry
**Given**: A landing step that succeeds with empty stdout/stderr
**When**: `run_landing_step` is called
**Then**:
- Telemetry is persisted
- stdout="" and stderr=""
- passed=true

### test_landing_step_with_large_output_truncates_correctly
**Given**: A landing step with 10KB stdout
**When**: `run_landing_step` is called
**Then**:
- Telemetry persisted with stdout truncated to 4000 chars
- stderr truncated to 4000 chars
- truncate_clean markers preserved

### test_resolve_landing_run_root_handles_empty_workspace_path
**Given**: Artifact with workspace=Some(WorkspaceLifecycle { path: "" })
**When**: `resolve_landing_run_root(config, artifact)` is called
**Then**:
- Returns repo_root (empty path is treated as None)

### test_resolve_execution_root_ignores_workspace_path
**Given**: repo_root="/repo"
**Given**: workspace=Some(WorkspaceLifecycle { path: "/workspace/xyz" })
**When**: `resolve_execution_root(repo_root, workspace)` is called
**Then**:
- Returns PathBuf("/repo")
- Ignores workspace path

### test_prepare_workspace_lifecycle_with_skip_policy_returns_none
**Given**: config.workspace_policy = Skip
**When**: `prepare_workspace_lifecycle(ctx, input, config)` is called
**Then**:
- Returns Ok(None)
- No zjj commands executed

### test_prepare_workspace_lifecycle_for_non_workspace_stage_returns_none
**Given**: stage=Explore (not a workspace stage)
**When**: `prepare_workspace_lifecycle(ctx, input, config)` is called
**Then**:
- Returns Ok(None)

---

## Contract Verification Tests

### test_precondition_workspace_preparation_always_skipped
**Given**: Any stage and attempt
**When**: `prepare_workspace_lifecycle` is called
**Then**:
- Returns Ok(None)
- No zjj commands are executed
- No workspace paths are resolved

### test_postcondition_stage_artifact_workspace_always_none
**Given**: StageExecutionResult from any stage
**When**: StageArtifact is built
**Then**:
- artifact.workspace is None
- No workspace data persisted

### test_postcondition_landing_steps_exclude_zjj
**Given**: LANDING_STEPS constant
**When**: Steps are enumerated
**Then**:
- No step has id starting with "zjj_"
- Remaining steps: moon_ci, br_close, br_sync_flush_only
- Count is 3 (was 5)

### test_postcondition_execution_root_always_repo_root
**Given**: Any stage artifact
**When**: `resolve_execution_root` is called
**Then**:
- Returns repo_root
- Never returns workspace path

### test_postcondition_landing_telemetry_excludes_zjj
**Given**: All landing steps completed
**When**: Landing telemetry keys are queried
**Then**:
- No key contains "zjj_sync" or "zjj_done"
- Keys: landing_step_moon_ci, landing_step_br_close, landing_step_br_sync_flush_only

### test_invariant_backward_compatibility_with_old_artifacts
**Given**: JSON artifact with workspace data (from pre-removal)
**When**: JSON is deserialized to StageArtifact
**Then**:
- Deserialization succeeds
- workspace field can be Some or None
- Old artifacts remain readable

---

## Integration Tests

### test_full_landing_plane_workflow_after_zjj_removal
**Given**: A run at ShipGate stage with all stages completed
**When**: Pipeline reaches `completed_stage_next_action` and calls `run_landing_plane`
**Then**:
- moon_ci executes (1800s timeout)
- br close executes (60s timeout)
- br sync --flush-only executes (60s timeout)
- Run marked as completed
- Timeline event emitted: RunShipped
- All telemetry persisted
- Returns Ok(())

### test_landing_plane_retry_on_failure
**Given**: moon_ci fails with test failures
**When**: `run_landing_plane` returns Err(LandingFailure)
**When**: Pipeline handles failure and transitions to Implementation stage
**Then**:
- state.current_stage = Implementation
- state.attempt = 1
- last_failure set correctly
- Next iteration retries landing plane after Implementation completes

### test_multiple_landing_attempts_with_idempotency
**Given**: moon_ci completed on first attempt
**When**: Pipeline retries landing plane (e.g., after br close fails)
**Then**:
- moon_ci step skipped (telemetry shows completed)
- br_close re-executed
- br_sync_flush_only re-executed

---

## Given-When-Then Scenarios

### Scenario 1: Successful landing plane execution
Given: Pipeline reaches final stage (Witness or ShipGate)
And: All stages completed successfully
And: workspace=None in stage artifact

When: `completed_stage_next_action` calls `run_landing_plane`

Then:
- moon_ci step executes with command "moon run :ci"
- moon_ci telemetry persisted at key "landing_step_moon_ci"
- br_close step executes with command "br close <bead_id>"
- br_close telemetry persisted at key "landing_step_br_close"
- br_sync_flush_only step executes with command "br sync --flush-only"
- br_sync_flush_only telemetry persisted at key "landing_step_br_sync_flush_only"
- `mark_run_completed` is called
- orchestrator status set to "shipped"
- RunCompleted event emitted

### Scenario 2: Landing step failure and retry
Given: moon_ci step fails with exit_code=1
And: stderr contains "test failed: assertion failed"

When: `run_landing_plane` receives moon_ci failure

Then:
- LandingFailure returned with:
  - failure_category = FailureCategory::TestFailed
  - next_stage = Stage::Implementation
  - output contains command and stderr
- `completed_stage_next_action` returns Err(LandingFailure)
- Pipeline transitions to Implementation stage
- last_failure set with TestFailed category
- Next iteration retries Implementation stage

### Scenario 3: Workspace preparation is skipped
Given: Any stage (Contract, Implementation, etc.)
And: RuntimeConfig loaded

When: `execute_and_accumulate_stage` calls `prepare_workspace_lifecycle`

Then:
- Function returns Ok(None)
- No zjj commands executed (no zjj add, queue, abort)
- WorkspaceLifecycleEvent not created
- StageArtifact.workspace = None

### Scenario 4: Idempotent landing step execution
Given: moon_ci step already executed and persisted
And: Telemetry exists at key "landing_step_moon_ci" with passed=true

When: Pipeline retries landing plane (after br close failure)

Then:
- moon_ci command NOT executed
- Existing telemetry read from Restate state
- `landing_step_completed` returns true
- Step skipped
- Execution continues to br_close

### Scenario 5: Execution root is always repo root
Given: Any stage execution
And: repo_root = "/home/user/project"

When: `execute_and_accumulate_stage` resolves execution root

Then:
- `prepare_workspace_lifecycle` returns None
- `resolve_execution_root` receives None workspace
- Returns PathBuf("/home/user/project")
- Stage execution runs in repository root
- No workspace path used

### Scenario 6: Backward compatibility with old artifacts
Given: Restate state contains old StageArtifact with workspace data
And: JSON: `{"workspace": {"name": "ws-123", "path": "/repo/__workspaces/ws-123", ...}}`

When: Artifact is deserialized to StageArtifact type

Then:
- Deserialization succeeds
- artifact.workspace = Some(WorkspaceLifecycle { ... })
- Artifact is readable and usable
- No errors when accessing workspace field

### Scenario 7: Landing telemetry does not contain zjj data
Given: Pipeline completed all landing steps successfully

When: Restate state is queried for landing telemetry keys

Then:
- Key "landing_step_moon_ci" exists (moon_ci telemetry)
- Key "landing_step_br_close" exists (br close telemetry)
- Key "landing_step_br_sync_flush_only" exists (br sync telemetry)
- Key "landing_step_zjj_sync" does NOT exist
- Key "landing_step_zjj_done" does NOT exist
- Total landing keys = 3

---

## Test Data Fixtures

### Mock Landing Steps
```rust
fn mock_moon_ci_step() -> CommandStep {
    CommandStep {
        id: "moon_ci".to_string(),
        label: "moon ci".to_string(),
        program: "moon".to_string(),
        args: vec!["run".to_string(), ":ci".to_string()],
        timeout_seconds: 1800,
        failure_category: FailureCategory::TestFailed,
        next_stage: Stage::Implementation,
    }
}

fn mock_br_close_step(bead_id: &str) -> CommandStep {
    CommandStep {
        id: "br_close".to_string(),
        label: "br close".to_string(),
        program: "br".to_string(),
        args: vec!["close".to_string(), bead_id.to_string()],
        timeout_seconds: 60,
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::ShipGate,
    }
}

fn mock_br_sync_flush_step() -> CommandStep {
    CommandStep {
        id: "br_sync_flush_only".to_string(),
        label: "br sync --flush-only".to_string(),
        program: "br".to_string(),
        args: vec!["sync".to_string(), "--flush-only".to_string()],
        timeout_seconds: 60,
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::ShipGate,
    }
}
```

### Mock State
```rust
fn mock_pipeline_state() -> PipelineState {
    PipelineState {
        orchestrator: OrchestratorState {
            status: "running".to_string(),
            stage: "witness".to_string(),
            attempt: 1,
            bead_id: "src-test".to_string(),
            context: "test".to_string(),
            model: "gpt-5".to_string(),
            last_failure: String::new(),
            last_output: String::new(),
            last_prompt: String::new(),
            updated_at: "2026-02-22T00:00:00Z".to_string(),
        },
        current_stage: Stage::Witness,
        attempt: 1,
        last_failure: None,
        resolved_models: std::collections::HashMap::new(),
        red_seal_ready: true,
    }
}

fn mock_stage_artifact() -> StageArtifact {
    StageArtifact {
        stage: "witness".to_string(),
        attempt: 1,
        failure_category: None,
        next_stage: None,
        timing: StageTiming {
            started_at: "2026-02-22T00:00:00Z".to_string(),
            completed_at: "2026-02-22T00:05:00Z".to_string(),
            duration_ms: 300_000,
        },
        workspace: None,
        input: StageInputData {
            run_id: "test-run".to_string(),
            bead_id: "src-test".to_string(),
            context: "test".to_string(),
            model: "gpt-5".to_string(),
            last_failure: None,
        },
        prompt: "test prompt".to_string(),
        output: StageOutputData {
            success: true,
            exit_code: 0,
            full_log: "test output".to_string(),
            feedback: "Success".to_string(),
            contract_document: None,
            implementation_code: None,
            test_results: None,
            adversarial_report: None,
        },
        task_tracking: None,
        gates: Vec::new(),
        status: StageStatus::Completed,
    }
}
```

---

## Test Implementation Order

1. **Unit tests for helper functions**
   - test_resolve_landing_run_root_returns_repo_root_without_workspace
   - test_resolve_execution_root_returns_repo_root_when_workspace_none
   - test_prepare_workspace_lifecycle_always_returns_none
   - test_build_stage_artifact_sets_workspace_to_none

2. **Unit tests for landing steps**
   - test_landing_step_executes_with_idempotency
   - test_landing_step_fails_when_command_returns_non_zero_exit
   - test_landing_step_with_empty_output_still_persists_telemetry
   - test_landing_step_with_large_output_truncates_correctly

3. **Unit tests for landing plane**
   - test_landing_plane_executes_remaining_steps_after_zjj_removal
   - test_landing_plane_returns_landing_failure_on_step_failure

4. **Contract verification tests**
   - test_precondition_workspace_preparation_always_skipped
   - test_postcondition_stage_artifact_workspace_always_none
   - test_postcondition_landing_steps_exclude_zjj
   - test_postcondition_execution_root_always_repo_root
   - test_postcondition_landing_telemetry_excludes_zjj
   - test_invariant_backward_compatibility_with_old_artifacts

5. **Integration tests**
   - test_full_landing_plane_workflow_after_zjj_removal
   - test_landing_plane_retry_on_failure
   - test_multiple_landing_attempts_with_idempotency

6. **Edge case tests**
   - All edge case tests from above section
