# Quality Gates

This repo currently enforces gates through lifecycle workflow logic and moon tasks.

## Gate Layers

1. **Input validation**
   - Reject invalid repo slugs/models.
   - Reject invalid lifecycle DAGs (missing dependencies, cycles, out-of-order dependencies).

2. **Execution validation**
   - `opencode` must complete successfully.
   - `moon_ci` must complete successfully.

3. **Diff validation**
   - Workspace diff cannot be empty.
   - Workspace diff cannot contain only `.beads` changes.

4. **Delivery validation**
   - Bookmark creation/push must succeed.
   - PR creation must succeed.

## Commands

```bash
moon run :quick
moon run :test
moon run :ci
```

## Observability

```bash
oya status --key <workflow_or_bead_key>
```

Status snapshots include step-level progress and terminal outcome (`done`, `success`, `message`).
