# OYA Implementation Plan: auth-test

Request context: testing
Attempt: 1

## Objective

Add deterministic authentication-focused test coverage for Oya orchestration so auth failure handling, OpenCode basic-auth behavior, and auth-related pipeline contracts are validated without relying on external services in default test runs.

## Codebase Alignment Snapshot

- Retry policy logic is defined by `is_retryable_failure` in `src/lib.rs`; `FailureCategory::AuthFailed` is currently non-retryable.
- Failure category and stage/gate contracts live in `src/types.rs` (`FailureCategory`, `StageName`, `Gate`).
- Existing auth behavior coverage is partial in `tests/pipeline_logic.rs` and `tests/behavior.rs` (non-retryable auth-failure assertions).
- OpenCode auth wiring is implemented in `src/main.rs` (`opencode_config`, `fetch_opencode_text`, `fetch_text_with_client`) via optional `OYA_OPENCODE_PASSWORD` basic auth.
- Integration coverage in `tests/integration.rs` currently focuses on JSON/SSE parsing and poll snapshots, with no explicit auth status contract tests.
- Contract/docs files (`tests/contract_verify.rs`, `docs/TEST_SUITE.md`, `docs/BEHAVIOR_TESTS.md`) still include cargo-based invocation text that is not moon-only aligned.

## Exact Implementation Steps

1. Update crate-level design contract for this bead in `src/lib.rs`.
   - Replace the top-level contract block with `auth-test` scope.
   - Define acceptance bullets for auth classification determinism, credential handling, status-to-decision mapping, and no-panic parsing behavior.
   - Keep constraints explicit: deterministic outputs for equivalent inputs, monotonic report semantics where applicable, and no panic-style APIs.

2. Add an auth-focused pure contract surface in `src/lib.rs` following existing contract patterns.
   - Introduce minimal auth-test domain types/functions for deterministic auth check planning/evaluation (input, check, report, decision, error).
   - Reuse existing validation patterns already used across this file (trim, non-empty checks, max lengths, invalid control-character rejection).
   - Add explicit mapping rules for auth-related failure signals (invalid credentials, unauthorized/forbidden responses, missing credential input).

3. Refactor auth decision points in `src/main.rs` behind testable pure helpers.
   - Extract status and error-string classification logic into pure functions (or `src/lib.rs` helpers) used by `fetch_opencode_text` and `fetch_text_with_client` paths.
   - Ensure 401/403 and credential-missing paths map deterministically to auth-failure semantics used by stage execution.
   - Preserve existing runtime behavior and env defaults (`OYA_OPENCODE_BASE_URL`, optional `OYA_OPENCODE_PASSWORD`) while making auth expectations explicit.

4. Expand auth-first unit tests in `src/lib.rs` (RED-GREEN-REFACTOR).
   - Add failing tests first for empty/oversized/invalid auth inputs, 401/403 classification, malformed auth diagnostics, and decision mismatch detection.
   - Add determinism checks asserting equivalent auth inputs produce equivalent reports/decisions.
   - Keep test implementations pure and local (no network or external process dependency).

5. Extend behavioral and pipeline tests for auth outcomes.
   - In `tests/pipeline_logic.rs`, add table-style auth cases confirming retry policy boundaries between `AuthFailed`, retryable categories, and terminal decisions.
   - In `tests/behavior.rs`, add Given-When-Then scenarios for auth failure at early and mid-pipeline stages, verifying no retry loops and consistent failure propagation.
   - Ensure assertions are explicit on next-stage behavior and attempt counts.

6. Add auth integration and contract coverage.
   - In `tests/integration.rs`, add hermetic auth-response scenarios (401/403 payload shapes, unauthorized body truncation handling, auth-required SSE/poll endpoints).
   - In `tests/contract_verify.rs`, add contract checks that auth failure remains non-retryable and stage/gate contracts are unchanged by auth-test additions.
   - Replace panic-style test helpers in touched auth tests (`unwrap`/`expect`) with assertion-based handling to stay clippy-clean.

7. Align test documentation and command guidance.
   - Update `docs/TEST_SUITE.md` and `docs/BEHAVIOR_TESTS.md` auth-test coverage sections and expected execution layers.
   - Replace cargo invocations in touched docs/comments with moon-only equivalents.
   - Document which auth tests run in default `:test` vs ignored/manual contexts.

8. Run moon quality gates and resolve regressions in touched files.
   - Execute mandatory gates listed below in order and fix failures in code/tests (not lint configuration).
   - Preserve deny/forbid policy expectations (`unwrap`, `expect`, `panic`, `unsafe`) for production paths.

## Test Strategy

- Contract-first unit tests:
  - Drive auth contract API in `src/lib.rs` via RED-GREEN-REFACTOR for validation, mapping, report ordering, and decision invariants.
- Pipeline behavior tests:
  - Expand `tests/pipeline_logic.rs` and `tests/behavior.rs` to verify non-retryable auth behavior across attempts and stages.
- Integration boundary tests:
  - Extend `tests/integration.rs` with mocked unauthorized/forbidden responses and auth-protected endpoint payload edge cases.
- Regression confidence:
  - Keep `tests/state_machine.rs`, `tests/gates.rs`, and `tests/properties.rs` green to prevent drift in core orchestration contracts.

## Quality Gates

- Mandatory moon-only gates:
  - `moon run :check`
  - `moon run :test`
  - `moon run :quick`
  - `moon run :ci`

- Additional confidence gates:
  - `moon run :coverage`
  - `moon run :mutants-quick`

- Release blockers:
  - Any failure in mandatory moon gates.
  - Any direct cargo command introduced in touched docs/comments/workflow examples.
  - Any panic-style additions (`unwrap`, `expect`, `panic`, `unsafe`) in production auth paths.
  - Any auth test path in default `:test` that requires external OpenCode/ReState/OpenObserve availability.

## Acceptance Criteria

- `PLAN.md` is scoped to `auth-test` with request context `testing` and attempt `1`.
- Plan lists exact file-level implementation steps for auth contract logic, runtime wiring alignment, tests, and docs.
- Plan includes explicit TDD-oriented test strategy plus moon-only quality gates and release blockers.
