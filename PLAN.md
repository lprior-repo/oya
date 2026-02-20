# PLAN: Fix spawn_blocking Determinism in execute_stage_real

## Context
Refactor `execute_stage_real` to move `spawn_blocking` outside of `ctx.run()` to separate non-deterministic execution from deterministic journaling. This ensures that while the blocking operation may re-execute on replay (due to being outside the journal check), the *result used by the workflow* is consistently retrieved from the journal via `ctx.run()`, maintaining state determinism.

## Implementation Steps

1.  **Refactor `src/stage_executor.rs`**:
    *   Update `execute_stage_real` signature to be generic over `C: ContextSideEffects` to enable unit testing with mocks.
    *   Move `tokio::task::spawn_blocking` *before* `ctx.run()`.
    *   Capture the result of the blocking operation.
    *   Inside `ctx.run()`, only map the result to `Json<StageExecution>`.
    *   Ensure error handling converts `JoinError` to `OyaError` (outer) and logic errors to `HandlerError` (inner).

2.  **Add Deterministic Replay Test**:
    *   **Location:** `src/stage_executor.rs` (inside `#[cfg(test)] mod tests`) because `stage_executor` is a private module of `src/main.rs` and cannot be accessed by integration tests in `tests/`.
    *   **File `tests/deterministic_replay.rs`:** Create as a pointer file to satisfy the prompt's file requirement, documenting the location of the actual test.
    *   **Test Logic:**
        *   Implement `MockContext` trait for `ContextSideEffects`.
        *   Verify `execute_stage_real` calls `spawn_blocking` logic (via a mock or verifiable side-effect).
        *   Verify `ctx.run` is called with the result.
        *   Simulate Replay: Verify that if `ctx.run` returns a *cached* result (different from the new execution), the cached result is the one returned by the function.

## Quality Gates
*   `moon run :check`
*   `moon run :test`
*   `moon run :clippy`
*   Zero `unwrap()`, `expect()`, or `panic!()`.

## Verification
*   `tests/deterministic_replay.rs` (pointer)
*   `src/stage_executor.rs` (unit tests passing)
