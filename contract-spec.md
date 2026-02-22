# Contract Specification

## Context
- **Feature**: Remove ZJJ merge queue gate enum and configuration branches
- **Domain terms**:
  - `Gate`: Enumeration of pipeline quality gates (compiles, tests_pass, moon_ci, holdout_scenarios, cue_artifact_generated)
  - `GateCommand`: Parsed command representation (Moon tasks only after ZJJ removal)
  - `MergeQueuePolicy`: Runtime policy for ZJJ merge queue enforcement (TO BE REMOVED)
  - `RuntimeConfig`: Pipeline runtime configuration (merge_queue_policy TO BE REMOVED)
  - `StageName`: Pipeline stage enumeration (ShipGate gates TO BE REDUCED)
- **Assumptions**:
  - The ZJJ merge queue functionality is being deprecated or replaced
  - All tests for ZJJ-specific behavior must pass after removal (or be removed)
  - Backward compatibility is not required (breaking change acceptable)
- **Open questions**:
  - Should `OYA_SKIP_ZJJ_GATE` and `OYA_SKIP_ZJJ_WORKSPACE` env vars also be removed?
  - Should `Gate::MoonCi` be used as a replacement for the ZJJ gate?
  - Is `StageName::ShipGate` still valid with only `CueArtifactGenerated` gate?

## Preconditions

### Removal Operations
- Precondition: The codebase must compile before removal
- Precondition: All existing tests for ZJJ functionality must be identified
- Precondition: Any consumers of `Gate::ZjjMergeQueue` must be identified and updated

### RuntimeConfig Loading
- Precondition (BEFORE): RuntimeConfig loads OYA_DISABLE_ZJJ, OYA_SKIP_ZJJ_GATE, OYA_SKIP_ZJJ_WORKSPACE
- Precondition (AFTER): RuntimeConfig no longer loads these env vars
- Precondition: OYA_REPO_ROOT must still be readable

### Gate Execution
- Precondition: All non-ZJJ gates must continue to work
- Precondition: Moon gates must still parse and execute correctly
- Precondition: Revision validation for moon gates must still work

## Postconditions

### Type System
- Postcondition: `Gate` enum has 5 variants (not 6): Compiles, TestsPass, MoonCi, HoldoutScenarios, CueArtifactGenerated
- Postcondition: `Gate::ZjjMergeQueue` variant does NOT exist
- Postcondition: `Gate::as_str()` does NOT return "zjj_merge_queue"
- Postcondition: `TryFrom<&str>` for Gate does NOT accept "zjj_merge_queue"

### Stage Configuration
- Postcondition: `StageName::ShipGate.gates()` returns `[Gate::CueArtifactGenerated]` only
- Postcondition: ZJJ-related gates are not present in any stage's gate list

### Runtime Configuration
- Postcondition: `RuntimeConfig` struct has NO `merge_queue_policy` field
- Postcondition: `MergeQueuePolicy` enum does NOT exist
- Postcondition: `RuntimeConfig::load()` does NOT read OYA_DISABLE_ZJJ, OYA_SKIP_ZJJ_GATE, OYA_SKIP_ZJJ_WORKSPACE
- Postcondition: `RuntimeConfig` has only `workspace_policy` and `repo_root` fields

### Gate Command Parsing
- Postcondition: `GateCommand` enum has ONLY `Moon` variant
- Postcondition: `GateCommand::ZjjSyncStatus` variant does NOT exist
- Postcondition: `parse_gate_command_parts()` does NOT handle zjj commands
- Postcondition: Unsupported zjj commands return `OyaError`

### Gate Execution
- Postcondition: `execute_gate()` uses MOON_TIMEOUT_SECONDS for ALL gates
- Postcondition: `ZJJ_TIMEOUT_SECONDS` constant is not used
- Postcondition: ZJJ-specific revision handling is removed

### Failure Mapping
- Postcondition: `gate_failure_mapping()` has NO entry for `Gate::ZjjMergeQueue`
- Postcondition: ZJJ-specific failure routing is removed

