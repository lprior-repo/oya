# PLAN: src-2a6 - Governance Hard-Gate with Moon Check Clippy Test Sequence

## Overview
Implement strict sequential governance gate that executes `moon run :check` → `moon run :clippy` → `moon run :test` in fixed order with short-circuit failure. First failing command returns raw output for retry.

## Current State
- `src/types/pipeline.rs:115-120` - Gate enum has: Compiles, TestsPass, MoonCi, ZjjMergeQueue
- `src/beads/moon_command.rs:33-46` - `generate_moon_command` maps gates to commands
- `src/runtime_tools/gates.rs:18-36` - `execute_gate` runs single command, captures output
- `src/runtime_tools/gates.rs:38-53` - MoonTask enum has: Check, Test, Ci (missing Clippy)
- No sequential gate execution with short-circuit exists

## Requirements (from CUE spec)
1. **Fixed order**: check → clippy → test (non-negotiable)
2. **Short-circuit**: First failure stops evaluation, returns raw output
3. **Raw output**: Never summarize - return verbatim command output
4. **No agent authority**: Agent cannot run or skip governance commands

## Implementation Tasks

### Phase 1: Add Clippy MoonTask
**File**: `src/runtime_tools/gates.rs`

1. Add `Clippy` variant to `MoonTask` enum (line 49):
```rust
#[derive(Clone, Copy)]
pub(crate) enum MoonTask {
    Check,
    Clippy,  // NEW
    Test,
    Ci,
}
```

2. Update `from_task_name` (line 102):
```rust
":clippy" => Some(Self::Clippy),
```

3. Update `as_task_name` (line 111):
```rust
MoonTask::Clippy => ":clippy",
```

### Phase 2: Add GovernanceGate Type
**File**: `src/types/pipeline.rs`

1. Add new gate variant (after line 119):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gate {
    Compiles,
    TestsPass,
    MoonCi,
    ZjjMergeQueue,
    Governance,  // NEW: sequential check → clippy → test
}
```

2. Update `as_str` and `TryFrom<&str>` implementations for Gate

### Phase 3: Implement Sequential Governance Runner
**File**: `src/runtime_tools/gates.rs`

1. Add governance step tracking:
```rust
#[derive(Debug, Clone)]
pub(crate) struct GovernanceStep {
    pub(crate) task: MoonTask,
    pub(crate) passed: bool,
    pub(crate) output: String,
}

pub(crate) struct GovernanceResult {
    pub(crate) steps: Vec<GovernanceStep>,
    pub(crate) first_failure: Option<(MoonTask, String)>,
    pub(crate) passed: bool,
}
```

2. Add `execute_governance_gate` function:
```rust
pub(crate) fn execute_governance_gate(repo_root: &PathBuf) -> Result<GovernanceResult, OyaError> {
    let sequence: [MoonTask; 3] = [MoonTask::Check, MoonTask::Clippy, MoonTask::Test];
    let mut steps = Vec::new();
    let mut first_failure: Option<(MoonTask, String)> = None;

    for task in sequence {
        let evidence = execute_single_governance_step(task, repo_root)?;
        if !evidence.passed && first_failure.is_none() {
            first_failure = Some((task, evidence.output.clone()));
        }
        steps.push(GovernanceStep {
            task,
            passed: evidence.passed,
            output: evidence.output,
        });
        if !evidence.passed {
            break; // SHORT-CIRCUIT
        }
    }

    Ok(GovernanceResult {
        passed: first_failure.is_none(),
        steps,
        first_failure,
    })
}
```

3. Add helper `execute_single_governance_step`:
```rust
fn execute_single_governance_step(
    task: MoonTask,
    repo_root: &PathBuf,
) -> Result<GateEvidence, OyaError> {
    let command = format!("moon run :{}", task.as_task_name());
    let parsed = parse_gate_command(&command)?;
    let (program, args) = parsed.command_parts();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (passed, stdout, stderr, exit_code) = run_command_with_timeout_with_exit(
        program.as_str(),
        &arg_refs,
        MOON_TIMEOUT_SECONDS,
        repo_root,
    )?;
    Ok(GateEvidence {
        command,
        passed,
        exit_code,
        output: combine_command_output(stdout, stderr),
    })
}
```

### Phase 4: Update Moon Command Generation
**File**: `src/beads/moon_command.rs`

1. Add Governance case to `generate_moon_command` (line 34):
```rust
Gate::Governance => ("governance", "Run governance gate (check → clippy → test)", "governance"),
```

### Phase 5: Integrate with execute_gate
**File**: `src/runtime_tools/gates.rs`

1. Update `execute_gate` to handle Governance:
```rust
pub(crate) fn execute_gate(gate: Gate, repo_root: &PathBuf) -> Result<GateEvidence, OyaError> {
    match gate {
        Gate::Governance => {
            let result = execute_governance_gate(repo_root)?;
            match result.first_failure {
                Some((task, output)) => Ok(GateEvidence {
                    command: format!("governance:{}:failed", task.as_task_name()),
                    passed: false,
                    exit_code: 1,
                    output,  // RAW OUTPUT for retry
                }),
                None => Ok(GateEvidence {
                    command: "governance:passed".to_string(),
                    passed: true,
                    exit_code: 0,
                    output: result.steps.iter()
                        .map(|s| format!("{}: ✓", s.task.as_task_name()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                }),
            }
        }
        _ => {
            // existing logic
            let command = generate_moon_command(&gate).command;
            // ...
        }
    }
}
```

### Phase 6: Add Tests
**File**: `src/runtime_tools/gates.rs` (append tests)

1. `test_governance_check_fails_short_circuits` - check fails, clippy/test not run
2. `test_governance_clippy_fails_short_circuits` - check passes, clippy fails, test not run
3. `test_governance_all_pass` - all three pass
4. `test_governance_returns_raw_output` - failure output is verbatim

**File**: `src/beads/moon_command.rs` (append tests)

1. `test_generate_governance_command` - Governance gate maps correctly

## Test Strategy

### Unit Tests (RED first)
1. **gates.rs**:
   - `test_governance_check_fails_returns_check_output` - Raw check output on check failure
   - `test_governance_clippy_fails_returns_clippy_output` - Raw clippy output on clippy failure  
   - `test_governance_test_fails_returns_test_output` - Raw test output on test failure
   - `test_governance_all_pass_returns_success` - All three pass returns passed=true
   - `test_governance_short_circuits_on_first_failure` - Never runs steps after failure

2. **moon_command.rs**:
   - `test_generate_governance_command` - Maps Gate::Governance to "governance" task

### Integration Tests
- Requires mocking command execution - use existing `run_command_with_timeout_with_exit` pattern

## Quality Gates
1. `moon run :check` - Compiles
2. `moon run :clippy` - No warnings
3. `moon run :test` - All tests pass (RED → GREEN)

## Verification Commands
```bash
moon run :check && moon run :clippy && moon run :test
```

## Files to Modify
| File | Changes |
|------|---------|
| `src/types/pipeline.rs` | Add Gate::Governance variant |
| `src/runtime_tools/gates.rs` | Add MoonTask::Clippy, GovernanceResult, execute_governance_gate |
| `src/beads/moon_command.rs` | Add Governance case |

## Invariants
- Gate order ALWAYS: check → clippy → test
- Short-circuit ALWAYS: first failure stops
- Output ALWAYS: raw verbatim output from failing command
- Agent NEVER: can run or skip governance commands

## Risk Mitigation
- Existing gates (Compiles, TestsPass, MoonCi, ZjjMergeQueue) unchanged
- Governance is additive - no breaking changes
- Tests verify short-circuit behavior explicitly
