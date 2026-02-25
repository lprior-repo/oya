# Research Notes: verified-test

## Implementation constraints
- Keep crate-level safety guards unchanged in `src/main.rs` and `src/lib.rs`: deny `unwrap`, `expect`, `panic`; forbid `unsafe`.
- Preserve stage graph and policies in `src/types.rs`: `research -> plan -> contract -> tdd15 -> qa -> red_queen -> gpt_review -> ship_gate`, `max_attempts = 3`, gate map from `StageName::gates()`.
- Preserve retry policy: only `TestFailed`, `LintFailed`, and `OutputParseFailure` are retryable (`is_retryable_failure`); all other failures terminate run.
- Keep gate execution moon-only in `src/main.rs`: `moon run :check|:test|:quick|:ci`; ship gate order is `:ci -> (optional jj dry-run) -> :quick -> :test`.
- Keep workspace provisioning order for workspace stages in `src/main.rs`: `jj workspace add <ws>` before sync/rebase flow; workspace names come from `build_jj_workspace_name` and remain <= 64 chars.
- Keep deterministic workflow side effects in Restate handlers: env/time reads wrapped via `ctx.run`, state/timeline persisted with stable keys (`state`, `timeline`, `event_seq`, `<stage>_<attempt>_*`).
- Preserve stage fallback behavior on failures: QA/RedQueen test failures route to `tdd15`; review/lint failures route to `gpt_review`; ship-gate merge conflicts route to `gpt_review`.
- Keep OpenCode monitoring/parsing contracts: bounded payload sizes, strict JSON parsing, control-char rejection, SSE `data:` extraction, and credential-free URL validation.
- Keep lib contract style across flows (`build_* -> start/run/capture_* -> evaluate_* -> validate_*`), with decision derived from checks (never manually injected).
- Keep validator invariants in `src/lib.rs`: exact check/stage cardinality/order, non-empty diagnostics, max-length limits, monotonic timestamps, and decision/status coherence.
- Preserve telemetry behavior in `src/telemetry/*`: shared tracing registry, JSON log layer + OTLP trace layer, shutdown flush guard, config validation for endpoint/sampler.

## Implementation-ready focus
- Extend existing typed contracts instead of adding ad-hoc paths; add fields/functions in the same plan/observation/report/validate shape.
- Put tests on invariants first (input normalization, endpoint contracts, stage order, timestamp monotonicity, derived decision correctness).
- For orchestrator changes, verify both pass progression and failure reroute behavior, plus stable state/timeline serialization keys used by polling.
