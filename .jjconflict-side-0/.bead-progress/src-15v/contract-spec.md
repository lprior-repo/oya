# Contract Specification: Hard-Remove zjj from Landing Workspace and Ship-Gate

## Purpose and Goals

Remove all zjj dependencies from the landing workspace execution and ship-gate validation. The orchestrator should use only moon and br commands for final validation, eliminating the zjj layer entirely.

## Error Taxonomy

```rust
#[derive(Debug, Error)]
pub enum LandingRemovalError {
    #[error("zjj command still present in landing steps: {step_id}")]
    ZjjStepStillPresent { step_id: String },

    #[error("zjj merge queue gate still configured for ShipGate stage")]
    ZjjGateStillConfigured,

    #[error("zjj workspace preparation still active for stage: {stage}")]
    ZjjWorkspacePrepActive { stage: String },

    #[error("zjj sync status parsing still implemented")]
    ZjjSyncStatusParsingPresent,

    #[error("landing execution failed: missing required step {step_id}")]
    RequiredLandingStepMissing { step_id: String },

    #[error("landing execution failed: moon ci step returned error")]
    MoonCiStepFailed,
}
```

## Contract Signatures

### 1. Landing Steps Array (Static Configuration)

**Location**: `src/main.rs::LANDING_STEPS`

**Invariant**:
- LANDING_STEPS MUST contain exactly 3 steps:
  1. `moon_ci` - moon run :ci
  2. `br_close` - br close <bead_id>
  3. `br_sync_flush` - br sync --flush-only

**Forbidden Elements**:
- MUST NOT contain `zjj_sync` step
- MUST NOT contain `zjj_done` step
- All step programs MUST be either "moon" or "br"

**Type Signature**:
```rust
const LANDING_STEPS: &[LandingStepTemplate] = &[
    LandingStepTemplate {
        id: "moon_ci",
        label: "moon ci",
        program: "moon",
        args: &["run", ":ci"],
        timeout_seconds: 1_800,
        failure_category: FailureCategory::TestFailed,
        next_stage: Stage::Implementation,
    },
    LandingStepTemplate {
        id: "br_close",
        label: "br close",
        program: "br",
        args: &["close"],
        timeout_seconds: 60,
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::ShipGate,
    },
    LandingStepTemplate {
        id: "br_sync_flush",
        label: "br sync --flush-only",
        program: "br",
        args: &["sync", "--flush-only"],
        timeout_seconds: 60,
        failure_category: FailureCategory::OutputParseFailure,
        next_stage: Stage::ShipGate,
    },
];
```

**Preconditions**:
- bead_id is a valid, non-empty string
- br is available in PATH

**Postconditions**:
- All steps execute in order
- moon ci must pass (exit code 0) before proceeding
- br close and br sync execute sequentially
- On failure, stage transitions to Implementation for retry

**Invariants**:
- Step count is always 3
- All programs are moon or br (no zjj)
- Timeouts are positive (>= 60 seconds)

---

### 2. ShipGate Gates (Dynamic Configuration)

**Location**: `src/runtime_tools/gates.rs::gate_failure_mapping`

**Invariant**:
- `Stage::ShipGate.gates()` MUST return only moon-based gates
- MUST NOT include `Gate::ZjjMergeQueue`

**Type Contract**:
```rust
// Stage::ShipGate.gates() implementation
impl Stage {
    pub fn gates(&self) -> Vec<Gate> {
        match self {
            Stage::ShipGate => vec![
                Gate::CueArtifactGenerated,
                // Gate::ZjjMergeQueue // MUST BE REMOVED
            ],
            _ => vec![],
        }
    }
}
```

**Preconditions**:
- Stage is ShipGate
- MergeQueuePolicy is Skip (zjj queue disabled)

**Postconditions**:
- Only moon-based gates are executed
- No zjj commands are invoked
- Gate failure routes to Implementation stage

**Invariants**:
- ShipGate always has at least 1 gate (CueArtifactGenerated)
- All gates use moon program (not zjj)
- Gate failure mapping for ZjjMergeQueue is removed

---

### 3. Gate Command Parsing (GateCommand Enum)

**Location**: `src/runtime_tools/gates.rs::GateCommand`

**Invariant**:
- `GateCommand` enum MUST NOT have `ZjjSyncStatus` variant
- `parse_gate_command_parts` MUST reject zjj commands

**Type Signature**:
```rust
#[derive(Clone)]
pub(crate) enum GateCommand {
    Moon { task: MoonTask, passthrough: Vec<String> },
    // ZjjSyncStatus // MUST BE REMOVED
}

pub(crate) fn parse_gate_command_parts(command: ParsedCommandParts) -> Result<GateCommand, OyaError> {
    match (command.program.as_str(), command.args.as_slice()) {
        ("moon", moon_args) => parse_moon_gate_command(moon_args),
        ("zjj", zjj_args) if zjj_args == ["sync", "--status"] => {
            // MUST RETURN ERROR: "unsupported gate command"
            Err(OyaError("unsupported gate command: zjj sync --status".to_string()))
        }
        _ => Err(OyaError(format!(
            "unsupported gate command: {} {}",
            command.program,
            command.args.join(" ")
        ))),
    }
}
```

