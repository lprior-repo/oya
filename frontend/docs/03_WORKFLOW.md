# Workflow: Pull -> Isolate -> Verify -> Merge

1. **Pull**: `bd ready --json` discovers unblocked beads.
2. **Isolate**: create a Git branch or Git worktree when isolation is required.
3. **Verify**: `moon run :ci --force`.
4. **Merge**: push the Git branch and open a GitHub PR.
