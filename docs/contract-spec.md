# Contract Specification: Git-Beads Coordination

## Status

This is the active VCS coordination contract. Older alternate-VCS contracts are historical only and are not operator guidance.

## Context

- **Feature**: Coordinate Git/GitHub branch delivery with bd issue tracking.
- **Domain Terms**:
  - `run_id`: unique identifier for a pipeline execution.
  - `stage`: current pipeline stage.
  - `attempt`: retry attempt number.
  - `branch`: Git branch name in format `oya-<run_id>-<stage>-a<attempt>`.
  - `gate`: quality gate that must pass before stage completion.
- **Assumptions**:
  - Git is the only active version-control tool.
  - Git worktrees are optional and used only for physical directory isolation.
  - Landing flow uses `git fetch origin`, `git rebase origin/main`, `git push`, and GitHub PR review.

## Preconditions

- Git CLI must be installed and available in PATH.
- Current directory must be within a Git repository.
- `origin` remote must resolve to a GitHub `OWNER/REPO` slug.
- Branch names must be normalized ASCII and <= 64 characters.

## Postconditions

- Branch names are deterministic for identical `run_id`, `stage`, and `attempt` inputs.
- `git fetch origin` and `git rebase origin/main` complete before push.
- `git push -u origin HEAD:<branch>` publishes the review branch.
- bd issue state is closed and synced only after verification gates pass.

## Invariants

- Active Oya workflows do not require an alternate VCS binary.
- All build/test/lint gates run through Moon.
- Failed gates preserve stdout/stderr evidence.
- GitHub PR delivery is the only active merge-flow contract.

## Error Taxonomy

```rust
#[derive(Debug, Error)]
pub enum CoordinationError {
    #[error("git command not found: is git installed?")]
    GitNotFound,

    #[error("git command failed: {command} (exit code: {code})")]
    GitCommandFailed {
        command: String,
        code: i32,
        output: String,
    },

    #[error("invalid GitHub origin remote: {0}")]
    InvalidOrigin(String),

    #[error("gate command execution timed out after {seconds}s")]
    GateTimeout { seconds: u64 },
}
```

## Non-goals

- Supporting alternate VCS workflows.
- Bypassing bd for issue lifecycle state.
- Running direct build/test/lint commands outside Moon.
