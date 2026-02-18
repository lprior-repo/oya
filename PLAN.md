# OYA Implementation Plan: src-kes

Request context: observability test run
Attempt: 1

## Objective

Strengthen observability test coverage for the existing Rust `src-kes` contract and orchestrator run path so stage execution, workspace lifecycle, and ops-monitor telemetry remain deterministic and regression-resistant.

## Codebase Alignment Snapshot

- `src-kes` already exists in `src/lib.rs`; extend current contracts (`build_src_kes_plan`, CRUD functions, `validate_src_kes_report`) instead of introducing a parallel implementation.
- Orchestrator stage flow and quality-gate execution live in `src/main.rs`; preserve stage order, retry semantics, and moon/zjj command boundaries.
- Pipeline domain behavior is already tested across `tests/pipeline_logic.rs`, `tests/state_machine.rs`, `tests/gates.rs`, and `tests/behavior.rs`; add observability-focused assertions where gaps exist.
- Ops-monitor parsing and normalization logic is already in `src/lib.rs` (`build_zjj_workspace_name`, `parse_opencode_*`, `build_opencode_poll_snapshot`); coverage should expand without changing public contracts unnecessarily.

## Exact Implementation Steps

1. Baseline existing observability surfaces and lock invariants in tests first.
   - Identify current guarantees for timeline events, stage transitions, gate command mapping, and workspace lifecycle (`zjj queue --add` before `zjj add`).
   - Convert those guarantees into explicit failing tests before implementation changes.

2. Expand `src-kes` report observability contract tests in `src/lib.rs`.
   - Add tests for strict stage ordering: `PlanBuild -> RuntimeStart -> RouteContract -> CrudContract -> FinalDecision`.
   - Add tests for diagnostics non-emptiness and monotonic timestamps across every stage.
   - Add tests for decision derivation mismatches and partial-stage failure observability.

3. Add orchestrator observability behavior tests for stage execution and failure context.
   - Extend stage-level tests to assert retryable failure categories remain exactly `TestFailed`, `LintFailed`, and `OutputParseFailure`.
   - Add assertions that stage prompt/failure context rendering includes actionable output summaries for timeout and parse failures.
   - Verify ShipGate behavior remains `moon :ci` then `zjj done --dry-run` then `moon :quick` and `moon :test`.

4. Add workspace lifecycle observability tests around queue/add sequencing.
   - Add/extend tests to assert workspace preparation emits queue and add events in order and captures command/exit metadata.
   - Add negative-path tests for queue failure and add failure to ensure emitted diagnostics are truncated, deterministic, and actionable.
   - Verify skip flags (`OYA_SKIP_ZJJ_WORKSPACE`, `OYA_SKIP_ZJJ_GATE`) preserve expected event/report behavior.

5. Expand ops-monitor parser test matrix for run-time telemetry inputs.
   - Add SSE parsing edge cases (multi-event chunks, payload length boundaries, control-char rejection, max-event truncation).
   - Add poll snapshot aggregation tests with mixed busy/idle sessions and mixed permission/question JSON envelope shapes.
   - Keep parser outputs stable and sorted/deterministic for repeated identical inputs.

6. Keep boundaries strict while implementing.
   - Do not alter route/status contract for `src-kes` CRUD endpoints.
   - Do not weaken lint safety attributes or introduce unwrap/expect/panic.
   - Do not change production stage sequencing or gate command intent beyond what tests require.

## Test Strategy

- Unit tests (`src/lib.rs`):
  - `src-kes` report invariants: stage order, timestamp monotonicity, diagnostics presence, and decision consistency.
  - ops-monitor parsing/normalization: workspace name, busy session extraction, pending counts, SSE payload parsing.

- Pipeline behavior tests (`tests/`):
  - stage transition + retry policy correctness for observability-relevant failures.
  - gate mapping and execution ordering assertions, including ShipGate command sequence expectations.
  - workspace lifecycle observability assertions for queue/add and dry-run merge checks.

- Regression tests:
  - preserve existing `src-kes` deterministic CRUD flow tests.
  - preserve existing orchestrator happy-path/failure-path state-machine behavior.

- Optional stress verification (if needed during implementation):
  - repeat selected parser tests with larger payload fixtures to validate deterministic truncation and bounded outputs.

## Quality Gates

- Mandatory gates (moon-only invocation):
  - `moon run :check`
  - `moon run :test`
  - `moon run :quick`
  - `moon run :ci`

- Definition of done:
  - observability-focused tests pass and prove deterministic behavior for report validation, parser outputs, and stage/gate sequencing.
  - no regression in existing `src-kes` contract behavior or orchestrator stage transitions.
  - no new lint violations, safety invariant violations, or workflow contract drift.

- Release blockers:
  - any nondeterministic timeline/report/parser behavior under identical inputs.
  - any change that weakens retry policy boundaries or workspace lifecycle ordering.
  - any failure in mandatory moon quality gates.

## Acceptance Criteria

- `PLAN.md` reflects the current Rust-based `src-kes` implementation and observability constraints.
- Test coverage explicitly validates observability contracts for `src-kes`, orchestrator stages, and ops-monitor parsing.
- Mandatory moon gates complete successfully with no contract regressions.
