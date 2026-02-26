# REVERSE PROMPT: Fix spawn_blocking Journaling in execute_stage_real

## Context

You are fixing a CRITICAL determinism bug in the OYA workflow executor. The previous implementation was hallucinated and did NOT address the actual bug. This is a P0 (highest priority) bug that MUST be fixed correctly.

## The Actual Bug

**Location:** `src/stage_executor.rs:51-82` in function `execute_stage_real`

**Current Code (BROKEN):**
```rust
pub(super) async fn execute_stage_real(
    ctx: &WorkflowContext<'_>,
    request: StageExecutionRequest,
    merge_queue_policy: MergeQueuePolicy,
    repo_root: PathBuf,
) -> Result<(StageResult, String), OyaError> {
    let input = StageBlockingInput { request: request.clone(), merge_queue_policy, repo_root };
    let execution: Json<StageExecution> = ctx
        .run(|| async move {
            let result = tokio::task::spawn_blocking(move || execute_stage_blocking(input))
                .await
                .map_err(|error| HandlerError::from(format!("spawn_blocking failed: {}", error)))?;
            result.map(Json).map_err(|e| HandlerError::from(e.0))
        })
        .await
        .map_err(|e| OyaError(format!("ctx.run failed: {}", e)))?;
    // ... rest
}
```

**The Problem:**
1. Non-deterministic stage execution can diverge between original run and replay if the closure body does not remain fully journal-consistent.
2. Prior fixes changed behavior without preserving replay consistency at state-write boundaries.
3. We must enforce one journaled execution contract for full stage execution and avoid partial journaling patterns.
4. Any divergence in command output/state payload across replay causes Restate mismatch failures.
5. The fix must include replay-safe implementation and mock-based contract tests.

**Why This Breaks Restate:**
- Restate journals the RESULT of `ctx.run()`
- On replay, it should use the journaled result WITHOUT re-executing
- But `spawn_blocking` inside the closure means it gets re-executed on every replay
- This is a fundamental violation of Restate's determinism guarantee

## The Correct Pattern

Reference: `src/pipeline/state.rs:17-21` shows the correct pattern:

```rust
pub(crate) async fn deterministic_timestamp(
    ctx: &WorkflowContext<'_>,
) -> Result<String, TerminalError> {
    ctx.run(|| async move { Ok::<_, HandlerError>(chrono::Utc::now().to_rfc3339()) }).await
}
```

**Key Insight:** For this workflow, stage execution must be treated as one journaled side-effect boundary so replay reads the same recorded value at subsequent state writes.

## Your Task

### Step 1: Refactor `execute_stage_real` to Preserve Replay Consistency

**WRONG (caused mismatch):**
```rust
let blocking_result = tokio::task::spawn_blocking(move || execute_stage_blocking(input)).await ...;
let execution: Json<StageExecution> = ctx.run(move || async move {
    blocking_result.map(Json) ...
}).await
```

**CORRECT:**
```rust
// 1. Keep full stage execution inside one journaled closure
let execution: Json<StageExecution> = ctx
    .run(move || async move {
        let result = tokio::task::spawn_blocking(move || execute_stage_blocking(input))
            .await
            .map_err(|error| HandlerError::from(format!("spawn_blocking failed: {}", error)))?;
        result.map(Json).map_err(|e| HandlerError::from(e.0))
    })
    .await
    .map_err(|e| OyaError(format!("ctx.run failed: {}", e)))?;
```

### Step 2: Update Error Handling

- The outer `spawn_blocking` error should be converted to `OyaError` (not `HandlerError`)
- Only the inner result mapping errors become `HandlerError` inside `ctx.run()`

### Step 3: Add Deterministic Replay Tests

Create a new test file `tests/deterministic_replay.rs` that:

```rust
#[tokio::test]
async fn test_execute_stage_real_deterministic_replay() {
    // 1. Mock a workflow context that simulates replay
    // 2. Execute execute_stage_real once
    // 3. Verify the result is journaled
    // 4. Simulate replay: call again with same inputs
    // 5. ASSERT: spawn_blocking is NOT called again (use mock counters)
    // 6. ASSERT: journaled result is used directly
}
```

### Step 4: Document the Pattern

Add a doc comment to `execute_stage_real`:

```rust
/// Executes a stage with deterministic Restate journaling.
///
/// # Determinism Contract
///
/// This function MUST ensure that on workflow replay:
/// - The blocking operation (`spawn_blocking`) is NOT re-executed
/// - Only the journaled result is used
/// - Each unique input produces the same output every time
///
/// # Implementation Pattern
///
/// 1. Execute `spawn_blocking` OUTSIDE of `ctx.run()` (non-deterministic part)
/// 2. Wrap result transformation in `ctx.run()` (deterministic journaling)
///
/// # Why This Matters
///
/// If `spawn_blocking` is inside `ctx.run()`, Restate will re-execute it on every replay,
/// breaking determinism and potentially causing state divergence.
pub(super) async fn execute_stage_real(...) -> Result<...>
```

### Step 5: Verify All Execution Paths

Check that other similar functions in `src/stage_executor.rs` follow the same pattern:
- `execute_stage_blocking` (lines 84-96) - already correct (sync function)
- `execute_prompt_driven_stage` (lines 98-120) - already correct (sync)
- `execute_prompt_stage` (lines 122-145) - already correct (sync)

## Acceptance Criteria

1. ✅ Stage execution closure remains fully journaled in `ctx.run()`
2. ✅ Replay reuses journaled value and does not re-run mocked execution branch
3. ✅ Error handling properly separates OyaError (outer) from HandlerError (inner)
4. ✅ Test added that verifies spawn_blocking is not called on replay
5. ✅ Documentation explains the determinism pattern
6. ✅ `moon run :clippy` passes (no unwrap/expect/panic)
7. ✅ `moon run :test` passes (all tests green)
8. ✅ Code review confirms no other functions have this anti-pattern

## What NOT To Do

❌ Do NOT rename services (that was hallucinated work)
❌ Do NOT add unrelated design contracts
❌ Do NOT modify scripts or configuration
❌ Do NOT add "binary testing" contracts (completely unrelated)
❌ Do NOT use `.unwrap()` or `.expect()`
❌ Do NOT modify clippy configuration

## Files To Modify

1. `src/stage_executor.rs` - Fix the actual bug (lines 51-82)
2. `tests/deterministic_replay.rs` - Add new test file
3. `src/stage_executor.rs` - Update documentation

## Files NOT To Modify

- ❌ `src/main.rs` - no service renames needed
- ❌ `src/usage.rs` - no changes needed
- ❌ `src/workflow_runner/poller.rs` - no changes needed
- ❌ `scripts/*.sh` - no changes needed
- ❌ `src/lib.rs` - no unrelated contracts

## Verification Commands

After implementing, run:

```bash
# Check the fix doesn't break anything
moon run :check

# Run tests
moon run :test

# Verify clippy compliance
moon run :clippy

# Full CI
moon run :ci
```

## Summary

**The bug:** `spawn_blocking` inside `ctx.run()` breaks Restate determinism
**The fix:** Move `spawn_blocking` outside `ctx.run()`, journal only the result
**The test:** Verify spawn_blocking is not called on replay
**The pattern:** Non-deterministic work outside, result journaling inside

This is a critical correctness bug. Fix it properly this time.
