# OYA Implementation Plan: bead-cupid

Request context: functional core
Attempt: 1

## Objective

Deliver a deterministic, typed `bead-cupid` functional core in `src/lib.rs` that follows the existing smoke/bead-min contract pattern, while preserving orchestrator workflow behavior, stage policy, retry semantics, and command gates already enforced in the codebase.

## Codebase Alignment Snapshot

- `src/lib.rs` already contains complete functional-core patterns for `Smoke*`, `SmokeBead*`, `LeanBead*`, and `BeadMin*`, including strict input/endpoint/report invariants.
- `src/lib.rs` and `src/main.rs` already enforce safety guardrails (`deny unwrap/expect/panic`, `forbid unsafe`) and must remain unchanged.
- `src/main.rs` must retain orchestrator state/timeline keys, timeout contracts (OpenCode 300s, moon 900s, zjj 60s), and request parsing compatibility (JSON object or JSON string).
- `src/types.rs` must retain canonical stage sequence and policy (`research -> plan -> contract -> tdd15 -> qa -> red_queen -> gpt_review -> ship_gate`, `max_attempts = 3`).
- `docs/RESEARCH_NOTES.md` already captures bead-cupid constraints and is the source of truth for runtime defaults and invariants.

## Exact Implementation Steps

1. Add bead-cupid test scaffolding first in `src/lib.rs` (TDD red phase).
   - Add helper constructor(s) for a valid `CupidReport` in `#[cfg(test)]`.
   - Add failing tests for build/start/capture/evaluate/validate and full pipeline success.

2. Introduce bead-cupid domain types in `src/lib.rs`.
   - Add constants: default runtime command, default ingress health URL, diagnostics max length.
   - Add typed models: `CupidInput`, `CupidPlan`, `CupidRuntimeHandle`, `CupidCheckName`, `CupidCheckObservation`, `CupidObservation`, `CupidStageName`, `CupidStageStatus`, `CupidStageReport`, `CupidDecision`, `CupidReport`, `CupidError`.

3. Implement planning/runtime/observation functions with strict invariants.
   - Implement `build_cupid_plan` with run-id trim, empty/length/charset/control-char checks and deterministic endpoint construction.
   - Implement `start_cupid_runtime` enforcing exact runtime command and strict URL contract validation.
   - Implement `capture_cupid_observation` returning exactly two checks (`IngressHealth`, `OrchestratorStatus`) with non-empty diagnostics and monotonic timestamps.

4. Implement deterministic evaluation/report validation functions.
   - Implement `evaluate_cupid_result` to require exactly one check per type and derive decision strictly from check pass/fail booleans.
   - Implement `validate_cupid_report` enforcing check cardinality, endpoint coherence, diagnostics integrity/length bounds, fixed stage order, monotonic timestamps, and decision/final-stage consistency.
   - Add private helpers for run-id normalization, endpoint contract matching, final diagnostics string, and check-level validation.

5. Keep orchestrator and policy surfaces unchanged.
   - Do not change `src/main.rs` retry categories, state key schema, prompt stage mapping, or gate execution contracts.
   - Do not change `src/types.rs` stage order, gate definitions, or max-attempt policy.

6. Keep docs aligned to implementation truth.
   - Update `docs/RESEARCH_NOTES.md` only if bead-cupid runtime/default invariants differ from implemented behavior.
   - Update `docs/QA_RESTATE_VALIDATION.md` only if new bead-cupid runtime checks need explicit validation steps.

## Test Strategy

- Functional-core unit tests in `src/lib.rs` for bead-cupid:
  - plan validation matrix: empty/whitespace run-id, max-length boundary, oversized run-id, control chars, invalid charset (`../`, query injection), trim normalization.
  - runtime startup validation: invalid runtime command, invalid ingress URL/scheme/credentials/contract mismatch, invalid orchestrator URL/contract mismatch.
  - observation validation: runtime-not-ready path, invalid endpoint/runtime path, exact two-check output, stable check names, non-empty diagnostics, timestamp monotonicity.
  - evaluation validation: missing checks, duplicate checks, deterministic stage order (`IngressHealth -> OrchestratorStatus -> FinalDecision`), decision derivation from check outcomes only.
  - report validation: invalid check counts, invalid endpoints, empty/oversized/control-char diagnostics, invalid stage count/order, non-monotonic timestamps, stage-status mismatch, decision mismatch, final-diagnostics mismatch.

- Regression protection:
  - keep existing `Smoke*`, `SmokeBead*`, `LeanBead*`, and `BeadMin*` tests passing unchanged.
  - keep orchestrator tests for object-or-string request payload compatibility and retry behavior passing unchanged.

- Runtime validation sequence (post-unit tests):
  - `scripts/dev-up.sh`
  - `http://localhost:8080/restate/health`
  - `scripts/pipeline-run.sh <run_id> bead-cupid "functional core"`
  - optional `scripts/dev-reset.sh` for reset/replay checks
  - `scripts/dev-down.sh`

## Quality Gates

- Safety invariants that must stay enforced:
  - `#![deny(clippy::unwrap_used)]`
  - `#![deny(clippy::expect_used)]`
  - `#![deny(clippy::panic)]`
  - `#![forbid(unsafe_code)]`

- Mandatory build/test gates:
  - `moon run :check`
  - `moon run :test`
  - `moon run :quick`
  - `moon run :ci`

- Merge-readiness gate:
  - `zjj done --dry-run` (unless explicitly bypassed with `OYA_SKIP_ZJJ_GATE=1`)

- Release blockers:
  - any bead-cupid drift from default runtime/endpoint contracts.
  - any regression in stage order, retry categories, gate mapping, timeout behavior, or attempt limits.
  - any break in orchestrator request compatibility or state key schema.
  - any introduction of panic/unwrap/expect/unsafe usage.

## Acceptance Criteria

- `bead-cupid` functional core APIs in `src/lib.rs` are implemented with deterministic typed behavior and strict contract validation.
- bead-cupid unit tests and existing regression suites pass under moon gates.
- orchestrator workflow behavior and policy surfaces remain unchanged.
- docs and implementation constraints are aligned with no contract drift.
