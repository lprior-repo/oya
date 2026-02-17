# manual-e2e-bead: string payload accepted

## Start payload contract (`src/main.rs`)
- `start` accepts `Json<serde_json::Value>` and parser supports only two shapes: JSON object or JSON string containing a JSON object.
- Object path uses `serde_json::from_value`; string path uses `serde_json::from_str` into `StartRequestPayload`.
- Payload model is `StartRequestPayload { bead_id: Option<String>, context: Option<String> }`.
- Non-object/non-string payloads (`number/array/bool/null`) fail with `expected object or JSON string`.

## Runtime behavior after parse
- `bead_id` defaults to `"unknown"` when missing; `context` defaults to `""` when missing.
- `run_id` is always `ctx.key()` and is returned immediately after `tokio::spawn`.
- `start` does not surface async pipeline failures in response; failures are logged from background execution.

## Idempotency and status constraints
- `start_or_resume_pipeline` sets `run.id = ctx.key()` and inserts via `insert_bead_run_if_absent`.
- Duplicate `run_id` is treated as success (`DuplicateRunKey` short-circuits with no-op).
- `get_status` requires persisted run; otherwise returns `Run not found`.

## Stage and gate constraints relevant to manual e2e
- Canonical stage sequence is fixed: `research -> plan -> contract -> tdd15 -> qa -> red_queen -> gpt_review -> ship_gate`.
- Per-stage max attempts are fixed at `3`.
- Retryability is fixed by `FailureCategory`: `OutputParseFailure` is terminal; `CompileFailed/TestFailed/LintFailed/MergeConflict` are retryable.
- Quality gates execute fixed commands: `moon run :check|:test|:quick|:ci` and `zjj done --dry-run`.

## Existing test coverage proving requirement
- `parse_start_request_accepts_json_string_payload` verifies acceptance of string body `{"bead_id":"manual-e2e-bead","context":"string payload"}`.
- Companion tests verify object-body acceptance and non-object/non-string rejection.