### Tests
- Postcondition: All tests compile
- Postcondition: All non-ZJJ tests pass
- Postcondition: No test references `Gate::ZjjMergeQueue`
- Postcondition: No test references `MergeQueuePolicy`
- Postcondition: No test references `ZjjSyncStatus`

## Invariants

### Type Safety
- Invariant: All `Gate` enum variants are valid pipeline quality checks
- Invariant: Each stage's gates are appropriate for that stage's purpose

### Runtime Configuration
- Invariant: RuntimeConfig must be loadable from environment
- Invariant: RuntimeConfig fields are all required and validated

### Gate Execution
- Invariant: All gates have executable commands
- Invariant: All gates have defined failure outcomes
- Invariant: Gate commands are parseable and executable

## Error Taxonomy

### Domain Errors (Preserved)
- `OyaError::ParseError` - when gate command cannot be parsed
- `OyaError::ExecutionError` - when gate command fails to execute
- `OyaError::RevisionError` - when git revision validation fails

### Error Removal
- `OyaError` variants specific to ZJJ queue should be removed
- Error handling for ZJJ sync status should be removed

## Contract Signatures

### Types Module (src/types/pipeline.rs)

```rust
// BEFORE:
pub enum Gate {
    Compiles,
    TestsPass,
    MoonCi,
    HoldoutScenarios,
    ZjjMergeQueue,  // TO BE REMOVED
    CueArtifactGenerated,
}

// AFTER:
pub enum Gate {
    Compiles,
    TestsPass,
    MoonCi,
    HoldoutScenarios,
    CueArtifactGenerated,
}

// BEFORE:
impl StageName {
    pub fn gates(&self) -> Vec<Gate> {
        match self {
            // ...
            Self::ShipGate => vec![Gate::CueArtifactGenerated, Gate::ZjjMergeQueue],  // REMOVE ZjjMergeQueue
        }
    }
}

// AFTER:
impl StageName {
    pub fn gates(&self) -> Vec<Gate> {
        match self {
            // ...
            Self::ShipGate => vec![Gate::CueArtifactGenerated],
        }
    }
}
```

### Pipeline Module (src/pipeline/mod.rs)

```rust
// BEFORE:
pub(super) struct RuntimeConfig {
    pub(super) workspace_policy: WorkspacePreparationPolicy,
    pub(super) merge_queue_policy: MergeQueuePolicy,  // TO BE REMOVED
    pub(super) repo_root: PathBuf,
}

#[derive(Clone, Copy)]
pub(super) enum MergeQueuePolicy {  // TO BE REMOVED
    Enforce,
    Skip,
}

// AFTER:
pub(super) struct RuntimeConfig {
    pub(super) workspace_policy: WorkspacePreparationPolicy,
    pub(super) repo_root: PathBuf,
}

// BEFORE:
impl RuntimeConfig {
    pub(super) async fn load(ctx: &WorkflowContext<'_>) -> Result<Self, OyaError> {
        let disable_zjj = Self::read_flag(ctx, "OYA_DISABLE_ZJJ").await?;
        let (skip_zjj_workspace, skip_zjj_gate) =
            Self::read_zjj_skip_flags(ctx, disable_zjj).await?;  // TO BE REMOVED
        // ...
        Ok(Self {
            workspace_policy: WorkspacePreparationPolicy::from_skip_flag(skip_zjj_workspace),
            merge_queue_policy: MergeQueuePolicy::from_skip_flag(skip_zjj_gate),  // TO BE REMOVED
            repo_root: PathBuf::from(repo_root_str),
        })
    }
}

// AFTER:
impl RuntimeConfig {
    pub(super) async fn load(ctx: &WorkflowContext<'_>) -> Result<Self, OyaError> {
        let repo_root_str = Self::stable_repo_root(ctx).await.map_err(|error| {
            OyaError(format!(
                "config error resolving repo root (OYA_REPO_ROOT or current_dir): {}",
                error
            ))
        })?;

        Ok(Self {
            workspace_policy: WorkspacePreparationPolicy::from_skip_flag(false),  // Simplify or remove
            repo_root: PathBuf::from(repo_root_str),
        })
    }
}
```

