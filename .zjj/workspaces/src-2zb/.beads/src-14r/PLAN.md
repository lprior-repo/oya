# PLAN: src-14r - Incomplete retryable failure classification

**Status**: DONE (closed)
**Type**: bug
**Priority**: 0
**Effort**: 1hr

## Problem

Code-level failures that AI can fix were incorrectly classified as non-retryable, causing premature pipeline termination when the AI could have fixed the issue on retry.

## Root Cause

`is_retryable_failure()` in `src/lib.rs:80-90` was missing `CompileFailed` from the retryable categories list.

## Solution

### Implementation Location
- `src/lib.rs:80-90` - `is_retryable_failure()` function

### Retryable Categories (AI-fixable)
| Category | Retryable | Rationale |
|----------|-----------|-----------|
| TestFailed | YES | AI can fix failing tests |
| TestsUnexpectedlyGreen | YES | AI can adjust test expectations |
| LintFailed | YES | AI can fix lint errors |
| OutputParseFailure | YES | AI can fix output format |
| CompileFailed | YES | AI can fix compilation errors |
| CiFailed | YES | AI can fix CI failures |

### Non-Retryable Categories (External intervention required)
| Category | Retryable | Rationale |
|----------|-----------|-----------|
| TestInfraFailed | NO | Infrastructure issue |
| MergeConflict | NO | Requires human resolution |
| RateLimited | NO | External rate limiting |
| AuthFailed | NO | External auth issue |
| ContextOverflow | NO | Token limit exceeded |
| ProviderUnavailable | NO | External service issue |
| MaxAttemptsExceeded | NO | Terminal state |

## Implementation Steps

1. **Research** (Gate 0)
   - [x] Review `FailureCategory` enum in `src/types/pipeline.rs:171-185`
   - [x] Review existing `is_retryable_failure()` in `src/lib.rs:80-90`
   - [x] Identify missing retryable categories

2. **Tests** (Gate 1)
   - [x] `test_compile_failed_is_retryable` in `tests/state_machine.rs:118-135`
   - [x] `test_infra_failed_is_non_retryable` in `src/main/tests.rs:378`

3. **Implementation** (Gate 2)
   - [x] Add `CompileFailed` to retryable list
   - [x] Review other categories for inclusion (all correctly classified)

4. **Verification** (Gate 3)
   - [x] `moon run :ci` passes
   - [x] All tests green

## Test Strategy

### Unit Tests
```rust
// tests/state_machine.rs:118-135
async fn test_compile_failed_is_retryable()
    - Given: Stage fails with CompileFailed
    - When: Pipeline evaluates retry
    - Then: next_stage == Some(StageName::Implementation) // Can retry

// src/main/tests.rs:378
fn test_infra_failed_is_non_retryable()
    - Given: FailureCategory::TestInfraFailed
    - When: is_retryable_failure() called
    - Then: returns false
```

### Acceptance Criteria
- [x] All code-level failures (compile, test, lint, parse) are retryable
- [x] Infrastructure failures remain non-retryable
- [x] `moon run :ci` passes
- [x] Zero unwrap/expect/panic in implementation

## Quality Gates

| Gate | Requirement | Status |
|------|-------------|--------|
| Gate 0 | Research complete | PASS |
| Gate 1 | Tests written and failing before fix | PASS |
| Gate 2 | All tests pass | PASS |
| Gate 3 | `moon run :ci` green | PASS |

## Files Modified

- `src/lib.rs:80-90` - `is_retryable_failure()` function

## Verification Commands

```bash
moon run :ci
moon run :test -- test_compile_failed_is_retryable
moon run :test -- test_infra_failed_is_non_retryable
```
