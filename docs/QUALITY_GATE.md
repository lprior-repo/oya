# Quality Gate System

## Current State

Quality gates are enforced through lifecycle execution plus moon tasks, not a standalone `quality_gate` Rust module.

## Active Gates

- **Pre-execution validation**: lifecycle request and DAG validation (model, repo slug, dependency graph).
- **Implementation gate**: OpenCode step must succeed.
- **Verification gate**: `moon_ci` step must succeed.
- **Change gate**: `validate_changes` rejects empty or bead-only diffs.
- **Delivery gate**: bookmark push and PR creation must succeed.

## Primary Command

```bash
moon run :ci
```

Run `moon run :ci --force` for uncached verification.

## Runtime Visibility

Use:

```bash
oya status --key <workflow_or_bead_key>
```

to inspect per-step status and terminal success/failure state.
