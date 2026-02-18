# OYA Implementation Plan: src-1ew

Request context: implement Pokemon CLI tool with REST API client
Attempt: 1

## Objective

Implement a Rust Pokemon CLI in this existing `oya` binary with deterministic contracts, typed error handling, and test-first coverage for `get-pokemon`, `list-pokemon`, and `search` commands backed by PokeAPI via `reqwest`.

## Codebase Alignment Snapshot

- Keep implementation inside existing crate boundaries: domain and validation logic in `src/lib.rs`, CLI wiring in `src/main.rs`, and integration-level checks in `tests/`.
- Follow established typed workflow pattern already present in `src/lib.rs`: `Input -> Plan -> RuntimeHandle -> Observation -> Report` plus explicit `CheckName`, `StageName`, `StageStatus`, `Decision`, and `Error` enums.
- Preserve safety constraints already enforced across crate: no `unwrap`, no `expect`, no `panic`, no `unsafe`.
- Reuse existing URL validation posture (http/https only, host required, no embedded credentials) used in current endpoint helpers.
- Keep moon-only quality gate workflow (`moon run ...`) and avoid direct `cargo` execution during verification.

## Exact Implementation Steps

1. Add failing contract tests first in `src/lib.rs` for new `src-1ew` domain types and flow.
   - Add red tests for normalized input validation (`query`, `limit`, `offset`, command mode), max-length enforcement, and control-char rejection.
   - Add red tests for exact stage order, monotonic timestamps, non-empty diagnostics, and decision derivation consistency.
   - Add red tests for endpoint contract validation: only PokeAPI base URL allowed by default, valid http/https URL shape, no credentials.

2. Introduce `src-1ew` typed contracts in `src/lib.rs`.
   - Add constants for bounds/defaults (query length, list page size, diagnostics size, request timeout).
   - Add enums/structs following existing style: `Src1ewInput`, `Src1ewPlan`, `Src1ewRuntimeHandle`, `Src1ewCheckName`, `Src1ewObservation`, `Src1ewStageName`, `Src1ewStageStatus`, `Src1ewDecision`, `Src1ewReport`, `Src1ewError`.
   - Add pure validators/normalizers for command arguments and query text.

3. Implement deterministic contract functions in `src/lib.rs`.
   - `build_src_1ew_plan(&Src1ewInput) -> Result<Src1ewPlan, Src1ewError>`
   - `start_src_1ew_runtime(&Src1ewPlan) -> Result<Src1ewRuntimeHandle, Src1ewError>`
   - `capture_src_1ew_observation(&Src1ewRuntimeHandle) -> Result<Src1ewObservation, Src1ewError>`
   - `evaluate_src_1ew_result(&Src1ewObservation) -> Result<Src1ewReport, Src1ewError>`
   - `validate_src_1ew_report(&Src1ewReport) -> Result<(), Src1ewError>`

4. Add HTTP client adapter and response mapping in `src/lib.rs` behind typed boundaries.
   - Add request builders for `/pokemon/{name_or_id}` and `/pokemon?limit={limit}&offset={offset}`.
   - Add deterministic DTOs for required PokeAPI fields only (minimal serde structs).
   - Add `search` behavior as explicit contract: normalize query, fetch bounded list pages, filter by case-insensitive name contains, preserve stable output order.
   - Classify failures into typed variants (transport, non-2xx status, decode failure, empty result, invalid input).

5. Add CLI surface in `src/main.rs` without breaking existing modes.
   - Extend `CliCommand` with `Pokemon` command and nested subcommands: `GetPokemon`, `ListPokemon`, `Search`.
   - Keep existing `Serve` and `OpsPoll` behavior unchanged as defaults.
   - Add output formatter that prints deterministic, parseable text rows for each command.

6. Add command execution wiring in `src/main.rs`.
   - Route parsed CLI args into `src-1ew` flow builder/evaluator functions.
   - Ensure exit behavior is typed and consistent: success returns `Ok(())`, failures return clear `OyaError` messages without panics.
   - Apply timeout and base URL configuration through validated environment-backed options where needed.

7. Add unit tests for adapter and parser logic in `src/lib.rs`.
   - Validate JSON decoding for get/list/search fixtures.
   - Validate bounded diagnostics and stable ordering for repeated identical inputs.
   - Validate that duplicate/malformed entries do not violate report invariants.

8. Add CLI and integration tests in `tests/integration.rs` (or dedicated `tests/pokemon_cli.rs`).
   - Use `wiremock` to simulate PokeAPI success/failure payloads and status codes.
   - Verify command parsing and output for `get-pokemon`, `list-pokemon`, and `search`.
   - Verify error-path behavior for timeout, 404, invalid JSON, and empty search results.

9. Refactor for clarity without changing contracts.
   - Keep functional core/imperative shell separation: pure normalization and report validation isolated from network side effects.
   - Deduplicate shared validation helpers where it reduces repetition and preserves existing behavior.

10. Run required moon gates and finalize artifacts.
    - Execute mandatory checks in order defined below.
    - Keep plan and tests synchronized with any implementation adjustments before completion.

## Test Strategy

- Unit tests (`src/lib.rs`):
  - Contract invariants for full `src-1ew` flow (stage order, diagnostics bounds, monotonic time, decision derivation).
  - Input normalization/validation for command parameters (`name_or_id`, `limit`, `offset`, `query`).
  - URL and endpoint contract validation for PokeAPI client configuration.

- Adapter tests (`src/lib.rs`):
  - Serde decode tests for minimal Pokemon payloads.
  - Error mapping tests for transport/status/deserialize failures.
  - Deterministic search filtering and ordering tests.

- Integration tests (`tests/`):
  - `wiremock`-backed end-to-end command flows for get/list/search.
  - CLI output contract tests for both success and failure cases.
  - Regression checks that existing `serve` and `ops-poll` commands still parse and route correctly.

- Regression guardrails:
  - No behavior drift in existing orchestrator stage flow and gate-related tests.
  - No lint/safety regressions against crate-level deny/forbid rules.

## Quality Gates

- Mandatory moon-only gates:
  - `moon run :check`
  - `moon run :test`
  - `moon run :quick`
  - `moon run :ci`

- Additional quality checks before done:
  - New tests cover command contracts and error typing for all three Pokemon commands.
  - Existing tests remain green with no orchestrator behavior regressions.
  - Public-facing CLI text output is deterministic for identical inputs.

- Release blockers:
  - Any direct `cargo` invocation for verification instead of `moon run` tasks.
  - Any panic/unwrap/expect usage introduced in new code paths.
  - Any nondeterministic output/order in report or CLI output for identical fixtures.
  - Any failing mandatory moon quality gate.

## Acceptance Criteria

- `PLAN.md` contains actionable, file-specific implementation steps for `src-1ew`.
- Plan includes explicit test-first strategy for unit, adapter, and integration layers.
- Plan includes moon-only quality gates and clear release blockers aligned to current repository rules.
