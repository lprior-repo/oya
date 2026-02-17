# OYA Implementation Plan: manual-e2e-bead

## Objective

Execute a manual end-to-end orchestrator runbook that proves canonical stage flow, retry policy behavior, idempotent start handling, gate evidence persistence, and ship-gate command ordering for `manual-e2e-bead`.

## Codebase Alignment Snapshot

- Canonical stage order: `research -> plan -> contract -> tdd15 -> qa -> red_queen -> gpt_review -> ship_gate`.
- Stage max attempts: `3` (`StageName::max_attempts`).
- Required object handlers: `start`, `get_status`, `ping`.
- Duplicate `start` with same run key is ignored (idempotent behavior).
- Retryable failures: `test_failed`, `test_infra_failed`, `compile_failed`, `lint_failed`, `merge_conflict`, `rate_limited`.
- Non-retryable failures: `auth_failed`, `context_overflow`, `provider_unavailable`, `output_parse_failure`, `max_attempts_exceeded`.
- Runtime gate mapping:
  - `compiles -> moon run :check`
  - `tests_pass|edge_cases|no_vulnerabilities -> moon run :test`
  - `clippy_clean|security -> moon run :quick`
  - `moon_ci -> moon run :ci`
  - `zjj_merge_queue -> zjj done --dry-run`
- Ship gate execution order in runtime code path: `moon run :ci` -> `zjj done --dry-run` -> `moon run :quick` -> `moon run :test`.
- Command timeouts: `opencode=300s`, `moon=900s`, `zjj=60s`.

## Exact Implementation Steps

1. Prepare deterministic run IDs and evidence directories.
   - Run IDs:
     - `manual-e2e-happy-001`
     - `manual-e2e-retry-001`
     - `manual-e2e-nonretry-001`
   - Create:
     - `tmp/manual-e2e-bead/happy`
     - `tmp/manual-e2e-bead/retryable`
     - `tmp/manual-e2e-bead/nonretryable`
     - `tmp/manual-e2e-bead/logs`
     - `tmp/manual-e2e-bead/wrappers`
   - Base object URL: `http://127.0.0.1:9080/OyaOrchestrator`.

2. Start local runtime dependencies.
   - Start Restate locally using the repo standard local workflow.
   - Start OYA and capture logs:
     - `OYA_REPO_ROOT=/home/lewis/src/oya OYA_BIND_ADDR=127.0.0.1:9080 moon run :run`
   - Persist stdout/stderr to `tmp/manual-e2e-bead/logs/oya-baseline.log`.

3. Run hard pre-flight quality gates.
   - `moon run :check`
   - `moon run :test`
   - `moon run :quick`
   - Stop runbook immediately on first failure.

4. Verify object reachability for each run key.
   - For each run ID, call:
     - `POST {base}/{run_id}/ping`
   - Save response as:
     - `tmp/manual-e2e-bead/happy/ping.json`
     - `tmp/manual-e2e-bead/retryable/ping.json`
     - `tmp/manual-e2e-bead/nonretryable/ping.json`
   - Assert exact payload: `{"status":"ok","service":"OyaOrchestrator"}`.

5. Execute happy-path scenario (`manual-e2e-happy-001`).
   - Start run:
     - `POST {base}/manual-e2e-happy-001/start`
     - body: `{"bead_id":"manual-e2e-bead","context":"manual end-to-end pipeline test"}`
   - Poll status every 2s:
     - `POST {base}/manual-e2e-happy-001/get_status`
     - stop only on terminal state.
   - Assertions:
     - terminal state is `Shipped`
     - stage completion order is exactly `Research, Plan, Contract, Tdd15, Qa, RedQueen, GptReview, ShipGate`
   - Idempotency assertion:
     - repeat the exact same `start` request on `manual-e2e-happy-001`
     - verify no duplicate execution timeline/attempt stream is created.
   - Save request/response transcript and poll timeline under `tmp/manual-e2e-bead/happy`.

6. Execute retryable-failure scenario (`manual-e2e-retry-001`).
   - Create PATH wrapper for `moon` in `tmp/manual-e2e-bead/wrappers/moon`:
     - fail first `moon run :test` invocation once (non-zero)
     - pass-through to real `moon` for all later invocations.
   - Restart OYA with wrapper-prepended PATH and same `OYA_REPO_ROOT`, `OYA_BIND_ADDR`.
   - Save process log to `tmp/manual-e2e-bead/logs/oya-retryable.log`.
   - Start run and poll to terminal as in step 5.
   - Assertions:
     - retry attempts increment on failed stage and never exceed `3`
     - run eventually reaches `Shipped`
     - observed failure category is retryable (`test_failed` preferred; any retryable category acceptable).
   - Save wrapper, command trace, responses, and timeline under `tmp/manual-e2e-bead/retryable`.