### Gates Module (src/runtime_tools/gates.rs)

```rust
// BEFORE:
pub(crate) fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    let command = generate_moon_command(&gate).command;
    let timeout_seconds = match gate {
        Gate::ZjjMergeQueue => ZJJ_TIMEOUT_SECONDS,  // TO BE REMOVED
        _ => MOON_TIMEOUT_SECONDS,
    };
    // ...
}

// AFTER:
pub(crate) fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    let command = generate_moon_command(&gate).command;
    let timeout_seconds = MOON_TIMEOUT_SECONDS;  // Constant for all gates
    // ...
}

// BEFORE:
pub(crate) enum GateCommand {
    Moon { task: MoonTask, passthrough: Vec<String> },
    ZjjSyncStatus,  // TO BE REMOVED
}

// AFTER:
pub(crate) enum GateCommand {
    Moon { task: MoonTask, passthrough: Vec<String> },
}

// BEFORE:
fn parse_gate_command_parts(command: ParsedCommandParts) -> Result<GateCommand, OyaError> {
    match (command.program.as_str(), command.args.as_slice()) {
        ("moon", moon_args) => parse_moon_gate_command(moon_args),
        ("zjj", zjj_args) if zjj_args == ["sync", "--status"] => Ok(GateCommand::ZjjSyncStatus),  // TO BE REMOVED
        _ => Err(OyaError(format!(
            "unsupported gate command: {} {}",
            command.program,
            command.args.join(" ")
        ))),
    }
}

// AFTER:
fn parse_gate_command_parts(command: ParsedCommandParts) -> Result<GateCommand, OyaError> {
    match (command.program.as_str(), command.args.as_slice()) {
        ("moon", moon_args) => parse_moon_gate_command(moon_args),
        _ => Err(OyaError(format!(
            "unsupported gate command: {} {}",
            command.program,
            command.args.join(" ")
        ))),
    }
}

// BEFORE:
impl GateCommand {
    fn command_parts(&self) -> (String, Vec<String>) {
        match self {
            GateCommand::Moon { task, passthrough } => {
                let args = std::iter::once("run".to_string())
                    .chain(std::iter::once(task.as_task_name().to_string()))
                    .chain(passthrough.iter().cloned())
                    .collect();
                ("moon".to_string(), args)
            }
            GateCommand::ZjjSyncStatus => {  // TO BE REMOVED
                ("zjj".to_string(), vec!["sync".to_string(), "--status".to_string()])
            }
        }
    }
}

// AFTER:
impl GateCommand {
    fn command_parts(&self) -> (String, Vec<String>) {
        match self {
            GateCommand::Moon { task, passthrough } => {
                let args = std::iter::once("run".to_string())
                    .chain(std::iter::once(task.as_task_name().to_string()))
                    .chain(passthrough.iter().cloned())
                    .collect();
                ("moon".to_string(), args)
            }
        }
    }
}

// BEFORE:
fn gate_failure_mapping(stage: &Stage, gate: &Gate) -> Option<(FailureCategory, Stage)> {
    match (stage, gate) {
        // ...
        (&Stage::ShipGate, &Gate::ZjjMergeQueue) => {  // TO BE REMOVED
            Some((FailureCategory::MergeConflict, Stage::Implementation))
        }
        _ => None,
    }
}

// AFTER:
fn gate_failure_mapping(stage: &Stage, gate: &Gate) -> Option<(FailureCategory, Stage)> {
    match (stage, gate) {
        // ...
        _ => None,
    }
}
```

## Non-goals
- NOT adding new gate types to replace ZjjMergeQueue
- NOT preserving backward compatibility with ZJJ-based workflows
- NOT maintaining ZJJ-related test coverage
- NOT adding new runtime configuration options
