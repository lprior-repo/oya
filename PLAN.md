# PLAN: Fix spawn_blocking Determinism in execute_stage_real

## Context
Refactor `execute_stage_real` to move `spawn_blocking` *inside* of `ctx.run()`. This ensures that the blocking operation (which contains non-deterministic side effects like LLM calls) is journaled by Restate and **not re-executed on replay**.

## Implementation Steps

1.  **Refactor `src/stage_executor.rs`**:
    *   Update `execute_stage_real` implementation.
    *   Move the `run_stage_blocking` call inside the closure passed to `ctx.run()`.
    *   Ensure the closure captures `input` by value.
    *   Map the result to `Json<StageExecution>` inside the closure.
    *   Update documentation comments to reflect the correct pattern ("Execute inside `ctx.run()`").

2.  **Update Unit Tests**:
    *   **Location:** `src/stage_executor.rs` (inside `#[cfg(test)] mod tests`).
    *   Update `test_execute_stage_real_deterministic_replay_pattern` to verify that the execution logic is wrapped in the journaled future.
    *   Since `WorkflowContext` is hard to mock directly without the test suite, focus on verifying the function composition logic.

3.  **Manual Runtime Verification**:
    *   **Goal**: Verify that stage execution happens only once, even across restarts/replays.
    *   **Procedure**:
        1.  Start infrastructure: `scripts/dev-up.sh`
        2.  Trigger a workflow execution (e.g., `oya run --bead src-2nw`).
        3.  Observe logs (`docker logs oya-restate -f`) for "Executing stage..." messages.
        4.  Force a service restart during or after execution.
        5.  Verify that upon recovery/replay, the "Executing stage..." logic is NOT re-triggered, but the workflow proceeds.

## Quality Gates
*   `moon run :check` passes
*   `moon run :test` passes
*   `moon run :clippy` passes (no new warnings)
*   Zero `unwrap()`, `expect()`, or `panic!()`
*   Manual verification successful

## Verification
*   `src/stage_executor.rs` (unit tests)
*   Manual runtime verification logs
