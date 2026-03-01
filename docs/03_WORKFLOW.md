# Workflow: Pull -> Isolate -> Verify -> Merge

1. **Pull**: `bv` discover new beads.
2. **Isolate**: `jj workspace add <workspace>`.
3. **Verify**: `moon run :ci --force`.
4. **Merge**: `jj bookmark create <name>` + `git push`.
