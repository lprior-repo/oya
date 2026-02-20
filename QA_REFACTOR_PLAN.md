# QA Refactor Plan - OYA Orchestrator

**Generated:** 2026-02-20
**Status:** `BLOCK MERGE` - unresolved P0 security + P1 safety refactors

## Current State (Verified)

- `src/main.rs:107-154` already resolves models via usage tracker (`resolve_model_for_stage` + `get_active_model`) and reports outcomes (`report_stage_outcome`) for success/failure.
- `tests/contract_verify.rs:77-95` already parses opencode JSONL line-by-line.
- `Cargo.toml` still has no explicit `time` dependency; the lockfile still resolves `time` at `0.3.36`.
- `src/orchestrator_types.rs`, `src/pipeline/state.rs`, and `src/stage_executor.rs` still use tuple/string-heavy failure state.
- `src/pipeline/executor.rs:40` and `src/stage_executor.rs:43` still pass `Option<(FailureCategory, String)>`.
- `src/types/domain.rs` still keeps nullable `Option` fields in `AgentState` that allow invalid combinations.
- `src/quality_gate/mod.rs` still uses contradictory bools in `QualityGateResult`.
- `src/runtime_tools/command_exec.rs`, `src/runtime_tools/gates.rs`, and `src/workflow_runner.rs` still take `&PathBuf` in public/internal APIs.

## Execution Plan (Implementation First)

Each milestone includes owner, exact change location, and exit checks. Keep changes scoped strictly to the listed files.

### P0-1: Runtime Routing Safety (done)

- [x] Status: DONE (no further action required)
- Owner: core-runtime
- Files: `src/main.rs:115-154`
- Scope:
  - Keep `resolve_model_for_stage` usage in `build_start_context` and each stage transition.
  - Keep outcome reporting around every completed/failed stage via `report_stage_outcome`.
- Exit check:
  - `moon run :test`
  - smoke assertion in `src/main/tests.rs` covers `TestsUnexpectedlyGreen` mapping still present.

### P0-2: Security Baseline (must pass before merge)

- [ ] Status: PENDING
- Owner: platform
- Files:
  - `Cargo.toml` (add/override `time` dependency to `">=0.3.47"`)
  - `Cargo.lock` (updated by `moon run :ci` dependency resolution)
- Scope:
  - Raise transitive `time` to satisfy `RUSTSEC-2026-0009`.
  - Keep `tokio-tar` as accepted-risk dev dependency; record rationale in this file or ADR.
- Exit check:
  - `moon run :security`
  - confirm output no longer reports `RUSTSEC-2026-0009` with fixed `time`

### P1-1: Add failure-category behavior tests

- [ ] Status: PENDING
- Owner: test-coverage
- Files:
  - `src/main/tests.rs`
  - `src/orchestrator.rs` (if orchestration tests need helper fixtures)
- Scope (exact line targets):
  - Add test near existing gate-mapping cases in `src/main/tests.rs:97-165`.
  - New tests:
    - `tests_unexpectedly_green_maps_to_retry_loop` validates `TestsUnexpectedlyGreen` from `AcceptanceTest` routes to retry path.
    - `test_infra_failed_is_non_retryable` validates `TestInfraFailed` maps to a terminal/non-retry path in stage outcome logic.
- Exit check:
  - `moon run :test` with both new assertions green.
  - Ensure no behavior change in existing tests in `src/main/tests.rs`.

### P1-2: Replace tuple failure modeling

- [ ] Status: PENDING
- Owner: domain-core
- Files and targets:
  - `src/pipeline/state.rs:70` (`PipelineState.last_failure`)
  - `src/stage_executor.rs:43` (`StageExecutionRequest.last_failure`)
  - `src/stage_executor.rs:226-240` (`stage_failure_context` signature)
  - `src/main.rs:302-360` (`state.last_failure` read/write points)
  - `src/orchestrator.rs:39` (orchestrator request shape, if aligned)
- Scope:
  - Introduce shared `StageFailure` value type (new file/module or existing domain module).
  - Replace all `Option<(FailureCategory, String)>` with `Option<StageFailure>`.
  - Carry retry intent + timestamp metadata in the typed value instead of ad-hoc string semantics.
