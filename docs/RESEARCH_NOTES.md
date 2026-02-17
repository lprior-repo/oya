# bead-cupid

## Implementation constraints
- Keep crate safety attributes unchanged in `src/lib.rs` and `src/main.rs`: deny `unwrap_used`, `expect_used`, `panic`; forbid `unsafe_code`.
- Preserve the canonical stage sequence and transition model in `src/types.rs`: `research -> plan -> contract -> tdd15 -> qa -> red_queen -> gpt_review -> ship_gate`, with `max_attempts = 3`.
- Keep stage gate contracts stable (`StageName::gates` + runtime gate execution): `moon run :check`, `moon run :test`, `moon run :quick`, `moon run :ci`, `zjj done --dry-run`.
- Keep failure/retry behavior stable in orchestrator loop (`src/main.rs`): retry only `TestFailed`, `LintFailed`, `OutputParseFailure`; block on non-retryable failures or max attempts.
- Preserve orchestrator state/event schema and key conventions (`state`, `run_request`, `timeline`, `event_seq`, `event_####`, and per-stage `{stage}_{attempt}_{input|result|skill_output|gate_*|event}`).
- Preserve execution timeouts and shell contract in `src/main.rs`: OpenCode 300s, moon 900s, zjj 60s; commands execute via `timeout` from repo root.
- For functional-core additions in `src/lib.rs`, follow existing pattern: normalize input (`trim`), enforce length/charset/url constraints, return typed `Result` errors (no panic path), derive decisions only from observed checks, enforce fixed stage order, non-empty diagnostics, and monotonic timestamps.
- Keep runtime endpoint defaults and validation contracts unchanged for smoke-style flows: `scripts/dev-up.sh`, `http://localhost:8080/restate/health`, `http://localhost:8080/OyaOrchestrator/{run_id}/get_status`.
