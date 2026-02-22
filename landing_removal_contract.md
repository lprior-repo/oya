# Contract Specification: Remove zjj Landing and Workspace Execution Paths

## Context
- **Feature**: Remove zjj workspace orchestration and landing steps from the OYA orchestrator
- **Domain terms**:
  - `zjj` - Jujutsu-based workspace orchestration tool (being removed)
  - `landing plane` - Post-implementation quality gate steps (moon ci, zjj sync, zjj done, br close, br sync --flush-only)
  - `workspace lifecycle` - zjj workspace creation, queuing, and management (being removed)
  - `landing steps` - Individual commands executed after implementation completes

- **Assumptions**:
  1. zjj workspace execution is being removed entirely
  2. Landing plane will continue with moon_ci, br close, and br sync --flush-only only
  3. Workspace preparation will be completely removed (no workspace creation/management)
  4. Configuration flags (OYA_SKIP_ZJJ_WORKSPACE, OYA_SKIP_ZJJ_GATE) will be ignored or deprecated
  5. Workspace-related types (WorkspaceLifecycle, WorkspaceLifecycleEvent) will be removed from artifacts
  6. Stage artifacts will no longer contain workspace data

- **Open questions**:
  1. Should `WorkspacePreparationPolicy` be kept or removed? (Used in executor.rs)
  2. Should `MergeQueuePolicy` be kept or removed? (Used in stage_executor.rs for gates)

## Preconditions
- [ ] Pipeline execution is enabled (OYA_ENABLE_PIPELINE_EXECUTION=1)
- [ ] A valid bead exists with bead_id, context, and model
- [ ] Restate workflow context is available
- [ ] Repository root path is configured

## Postconditions
- [ ] zjj landing steps (zjj_sync, zjj_done) are removed from LANDING_STEPS
- [ ] Landing plane executes only: moon_ci, br close, br sync --flush-only
- [ ] Workspace preparation returns None (no zjj workspace creation)
- [ ] Stage artifacts contain `workspace: None`
- [ ] WorkspaceLifecycle and WorkspaceLifecycleEvent types are no longer used
- [ ] resolve_landing_run_root uses repo_root directly (no workspace path)

## Invariants
- [ ] Stage artifacts must serialize/deserialize without workspace field or with workspace=Null
- [ ] Landing plane must not call zjj commands
- [ ] Execution root is always repo_root (never a workspace path)
- [ ] Landing steps telemetry does not include zjj step data
- [ ] All existing stage artifacts in Restate state remain readable (backward compatibility)

## Error Taxonomy

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorRemovalError {
    /// Workspace preparation skipped (intentional - not an error)
    WorkspaceSkipped,

    /// Landing step execution failed
    LandingStepFailed {
        step_id: String,
        step_label: String,
        exit_code: i32,
        output: String,
    },

    /// Repository root resolution failed
    RepoRootResolutionFailed {
        reason: String,
    },

    /// State persistence failed
    StatePersistenceFailed {
        key: String,
        reason: String,
    },
}
```

### Error variants and when they occur:

1. **WorkspaceSkipped** (Informational, not a failure)
   - When: Workspace preparation is called
   - Reason: Workspace preparation is disabled (zjj removed)
   - Action: Continue with repo_root as execution root

2. **LandingStepFailed**
   - When: moon_ci, br close, or br sync --flush-only fails
   - Reason: Command exit_code != 0
   - Fields: step_id, step_label, exit_code, output
   - Action: Transition to Implementation stage for retry

3. **RepoRootResolutionFailed**
   - When: OYA_REPO_ROOT is invalid or current_dir() fails
   - Reason: Cannot determine repository location
   - Fields: reason
   - Action: Terminal workflow failure

4. **StatePersistenceFailed**
   - When: Restate set() or append_durable_event() fails
   - Reason: Restate storage error
   - Fields: key, reason
   - Action: Terminal workflow failure

## Contract Signatures

### Landing Plane

```rust
/// Execute landing plane steps (moon_ci, br close, br sync --flush-only)
///
/// Preconditions:
/// - Final artifact is available (stage completed)
/// - Repository root is configured
///
/// Postconditions:
/// - All landing steps executed in order
/// - Telemetry persisted for each step
/// - Run marked as completed if all steps pass
///
/// Returns:
/// - Ok(()) if all steps succeed
/// - Err(LandingFailure) if any step fails
pub async fn run_landing_plane(
    ctx: &WorkflowContext<'_>,
    state: &PipelineState,
    config: &RuntimeConfig,
    artifact: &StageArtifact,
) -> Result<(), LandingFailure>;

/// Execute a single landing step with idempotency
///
/// Preconditions:
/// - Step command is defined
/// - Telemetry key is unique per step
///
/// Postconditions:
/// - Step executed if not already completed
/// - Telemetry persisted
///
/// Returns:
/// - Ok(()) if step passes or already completed
/// - Err(LandingFailure) if step fails
async fn run_landing_step(
    ctx: &WorkflowContext<'_>,
    repo_root: &Path,
    step: CommandStep,
) -> Result<(), LandingFailure>;

