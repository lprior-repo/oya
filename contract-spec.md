# Contract Specification

## Context
- Feature: Fix workspace cleanup to remove filesystem directory without alternate VCS cleanup commands
- Domain terms:
  - `Compensation::ForgetWorkspace` - enum variant that triggers cleanup
  - `WorkspaceName` - identifier for the workspace
- `run_compensation` - function that executes compensation effects
  - `remove_workspace_dir` - function that removes directory from filesystem
- Assumptions:
  - WorkspaceName provides a way to get the filesystem path
  - The cleanup runs after PR is opened via lifecycle finalize
- Open Questions:
  - How exactly does WorkspaceName provide the filesystem path?

## Preconditions
- [ ] `Compensation::ForgetWorkspace` receives valid `WorkspaceName`
- [ ] WorkspaceName can resolve to a valid filesystem path
- [ ] Workspace directory is owned by the current Oya run

## Postconditions
- [ ] Workspace filesystem directory is removed
- [ ] Compensation result indicates success or failure with diagnostic
- [ ] If directory removal fails, error is captured in diagnostic

## Invariants
- [ ] Cleanup success means the workspace directory no longer exists
- [ ] Cleanup never depends on an alternate VCS binary
- [ ] Cleanup failure preserves a workspace diagnostic

## Error Taxonomy
- `LifecycleError::Terminal(FailureCategory::Workspace)` - when directory removal fails

## Contract Signatures
```rust
fn run_compensation(
    executor: &dyn CommandExecutor,
    compensation: Compensation,
) -> anyhow::Result<EffectJournalEntry>
```

## Type Encoding
| Precondition | Enforcement Level | Type / Pattern |
|---|---|---|
| WorkspaceName valid | Runtime | `WorkspaceName::from_str() -> Result` |
| path exists | Runtime | `Path::exists()` check |

## Violation Examples
- VIOLATES <P1>: Calling `run_compensation` with `Compensation::ForgetWorkspace { workspace: invalid_workspace }` where workspace path is invalid -- should return `Err` with a workspace diagnostic
- VIOLATES <P2>: Calling with valid workspace but directory removal fails due to permissions -- should return `Err` with diagnostic showing removal failure

## Ownership Contracts
- `executor: &dyn CommandExecutor` - shared borrow, read-only, no mutation
- `compensation: Compensation` - owned value, consumed by function

## Non-goals
- [ ] Reverting changes made during lifecycle execution
- [ ] Handling network failures (not applicable)
