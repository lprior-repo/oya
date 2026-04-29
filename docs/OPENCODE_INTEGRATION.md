# OpenCode Integration

## Overview

OYA uses OpenCode as a CLI subprocess adapter to get AI outputs for governed stage execution.

- Restate is the orchestration authority.
- OYA owns stage transitions, retry policy, typed failures, and terminal decisions.
- OpenCode executes prompts and returns structured outputs.

Related deep-dive:

- `docs/OPENCODE_AUTOMATION_LEARNINGS.md`
- `docs/QA_RESTATE_VALIDATION.md`

## Canonical Stage DAG

The only supported stage flow is:

1. `contract`
2. `tdd15`
3. `qa`
4. `red_queen`
5. `gpt_review`
6. `ship_gate`

Default retry lane:

- `qa`, `red_queen`, or `gpt_review` failures route back to `tdd15`.

## Execution Model

Each stage attempt is orchestrated by Restate and executed by OpenCode via subprocess.

```text
Bead run request
  -> Restate workflow (run_id, stage, attempt)
  -> Build context packet
  -> Invoke opencode CLI subprocess
  -> Parse structured output
  -> Persist artifacts + gate evidence
  -> Transition next stage or failure state
```

## Subprocess Contract

- OpenCode is invoked per stage attempt.
- Stage boundary means fresh execution context by default.
- All outputs must parse into the stage response contract.
- Non-parseable output is `output_parse_failure`.

## Request/Response Expectations

- Use strict structured prompts with explicit acceptance criteria.
- Require evidence payloads where applicable (`qa`, `red_queen`, `gpt_review`, `ship_gate`).
- Reject placeholder evidence (`todo`, `n/a`, `not run`).

## Requirements

- `opencode` CLI installed and available in `PATH`.
- Restate service available for workflow execution (Docker-first local default via `scripts/dev-up.sh`).
- Sled available for local persistence.

## Local Runtime (Default)

Use Docker-first runtime commands:

- `scripts/dev-up.sh` starts Restate (Docker), builds OYA, starts OYA service, and registers deployment.
- `scripts/dev-down.sh` stops local runtime.
- `scripts/dev-reset.sh` clears local Restate state when replay history conflicts with new workflow code.

For live QA validation workflow details (including ingress handler checks and deployment staleness
troubleshooting), see `docs/QA_RESTATE_VALIDATION.md`.

## CI/CD and Gates

Moon is the CI/CD wrapper command surface for this repo.

- Use `moon run :quick` for fast local checks.
- Use `moon run :ci` for full quality gates.
- Use `moon run :ci --force` for uncached confidence runs.

Do not document direct cargo commands as operator-facing gate commands.

## Workspace Isolation

- Git branches are the default isolation primitive.
- Git worktrees are used only when physical directory isolation is required.
- GitHub PRs are the merge-flow primitive.

## Phase 2: Git + OpenCode Polling

OYA ties stage execution to Git branch/worktree lifecycle for implementation stages and exposes a
small ops-monitor service for OpenCode observability.

### Git workspace lifecycle

- For `contract`, `tdd15`, `qa`, `red_queen`, `gpt_review`, and `ship_gate`, OYA runs:
  1. `git fetch origin`
  2. `git rebase origin/main`
  3. optional `git worktree add <path> <branch>` when physical isolation is required.
- Branch/worktree names are deterministic: `oya-<run_id>-<stage>-a<attempt>` (normalized and validated).
- OYA persists Git command evidence in stage state and timeline.

### OpenCode monitor endpoints

`Oya` service exposes:

- `poll_status`: snapshots OpenCode `/session/status`, `/permission`, and `/question`.
- `poll_events`: long-poll proxy for OpenCode `/event`, returns bounded parsed event payloads.

Environment variables:

- `OYA_OPENCODE_BASE_URL` (default: `http://127.0.0.1:4097`)
- `OYA_OPENCODE_PASSWORD` (optional; used as basic auth password for user `opencode`)

### CLI ops-poll command

Run a continuous terminal poller against OpenCode:

```bash
oya ops-poll
```

Environment variables:

- `OYA_OPENCODE_BASE_URL` (default: `http://127.0.0.1:4097`)
- `OYA_OPENCODE_PASSWORD` (optional)
- `OYA_POLL_INTERVAL_MS` (default: 2000, clamped to 500-30000)

Output format: `HH:MM:SS.mmm | <busy_sessions> | <pending_permissions> | <pending_questions>`

## Scope and Non-Goals (Current)

- No UI/frontend stream is in scope.
- No desktop/web operator console work is in scope.
- Focus is governance correctness, deterministic transitions, and reliable gate evidence.
