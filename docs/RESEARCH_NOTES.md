# src-1ew

## Implementation constraints
- Follow the established typed workflow pattern in `src/lib.rs`: define `Input -> Plan -> RuntimeHandle -> Observation -> Report` plus explicit `CheckName`, `StageName`, `StageStatus`, `Decision`, and `Error` enums.
- Keep public function shape aligned with existing flows (`build_*_plan`, `start_*_runtime`, `capture_*_observation`, `evaluate_*_result`, `validate_*_report`) and validate before each transition.
- Preserve crate safety/lint invariants: `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`, `#![forbid(unsafe_code)]`.
- Reuse strict field hygiene: trim input, reject empty values, enforce max-length constants, reject forbidden control chars, and keep identifiers ASCII-safe (alnum plus `-`/`_`).
- Keep runtime contract deterministic: runtime command fixed to `scripts/dev-up.sh`, ingress endpoint fixed to `http://localhost:8080/restate/health`, orchestrator endpoint formatted as `http://localhost:8080/OyaOrchestrator/{run_id}/get_status`.
- Validate endpoints with the same URL rules used elsewhere: only `http`/`https`, host required, no embedded credentials.
- Require one ingress check and one orchestrator check (no duplicates/missing checks), with diagnostics present and bounded.
- Enforce report invariants: exact stage order `IngressHealth -> OrchestratorStatus -> FinalDecision`, monotonic timestamps, diagnostics non-empty/size-bounded/control-char-free, and stage statuses consistent with check outcomes.
- Derive final decision from check success values and validate final-stage status/diagnostics against that derived decision.
- If any orchestrator wiring is touched in `src/main.rs`, preserve stage order/retry semantics, moon-only gates, and zjj ordering (`queue --add` before `add`).