7. Execute non-retryable-failure scenario (`manual-e2e-nonretry-001`).
   - Create PATH wrapper for `opencode` in `tmp/manual-e2e-bead/wrappers/opencode` that exits non-zero with parse-like failure output.
   - Restart OYA with wrapper-prepended PATH and same `OYA_REPO_ROOT`, `OYA_BIND_ADDR`.
   - Save process log to `tmp/manual-e2e-bead/logs/oya-nonretryable.log`.
   - Start run and poll to terminal as in step 5.
   - Assertions:
     - terminal state is `Failed`
     - no retry loop is scheduled after non-retryable classification
     - failure category includes `output_parse_failure` (preferred) or another non-retryable category.
   - Save wrapper, command trace, responses, and timeline under `tmp/manual-e2e-bead/nonretryable`.

8. Extract persisted DB evidence for all scenarios.
   - Use repo-local Rust helper (small bin or test helper) that reads `OyaDb` directly.
   - Emit:
     - `tmp/manual-e2e-bead/happy/db-evidence.json`
     - `tmp/manual-e2e-bead/retryable/db-evidence.json`
     - `tmp/manual-e2e-bead/nonretryable/db-evidence.json`
   - Each JSON must include:
     - all `StageAttempt` rows
     - all `StageResult` rows
     - all `GateResult` rows
     - all `Artifact` rows filtered to `quality_gate_report`
   - Assert each attempted stage has corresponding gate evidence and stage result records.

9. Validate ship-gate command order and merge-failure interpretation.
   - Using logs plus DB evidence, prove ship-gate runtime sequence is:
     1. `moon run :ci`
     2. `zjj done --dry-run`
     3. `moon run :quick`
     4. `moon run :test`
   - For failing merge checks, verify `zjj done --dry-run` constraint-pattern output (`CHECK constraint failed`, `closed_at`, `status`) is classified as merge failure behavior.

10. Run post-run confidence gate.
    - Execute `moon run :ci`.
    - Mark runbook failed if CI fails.

11. Produce final evidence report.
    - Create `tmp/manual-e2e-bead/report.md` containing:
      - per-run outcome
      - evidence file index (all captured JSON/log files)
      - pass/fail matrix for stage order, retry behavior, non-retry behavior, idempotency, persistence completeness, ship-gate ordering
      - unresolved issues and follow-ups.

## Test Strategy

- Happy-path validates deterministic canonical progression and terminal `Shipped` state.
- Retryable-path validates bounded retries (`<=3`) and successful recovery.
- Non-retryable-path validates immediate terminal failure and absence of retry scheduling.
- Idempotency check validates duplicate `start` as no-op for same run key.
- Persistence audit validates `StageAttempt`, `StageResult`, `GateResult`, and `quality_gate_report` artifact traceability.
- Ship-gate audit validates command execution order and merge-failure classification behavior.

## Quality Gates

Pre-flight gates (must pass before scenarios):

- `moon run :check`
- `moon run :test`
- `moon run :quick`

Runtime gates (must be present in persisted evidence):

- `moon run :check` for `compiles`
- `moon run :test` for `tests_pass`, `edge_cases`, `no_vulnerabilities`
- `moon run :quick` for `clippy_clean`, `security`
- `moon run :ci` for `moon_ci`
- `zjj done --dry-run` for `zjj_merge_queue`

Post-run gate:

- `moon run :ci`

## Acceptance Criteria

- `manual-e2e-happy-001` completes as `Shipped` with canonical stage order.
- `manual-e2e-retry-001` completes as `Shipped` with retries bounded at `3`.
- `manual-e2e-nonretry-001` completes as `Failed` without retry loop.
- Duplicate `start` on same run key does not create duplicate execution.
- Persisted evidence exists for `StageAttempt`, `StageResult`, `GateResult`, `Artifact(quality_gate_report)`.
- Ship-gate command order evidence matches runtime implementation.
- `tmp/manual-e2e-bead/report.md` exists and references all evidence artifacts.

## Out of Scope

- Frontend/UI validation.
- Refactoring orchestrator internals.
- Tooling replacement for Restate, Moon, or zjj.
