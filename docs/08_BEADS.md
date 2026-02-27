# Beads Integration

Use `br` as the work queue and lifecycle source of truth.

## Minimal Flow

```bash
br ready
br show <id>
br update <id> --status in_progress
oya lifecycle --bead <id> --repo <owner/repo>
moon run :ci
br close <id>
br sync --flush-only
```

See [03_WORKFLOW.md](03_WORKFLOW.md) for the full loop.