/// Resolve execution root for landing commands
///
/// Preconditions:
/// - Repository root is configured
///
/// Postconditions:
/// - Returns repo_root (no workspace path)
///
/// Returns:
/// - PathBuf pointing to repository root
fn resolve_landing_run_root(
    config: &RuntimeConfig,
    artifact: &StageArtifact,
) -> PathBuf;
```

### Workspace Preparation (Removed)

```rust
/// Prepare workspace lifecycle - NOW REMOVED
///
/// Previously prepared zjj workspaces, now always returns None
///
/// Preconditions:
/// - None (workspace preparation disabled)
///
/// Postconditions:
/// - Always returns Ok(None)
///
/// Returns:
/// - Ok(None) - workspace preparation skipped
async fn prepare_workspace_lifecycle(
    ctx: &WorkflowContext<'_>,
    input: &StageExecutionInput<'_>,
    config: &RuntimeConfig,
) -> Result<Option<WorkspaceLifecycle>, OyaError>;
```

### Stage Artifact

```rust
/// Build stage artifact with workspace field set to None
///
/// Preconditions:
/// - Stage execution completed
/// - Workspace lifecycle is None
///
/// Postconditions:
/// - Artifact.workspace is None
/// - All other fields populated correctly
///
/// Returns:
/// - Complete StageArtifact with workspace=None
fn build_stage_artifact(data: StageArtifactData<'_>) -> StageArtifact;
```

### Execution Root Resolution

```rust
/// Resolve execution root for stage commands
///
/// Preconditions:
/// - Repo root is configured
///
/// Postconditions:
/// - Returns repo_root (never workspace path)
///
/// Returns:
/// - PathBuf for repository root
fn resolve_execution_root(
    repo_root: &Path,
    workspace: Option<&WorkspaceLifecycle>,
) -> std::path::PathBuf;
```

## Type Changes

### Removed Types
```rust
// REMOVE: No longer needed without zjj
pub(super) struct WorkspaceLifecycleEvent {
    pub workspace: String,
    pub workspace_path: String,
    pub queue_command: String,
    pub queue_passed: bool,
    pub queue_exit_code: i32,
    pub queue_output: String,
    pub add_command: String,
    pub add_passed: bool,
    pub add_exit_code: i32,
    pub add_output: String,
    pub recorded_at: String,
}

// REMOVE: No longer needed without zjj
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkspaceLifecycle {
    pub name: String,
    pub path: String,
    pub queue_command: String,
    pub queue_passed: bool,
    pub queue_exit_code: i32,
    pub add_command: String,
    pub add_passed: bool,
    pub add_exit_code: i32,
}
```

### Modified Types
```rust
// MODIFY: workspace field now always None
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StageArtifact {
    pub stage: String,
    pub attempt: u32,
    pub failure_category: Option<String>,
    pub next_stage: Option<String>,
    pub timing: StageTiming,
    pub workspace: Option<WorkspaceLifecycle>,  // Will always be None
    pub input: StageInputData,
    pub prompt: String,
    pub output: StageOutputData,
    pub task_tracking: Option<TaskTracking>,
    pub gates: Vec<GateResultData>,
    pub status: StageStatus,
}
```

### Removed Landing Steps
```rust
// REMOVE from LANDING_STEPS:
LandingStepTemplate {
    id: "zjj_sync",
    label: "zjj sync",
    program: "zjj",
    args: &["sync"],
    timeout_seconds: 120,
    failure_category: FailureCategory::MergeConflict,
    next_stage: Stage::Implementation,
},

LandingStepTemplate {
    id: "zjj_done",
    label: "zjj done",
    program: "zjj",
    args: &["done"],
    timeout_seconds: 120,
    failure_category: FailureCategory::MergeConflict,
    next_stage: Stage::Implementation,
},
```

## Configuration Changes

### Deprecated Environment Variables
These variables will be ignored:
- `OYA_DISABLE_ZJJ` - zjj is permanently disabled
- `OYA_SKIP_ZJJ_WORKSPACE` - workspace prep always skipped
- `OYA_SKIP_ZJJ_GATE` - zjj gate doesn't exist

### Simplified RuntimeConfig
```rust
pub(super) struct RuntimeConfig {
    pub(super) workspace_policy: WorkspacePreparationPolicy,  // Always Skip
    pub(super) merge_queue_policy: MergeQueuePolicy,         // Always Skip
    pub(super) repo_root: PathBuf,
}
```

## Non-goals
- [ ] Removing `WorkspacePreparationPolicy` enum (kept for type compatibility)
- [ ] Removing `MergeQueuePolicy` enum (kept for gate compatibility)
- [ ] Modifying existing Restate state containing old workspace artifacts (must remain readable)
- [ ] Changing landing step execution logic (only removing zjj steps)
- [ ] Changing artifact serialization format (workspace field remains for backward compatibility)