- Exit check:
  - `moon run :test`
  - `moon run :clippy` (no `clippy::from_iter_instead_of_collect` regressions etc.)

### P1-3: DDD refactor for orchestrated state (string -> typed domain)

- [ ] Status: PENDING
- Owner: domain-core
- Files and targets:
  - `src/orchestrator_types.rs:66-77` (`OrchestratorState` fields)
  - `src/orchestrator_types.rs:67-73` (stage/status/model/updated_at fields)
  - `src/main.rs:718-739` (state transitions where status strings are assigned)
  - `src/main.rs:311-318` (state copy/persist points)
- Scope:
  - Introduce/consume typed `OrchestratorStatus` enum (`Running`, `Shipped`, `Failed`).
  - Replace `bead_id/model/updated_at` strings with domain/newtype types and parse-safe value object.
  - Ensure serialization/deserialization for persisted state remains unchanged externally (stable wire format or migration-safe conversion).
- Exit check:
  - `moon run :test`
  - persisted-state roundtrip still valid in `persist_stage_artifact`/`write_orchestrator_state` calls.

### P1-4: DDD refactor for `AgentState`

- [ ] Status: PENDING
- Owner: domain-core
- File/target: `src/types/domain.rs:82-160`
- Scope:
  - Replace `Option`-combined `AgentState` with a closed type-state enum (`AgentStatusData` style) that prevents illegal transitions.
  - Keep `validate_invariants` as exhaustive constructors and transitions only.
- Exit check:
  - existing `src/orchestrator/tests.rs` and `src/types` tests remain green.
  - add focused validation tests for each invalid constructor path.

### P1-5: Make quality gate result type-safe

- [ ] Status: PENDING
- Owner: quality
- File/targets:
  - `src/quality_gate/mod.rs:32-42` (`QualityGateResult` fields)
  - `src/quality_gate/mod.rs:95-164` (`run()` constructors)
- Scope:
  - Replace `spec_passed/spec_score/scenarios_*/overall_passed/failure_category/message` with sum type (`Passed`, `SpecFailed`, `ScenariosFailed` etc.).
  - Remove impossible combinations (`overall_passed=true` with failures).
- Exit check:
  - existing gate tests in `src/quality_gate/mod.rs` updated or expanded to cover all variants.

### P1-6: DDD types for pipeline input

- [ ] Status: PENDING
- Owner: domain-core
- File/targets:
  - `src/pipeline/state.rs:61-78` (`PipelineRunInput` and constructor)
  - callsites: `src/main.rs:259-261`, `src/main.rs:296-304`
- Scope:
  - Convert `run_id`, `bead_id`, `context` from raw `String` into domain wrappers (`RunId`, `BeadId`, `Context` or existing equivalents).
  - Keep deterministic constructors and serialization compatibility for runtime state.
- Exit check:
  - `moon run :test`
  - compile boundary tests around pipeline construction still pass.

### P1-7: Path API normalization (`&PathBuf` -> `&Path`)

- [ ] Status: PENDING
- Owner: runtime-tools
- File/targets:
  - `src/runtime_tools/command_exec.rs:14,39,65,83,193`
  - `src/runtime_tools/gates.rs:17`
  - `src/workflow_runner.rs:28`
- Scope:
  - Change function signatures to `&Path` for read-only filesystem inputs.
  - Keep cloning only at call sites where owned ownership is required (e.g., process spawning).
- Exit check:
  - `moon run :test`
  - no `std::path::PathBuf` references added in parameters above.

### P1-8: Coverage hardening for uncovered modules

- [ ] Status: PENDING
- Owner: test-coverage
- Targets:
  - `src/runtime_tools/command_exec.rs` (current low coverage hotspot)
  - `src/stage_executor.rs` (low coverage hotspot)
  - `src/types/pipeline.rs` and `src/usage.rs`
- Scope:
  - Add minimal deterministic unit tests for currently untested branches (timeouts, failure parsing, retry mapping, failure-context formatting).
  - Prefer small, pure test fixtures; avoid new integration network calls.
- Exit check:
  - `moon run :coverage`
  - move target module coverage toward the listed goals in this file's section.

