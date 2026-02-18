# src-kes

## Implementation constraints
- `src-kes` already exists as a Rust typed contract in `src/lib.rs`; build on existing `build_src_kes_plan`, `start_src_kes_server`, `register_user_routes`, CRUD functions, and `validate_src_kes_report` instead of introducing a parallel model.
- Preserve hard lint/safety invariants at crate boundaries: `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`, `#![forbid(unsafe_code)]`.
- Keep route contract exact and deterministic: `POST /users -> 201`, `GET /users/:id -> 200`, `PUT /users/:id -> 200`, `DELETE /users/:id -> 204`; contract comparison is set-based and strict.
- Maintain strict input normalization/validation behavior: trimmed non-empty fields, max-length enforcement, control-char rejection, lowercase email normalization, explicit email format checks, and derived user IDs constrained to ASCII alnum/hyphen.
- Preserve explicit error taxonomy and semantics in `SrcKesError` (`EmptyField`, `FieldTooLong`, `InvalidFieldContent`, `InvalidFieldFormat`, `InvalidRouteContract`, `DuplicateUserId`, `UserNotFound`, `InvalidReport`).
- Keep service logic pure/typed over `SrcKesServiceState` (no hidden side effects): create/update/delete return new state snapshots and deterministic records.
- Preserve report validation invariants: required stage order (`PlanBuild`, `RuntimeStart`, `RouteContract`, `CrudContract`, `FinalDecision`), non-empty diagnostics, monotonic timestamps, and decision derived from stage statuses.
- Do not break orchestrator/runtime contracts used by observability runs in `src/main.rs`: stage pipeline order, retryability (`TestFailed`, `LintFailed`, `OutputParseFailure` only), workspace lifecycle (`zjj queue` before `zjj add`), and moon-only quality gates.
