# OpenCode Integration

## Overview

Oya runs OpenCode as one lifecycle effect inside a deterministic step graph.

- Restate/Oya workflow is the orchestrator and source of truth.
- OpenCode is invoked as a subprocess effect (`Effect::Opencode`).
- Step progress is published in lifecycle status snapshots.

## Where It Lives

- Lifecycle DAG and step ordering: `src/lifecycle/workflow.rs`
- Effect execution + timeouts: `src/lifecycle/effects.rs`
- Status API wiring: `src/restate_oya/handlers.rs`

## Current Lifecycle Step Sequence

The runtime builds this step chain:

1. `mark_in_progress`
2. `workspace_prepare`
3. `workspace_add`
4. `opencode`
5. `moon_ci`
6. `jj_sync_main`
7. `jj_rebase_main`
8. `jj_track`
9. `jj_describe`
10. `validate_changes`
11. `bookmark_create`
12. `bookmark_push`
13. `pr_create`

## OpenCode Runtime Contract

- OpenCode receives a bead-specific prompt and model from the lifecycle engine.
- OpenCode runs in the jj workspace for that bead.
- On success, lifecycle proceeds to `moon_ci`; on failure, lifecycle returns classified error state.

## Timeouts and Validation

- OpenCode effect timeout: `1200s` (`OPENCODE_TIMEOUT_SECS`).
- CI effect timeout: `900s` (`MOON_CI_TIMEOUT_SECS`).
- Lifecycle DAG validation runs before execution (unknown dependencies, cycles, and order violations fail fast).

## Operator Commands

```bash
oya lifecycle --bead <id> --repo <owner/repo>
oya status --key <id>
oya cancel --key <id>
```

`oya status` now reports terminal not-found (`done=true`, `success=false`) when no lifecycle exists for the key.