## P1: Contract-First ATDD Plan (For These items)

Use this before changing code. All tests should be added in the order below:

1) Public API/contract tests first.
2) Negative/error-path tests.
3) Implementation guardrail tests (property/unit).

### 1) Public API / Contract scenarios

#### LLM routing correctness

- Given: start request has explicit model
  When: `build_start_context` executes
  Then: returned model equals request model and no usage tracker call is performed.
- Test name: `given_explicit_model_when_build_start_context_is_called_then_tracker_is_not_used`

- Given: start request omits model
  When: `build_start_context` executes
  Then: model comes from `resolve_model_for_stage(Stage::Plan)` via usage tracker and `get_active_model` is called once.
- Test name: `given_missing_model_when_build_start_context_runs_then_plan_tier_model_is_selected`

- Given: any non-plan stage is about to execute
  When: model is resolved
  Then: `tier_for_stage` is called and returned tier mapping must match `StageName::model_for_stage`.
- Test name: `given_pipeline_stage_when_resolve_model_for_stage_called_then_tier_is_from_stage_model_policy`

- Given: stage outcome is rate limited
  When: `report_stage_outcome` is invoked
  Then: `is_rate_limit_failure` is true and `report_outcome` payload marks `is_rate_limit` true.
- Test name: `given_rate_limit_failure_when_report_stage_outcome_runs_then_rate_limit_flag_is_true`

#### `OrchestratorState` as domain types

- Given: persisted state uses invalid status string
  When: state is loaded
  Then: state load fails with a contract error, not silent coercion.
- Test name: `given_invalid_status_string_when_loading_orchestrator_state_then_contract_load_fails`

- Given: successful and failed terminal statuses
  When: run transitions to completion/failure
  Then: `OrchestratorState::status` is always one of `Running | Shipped | Failed`, and `updated_at` is RFC3339 datetime.
- Test name: `given_terminal_stage_completion_when_orchestrator_updates_state_then_status_is_typed_and_timestamp_is_valid`

- Given: state is updated by stage artifacts
  When: persisted artifact is accepted
  Then: `bead_id` is `BeadId`, `model` is validated `ModelName`, and `last_failure` is `Option<StageFailure>`.
- Test name: `given_stage_artifact_when_state_is_updated_then_orchestrator_fields_are_typed_and_non_ambiguous`

#### `AgentState` type-state validity

- Given: public API receives an invalid state combination
  When: constructing or updating an agent
  Then: no invalid combination is representable; tests only need successful compile for valid states.
- Test name: `given_invalid_agent_payload_when_constructing_agent_state_then_type_state_prevents_invalid_combo`

- Given: transition `Idle -> Working`
  When required fields become present
  Then: working variant must include bead, stage, and start time.
- Test name: `given_idle_to_working_transition_when_fields_are_complete_then_constructs_working_variant`

- Given: transition to `Done`
  When: work completes
  Then: done variant has no active stage/bead metadata.
- Test name: `given_working_to_done_when_transition_occurs_then_inactive_agent_has_no_stage_payload`

#### `QualityGateResult` algebra

- Given: spec check fails
  When `QualityGate::run` evaluates phase one
  Then: result is `QualityGateResult::SpecFailed` and `overall_passed` cannot be true.
- Test name: `given_spec_fails_when_quality_gate_runs_then_spec_failed_variant_is_returned`

- Given: spec passes and scenario checks fail
  When phase two evaluates
  Then: result is `QualityGateResult::ScenariosFailed` with category and message.
- Test name: `given_scenarios_fail_when_gate_runs_then_scenarios_failed_variant_is_returned`

- Given: both checks pass
  When finalizing gate
  Then: result is `QualityGateResult::Passed` and retry flag is false.
- Test name: `given_quality_gate_checks_pass_when_gate_runs_then_passed_variant_is_returned`

#### `PipelineRunInput` types

- Given: `run_id` or `bead_id` is empty
  When constructing input
  Then: constructor fails with domain error (invalid input).
- Test name: `given_empty_ids_when_pipeline_input_constructed_then_input_factory_returns_error`

