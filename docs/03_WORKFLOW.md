# Workflow: Bead -> Lifecycle -> Verify -> Land

1. **Pick work**: `br ready`, then `br show <id>`.
2. **Claim work**: `br update <id> --status in_progress`.
3. **Run implementation flow**: `oya lifecycle --bead <id> --repo <owner/repo>`.
4. **Verify**: `moon run :ci` (use `--force` when needed).
5. **Close + sync bead state**: `br close <id>` then `br sync --flush-only`.
6. **Land with jj**: `jj git fetch`, `jj rebase -d main@origin`, `jj bookmark set <name> -r @`, `jj git push --bookmark <name>`.