**Preconditions**:
- Command string is non-empty
- Program is either "moon" or "zjj"

**Postconditions**:
- moon commands parse to `GateCommand::Moon`
- zjj commands return error `OyaError`
- No silent failures

**Invariants**:
- Only moon commands are supported
- All unsupported programs return descriptive errors

---

### 4. Gate Failure Mapping

**Location**: `src/runtime_tools/gates.rs::gate_failure_mapping`

**Invariant**:
- MUST NOT have mapping for `(Stage::ShipGate, Gate::ZjjMergeQueue)`

**Type Signature**:
```rust
fn gate_failure_mapping(stage: &Stage, gate: &Gate) -> Option<(FailureCategory, Stage)> {
    match (stage, gate) {
        (&Stage::ShipGate, &Gate::CueArtifactGenerated) => {
            Some((FailureCategory::OutputParseFailure, Stage::Implementation))
        }
        // (&Stage::ShipGate, &Gate::ZjjMergeQueue) => {
        //     Some((FailureCategory::MergeConflict, Stage::Implementation))
        // }
        // MUST BE REMOVED
        _ => None,
    }
}
```

**Preconditions**:
- Stage and gate are valid

**Postconditions**:
- ShipGate+CueArtifactGenerated routes to Implementation on failure
- ShipGate+ZjjMergeQueue returns None (not handled)

**Invariants**:
- All ShipGate gate failures route to Implementation
- No zjj-specific failure categories remain

---

### 5. Workspace Preparation (No zjj for Landing)

**Location**: `src/runtime_tools/workspace.rs::prepare_stage_workspace`

**Invariant**:
- ShipGate stage MUST skip workspace preparation
- MUST NOT queue ShipGate workspaces in zjj

**Type Signature**:
```rust
fn stage_uses_workspace(stage: &Stage) -> bool {
    // ShipGate removed from workspace-using stages
    matches!(stage, Stage::Contract | Stage::Implementation)
}

fn stage_requires_merge_queue(stage: &Stage) -> bool {
    // ShipGate no longer requires merge queue
    false
}

pub(crate) fn prepare_stage_workspace(
    request: WorkspacePrepRequest,
) -> Result<Option<WorkspaceLifecycleEvent>, OyaError> {
    // ShipGate returns Ok(None) - no workspace prep
    if request.workspace_policy.should_skip() || !stage_uses_workspace(&request.stage) {
        return Ok(None);
    }
    // ... rest of implementation
}
```

**Preconditions**:
- Request contains valid stage, bead_id, repo_root
- Workspace policy is configured

**Postconditions**:
- ShipGate stage returns Ok(None) immediately
- No zjj commands are executed for ShipGate
- No workspace lifecycle events are emitted for ShipGate

**Invariants**:
- ShipGate never prepares workspaces
- Only Contract and Implementation stages use workspaces
- Workspace policy is respected for all stages

---

## Preconditions Summary

**Global Preconditions**:
- System is in valid state (not corrupted)
- moon CLI is available in PATH
- br CLI is available in PATH
- Git repository is initialized

**Function-Specific Preconditions**:
- `landing_step_from_template`: Template must have all required fields
- `parse_gate_command_parts`: Command must be non-empty
- `prepare_stage_workspace`: repo_root must exist
- `gate_failure_mapping`: Stage and gate must be valid enum variants

## Postconditions Summary

**Global Postconditions**:
- No zjj commands are executed during landing
- No zjj workspaces are created for ShipGate
- ShipGate executes only moon gates
- Landing workflow completes with moon + br commands only

**Function-Specific Postconditions**:
- `landing_step_from_template`: Returns CommandStep with valid program/args
- `parse_gate_command_parts`: Returns GateCommand::Moon or error
- `prepare_stage_workspace`: Returns Ok(None) for ShipGate
- `gate_failure_mapping`: Returns None for zjj gates

## Invariants Summary

**Global Invariants**:
- LANDING_STEPS always has exactly 3 steps
- No zjj programs in landing steps
- ShipGate never uses zjj
- ShipGate never prepares workspaces
- All gate commands are moon-based

**Function-Specific Invariants**:
- `LANDING_STEPS`: count == 3, all programs in {moon, br}
- `Stage::gates()`: ShipGate gates exclude ZjjMergeQueue
- `stage_uses_workspace()`: ShipGate returns false
- `stage_requires_merge_queue()`: Always returns false
- `gate_failure_mapping()`: No (ShipGate, ZjjMergeQueue) entry