- Given: valid IDs and context
  When `pipeline_input` is called
  Then result is typed and preserves `context` text exactly.
- Test name: `given_valid_strings_when_pipeline_input_constructed_then_typed_input_roundtrips_context`

#### `StageFailure` semantics

- Given: `FailureCategory::TestsUnexpectedlyGreen`
  When mapped to `StageFailure`
  Then: `retryable == true` and failed-at timestamp is recorded.
- Test name: `given_tests_unexpectedly_green_when_stage_failure_created_then_retryable_is_true`

- Given: `FailureCategory::TestInfraFailed`
  When mapped to `StageFailure`
  Then: `retryable == false` and reason text is preserved.
- Test name: `given_test_infra_failed_when_stage_failure_created_then_retryable_is_false`

- Given: legacy tuple failure is transformed
  When converting to `StageFailure`
  Then: no message data is dropped and `failed_at` is set at conversion time.
- Test name: `given_legacy_failure_tuple_when_upgrading_to_stage_failure_then_message_is_preserved`

#### Path signatures (`&Path` not `&PathBuf`)

- Given: a command execution request path originates from workspace config
  When passed through command, workspace, gate, and stage entry points
  Then every helper accepts `&Path` without clone-only to retain ownership at call sites.
- Test name: `given_workspace_path_borrowed_when_passing_between_layers_then_no_clone_required`

### 2) Implementation guardrails

- Add error contracts first: every public fallible helper involved in these P1 items must have `Result<_, DomainError>` shape in tests or explicit equivalent domain error propagation.
- Add non-retryable/retryable edge tests for each failure category branch used in retry policy.
- Add transition table tests for `StageName`, `OrchestratorStatus`, `QualityGateResult`, and `StageFailure` to ensure exhaustive behavior.
- Add deterministic property tests for serialization round-trips for `OrchestratorState`, `PipelineRunInput`, `AgentState` variants, and `StageFailure`.

### 3) `command_exec` and `stage_executor` coverage commitments

- Command execution layer (`src/runtime_tools/command_exec.rs`):
  - timeout command path used when available.
  - spawn fallback path used when timeout binary missing.
  - command-not-found and spawn errors are converted to `OyaError` with useful detail.
  - command timed out returns exit code `124` and timeout message.
  - opencode CLI missing output triggers HTTP fallback.
  - tests should target at least 50% module coverage and be deterministic.

- Stage executor (`src/stage_executor.rs`):
  - `validate_attempt(0)` rejection.
  - deterministic replay preserves cached result (journal contract).
  - command/gate failure mapping into failure categories and next-stage routing.
  - stage failure context injected for retries only when present.
  - tests should raise coverage to at least 50% and include invalid-path negative tests.

Suggested command order:

1. Add contract tests in `tests/` first.
2. Run `moon run :test` and confirm fail-fast on red tests.
3. Implement only what is needed for green.
4. Run `moon run :coverage` and keep `command_exec.rs` + `stage_executor.rs` above target.

## Verification Checklist (Per Milestone + Final)

- For each milestone:
  - `moon run :test`
  - `moon run :clippy`
  - `moon run :security`
- Final merge gate:
  - `moon run :ci`

## Target Status Table

| Item | Status | Why |
|------|--------|-----|
| LLM routing via usage tracker | DONE | Already wired at `src/main.rs:107-154` and stage transitions use it |
| JSONL contract verify | DONE | `tests/contract_verify.rs:77-95` parses line-by-line |
| Security `time` vuln | PENDING | direct/time-pinning not yet present |
| Tuple-based failure state | PENDING | still used in pipeline/stage state |
| Orchestrator state typing | PENDING | still string-heavy in `src/orchestrator_types.rs` |
| Agent invalid state combos | PENDING | `AgentState` uses nullable fields in `src/types/domain.rs` |
| Quality gate inconsistency | PENDING | boolean/missing-field issue remains in `src/quality_gate/mod.rs` |
| Path API normalization | PENDING | remaining `&PathBuf` params in tools/runner |
| Coverage uplift | PENDING | current module coverage remains below targets |

---

## Scope Note

- No new global architecture changes.
- No `cargo` invocations in this repo; all quality gates use `moon` tasks.
