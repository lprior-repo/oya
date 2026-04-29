# Git Worktrees

Active Oya workflows use Git/GitHub only for version-control, isolation, and PR delivery.

Use a Git branch for normal work:

```bash
git switch -c <branch>
```

Use a Git worktree only when physical directory isolation is required:

```bash
git worktree add ../<workspace> <branch>
```
