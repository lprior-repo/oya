# OYA Architecture Master Plan

## Outcome

Build a governance-first orchestration platform where high-throughput AI execution is controlled by deterministic policy, explicit quality gates, and auditable decisions.

## Locked Decisions

1. OpenCode is used as a CLI subprocess execution adapter.
2. Restate is the orchestration runtime and authority.
3. zjj is the workspace isolation and merge-flow primitive.
4. Moon is the CI/CD wrapper command surface.
5. Beads (`br`) are the intake and lifecycle source of truth.
6. Sled is the persistence baseline until replacement is justified.
7. No UI/frontend stream is in scope for now.

## Authority Boundaries

- OYA/Restate decide stage transitions, retries, and terminal outcomes.
- OpenCode Adapter executes stage prompts and returns output only.
- Moon executes validation gates and returns gate evidence.
- zjj manages workspace isolation and merge-flow lifecycle.
- Sled persists canonical run and evidence state.

## System Context

### Control Plane

- OYA + Restate manage run state transitions, retries, and terminal states.
- Stage transitions are policy-governed and persisted.

### Execution Plane

- OpenCode Adapter runs as subprocess per stage attempt.
- Outputs are parsed into strict stage result contracts.

### Isolation Plane

- zjj provides per-run/per-stage workspaces.
- Sync, merge, and completion/abort are explicit lifecycle actions.

### Validation Plane

- Moon runs repository gates and emits pass/fail evidence.
- Operator-facing gate commands are `moon run ...`.

### Evidence Plane

- Sled stores runs, attempts, results, artifacts, and decisions.

## Canonical Stage DAG

1. `contract`
2. `tdd15`
3. `qa`
4. `red_queen`
5. `gpt_review`
6. `ship_gate`

Default transitions:

- `contract` pass -> `tdd15`
- `tdd15` pass -> `qa`
- `qa` pass -> `red_queen`
- `red_queen` pass -> `gpt_review`
- `gpt_review` pass -> `ship_gate`
- `qa`/`red_queen`/`gpt_review` fail -> `tdd15` retry lane

Terminal states:

- `shipped`
- `blocked`
- `failed`
- `aborted`

## OpenCode Integration Contract

- OpenCode Adapter is called as subprocess for each stage attempt.
- Context packet includes `run_id`, `bead_id`, `stage`, `attempt`, and required artifact summaries.
- Structured output parsing is mandatory.
- Parse failures become typed `output_parse_failure` outcomes.

## Failure Model

Primary failure categories include:

- `pending_permission`
- `pending_question`
- `test_failed`
- `test_not_executed`
- `test_infra_failed`
- `compile_failed`
- `lint_failed`
- `merge_conflict`
- `main_unhealthy`
- `rate_limited`
- `auth_failed`
- `context_overflow`
- `provider_unavailable`
- `output_parse_failure`

All failures are typed, persisted, and used to drive deterministic transitions.

## Quality Gates and Ship Logic

Ship requires all of the following:

1. Mandatory stages pass.
2. No critical policy violations.
3. Required Moon gates pass.
4. Artifact trail is complete.
5. Ship rationale payload is persisted.

No manual vibe overrides.

## Moon Gate Surface

- `moon run :quick`
- `moon run :ci`
- `moon run :ci --force`

Direct cargo commands are not operator-facing workflow commands in this repo.

## Data Model (Minimum)

- `bead_runs`: run lifecycle
- `stage_attempts`: stage + attempt state
- `stage_results`: structured outputs and failure categories
- `artifacts`: logs, reports, and evidence references
- `gate_results`: Moon gate outcomes
- `decisions`: ship/no-ship with rationale and approval mode

Idempotency key format:

- `run_id:stage:attempt`

## Throughput and WIP

- Target active beads in parallel: 6
- One Restate workflow per bead run
- Bounded execution worker pool
- Provider-aware retry and backoff

## Non-Goals (Current)

- UI/frontend productization
- replacing or removing zjj
- speculative multi-framework support
- UX polish ahead of governance correctness

## Immediate Next Moves

1. Keep docs and beads aligned to this file as source of truth.
2. Continue implementation in stream order A -> B -> C -> D -> F.
3. Enforce Moon gates and typed failure evidence for every stage run.
4. Keep all architectural documentation consistent with `docs/UBIQUITOUS_LANGUAGE.md`.
