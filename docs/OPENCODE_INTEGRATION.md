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

- zjj remains the isolation and merge-flow primitive.
- No stream is active to replace or remove zjj.

## Scope and Non-Goals (Current)

- No UI/frontend stream is in scope.
- No desktop/web operator console work is in scope.
- Focus is governance correctness, deterministic transitions, and reliable gate evidence.
