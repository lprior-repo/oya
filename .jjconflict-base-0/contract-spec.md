# Contract Specification

## Context
- Feature: Fix workspace cleanup to remove filesystem directory
- Domain terms:
  - `Compensation::ForgetWorkspace` - enum variant that triggers cleanup
  - `WorkspaceName` - identifier for the workspace
  - `run_compensation` - function that executes compensation commands
  - `remove_workspace_dir` - function that removes directory from filesystem
- Assumptions:
  - WorkspaceName provides a way to get the filesystem path
  - The cleanup runs after PR is opened via lifecycle finalize
- Open Questions:
  - How exactly does WorkspaceName provide the filesystem path?

## Preconditions
- [ ] `Compensation::ForgetWorkspace` receives valid `WorkspaceName`
- [ ] WorkspaceName can resolve to a valid filesystem path
- [ ] jj is available on the system

## Postconditions
- [ ] `jj workspace forget <workspace>` command is executed successfully
- [ ] Workspace filesystem directory is removed
- [ ] Compensation result indicates success or failure with diagnostic
- [ ] If either operation fails, error is captured in diagnostic

## Invariants
- [ ] Either both jj forget AND directory removal succeed, or both fail with diagnostic
- [ ] No partial state where jj forget succeeds but directory remains
- [ ] No partial state where directory removed but jj forget fails

## Error Taxonomy
- `LifecycleError::Terminal(FailureCategory::Workspace)` - when jj workspace forget fails
- `LifecycleError::Terminal(FailureCategory::Workspace)` - when directory removal fails
- `LifecycleError::Transient(FailureCategory::Command)` - when command times out

## Contract Signatures
```rust
async fn run_compensation(
    executor: &dyn CommandExecutor,
    compensation: Compensation,
) -> anyhow::Result<EffectJournalEntry>
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| WorkspaceName valid | Runtime | `WorkspaceName::from_str() -> Result` |
| jj available | Runtime check | Command returns error if not found |
| path exists | Runtime | `Path::exists()` check |

## Violation Examples
- VIOLATES <P1>: Calling `run_compensation` with `Compensation::ForgetWorkspace { workspace: invalid_workspace }` where workspace doesn't exist -- should return `Err` with diagnostic showing jj failure
- VIOLATES <P2>: Calling with valid workspace but directory removal fails due to permissions -- should return `Err` with diagnostic showing removal failure

## Ownership Contracts
- `executor: &dyn CommandExecutor` - shared borrow, read-only, no mutation
- `compensation: Compensation` - owned value, consumed by function

## Non-goals
- [ ] Reverting changes made during lifecycle execution
- [ ] Handling network failures (not applicable)
