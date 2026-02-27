# Error Handling: Lifecycle Runtime

## Core Rule

All fallible paths return typed errors (`Result<T, E>`). No `unwrap`, `expect`, or `panic` in source code.

## Error Model

`src/lifecycle/types/error.rs` models failures as:

- `LifecycleError::Terminal { category, message }`
- `LifecycleError::Transient { category, message }`

With categories:

- `Validation`
- `Workspace`
- `Bookmark`
- `PullRequest`
- `Command`

## Runtime Behavior

- Terminal failures stop lifecycle execution and trigger compensation logic where defined.
- Transient failures are surfaced with classification so callers can decide retry/backoff policy.
- Validation failures (invalid repo slug/model, bad DAG dependencies/cycles/order) fail before effect execution.
