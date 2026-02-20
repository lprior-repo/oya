# QA Refactor Plan - OYA Orchestrator

**Generated:** 2026-02-20
**Status:** BLOCK MERGE - Critical issues found

---

## Executive Summary

| Category | Count | Status |
|----------|-------|--------|
| Critical | 4 | BLOCK |
| Major | 8 | FIX BEFORE MERGE |
| Minor | 6 | BACKLOG |

---

## P0: CRITICAL (Block Merge)

### 1. LLM Routing Completely Broken

**Location:** `src/main.rs:109-110`

**Current Code (BROKEN):**
```rust
let model = parsed.model
    .map_or_else(|| "zai-coding-plan/glm-5".to_string(), std::convert::identity);
```

**Problem:**
- Hardcoded fallback model ignores tier-based routing
- `OyaUsageTracker.get_active_model()` is NEVER called
- `tier_for_stage()` is NEVER called in production
- No health tracking, no rotation, no circuit breaking

**Fix:**
```rust
// In build_start_context():
let tier = tier_for_stage(&Stage::Plan); // "c" for Plan stage
let model = ctx
    .object_client::<OyaUsageTracker>("default")
    .get_active_model(tier.to_string())
    .call()
    .await
    .map_err(|e| OyaError(format!("model selection failed: {}", e)))?;
```

**Files to modify:**
- `src/main.rs` - Wire up `get_active_model()` call
- `src/pipeline/executor.rs` - Call `report_outcome()` after stage completion

---

### 2. Security Vulnerabilities

**Command:** `moon run :security`

| ID | Crate | Severity | Fix |
|----|-------|----------|-----|
| RUSTSEC-2026-0009 | time v0.3.36 | CRITICAL | Upgrade to >=0.3.47 |
| RUSTSEC-2025-0111 | tokio-tar v0.3.1 | CRITICAL | No fix (dev dep only) |

**Action:**
1. Update `Cargo.toml` to force time >=0.3.47
2. Document accepted risk for tokio-tar (testcontainers dev dependency)

---

### 3. Broken Contract Test

**Location:** `tests/contract_verify.rs:70-86`

**Status:** FIXED (JSONL parsing)

The test now correctly parses JSONL format instead of expecting single JSON.

---

### 4. Functions Exceed Line Limit

**Status:** FIXED by refactoring

Functions have been refactored to be under 40 lines.

---

## P1: MAJOR (Fix Before Merge)

### 5. Missing Error Variant Tests

**Location:** `tests/behavior.rs` or `tests/state_machine.rs`

**Missing tests for:**
- `FailureCategory::TestsUnexpectedlyGreen`
- `FailureCategory::TestInfraFailed`

**Add tests:**
```rust
#[test]
fn tests_unexpectedly_green_triggers_rerun() {
    // ATDD gate should detect green tests during AcceptanceTest and retry
}

#[test]
fn test_infra_failed_is_non_retryable() {
    // TestInfraFailed should not trigger retry
}
```

---

### 6. Low Test Coverage

| Module | Coverage | Target |
|--------|----------|--------|
| `runtime_tools/command_exec.rs` | 4.90% | 50%+ |
| `stage_executor.rs` | 5.95% | 50%+ |
| `types/pipeline.rs` | 25.18% | 80%+ |
| `usage.rs` | 34.15% | 80%+ |

---

### 7. DDD Violation: OrchestratorState String Primitives

**Location:** `src/orchestrator_types.rs:66-76`

**Current (BAD):**
```rust
pub struct OrchestratorState {
    pub status: String,        // Should be enum
    pub stage: String,         // Should be StageName
    pub bead_id: String,       // Should be BeadId
    pub model: String,         // Should be ModelName
    pub last_failure: String,  // Should be Option<FailureDetail>
    pub updated_at: String,    // Should be DateTime<Utc>
}
```

**Refactor to:**
```rust
pub struct OrchestratorState {
    pub status: OrchestratorStatus,  // enum { Running, Shipped, Failed }
    pub stage: StageName,            // existing type
    pub bead_id: BeadId,             // newtype
    pub model: ModelName,            // newtype with validation
    pub last_failure: Option<FailureDetail>,
    pub updated_at: DateTime<Utc>,
}
```

---

### 8. DDD Violation: AgentState Option Combinations

**Location:** `src/types/domain.rs:82-122`

**Problem:** Multiple `Option` fields can combine into invalid states

**Refactor to type-state:**
```rust
pub enum AgentStatusVariant {
    Idle { last_update: DateTime<Utc> },
    Working {
        bead_id: BeadId,        // Required, not Option
        current_stage: StageName, // Required
        stage_started_at: DateTime<Utc>,
    },
    Error { error_message: String },
    Done,
}
```

---

### 9. DDD Violation: QualityGateResult Contradictory Bools

**Location:** `src/quality_gate/mod.rs:32-42`

**Problem:**
- `overall_passed: bool` can be true while `spec_passed: false`
- `failure_category: Option<FailureCategory>` can be None when failed

**Refactor to sum type:**
```rust
pub enum QualityGateResult {
    SpecFailed { iteration: u32, score: u32 },
    ScenariosFailed {
        iteration: u32,
        category: FailureCategory, // Required
        message: String,
    },
    Passed { iteration: u32, spec_score: u32 },
}
```

---

### 10. DDD Violation: PipelineRunInput String Primitives

**Location:** `src/pipeline/state.rs:61-65`

**Current:**
```rust
pub struct PipelineRunInput {
    pub run_id: String,   // Should be RunId
    pub bead_id: String,  // Should be BeadId
    pub context: String,  // Should be Context
}
```

---

### 11. Ad-hoc Tuple for Failure

**Location:** `src/pipeline/state.rs:70`

**Current:**
```rust
pub last_failure: Option<(FailureCategory, String)>,
```

**Refactor to:**
```rust
pub last_failure: Option<StageFailure>,

pub struct StageFailure {
    pub category: FailureCategory,
    pub message: String,
    pub failed_at: DateTime<Utc>,
    pub retryable: bool,
}
```

---

### 12. PathBuf vs Path

**Location:** `src/main.rs:416`, `src/runtime_tools/workspace.rs:62`

**Fix:** Change `&PathBuf` parameters to `&Path`

---

## P2: MINOR (Backlog)

### 13. Low Coverage Modules

Add integration tests for:
- `runtime_tools/command_exec.rs`
- `stage_executor.rs`

### 14. --version Flag Missing

**Location:** CLI parser

Add version flag support.

### 15. Unmaintained Dependencies

| Crate | Issue |
|-------|-------|
| fxhash | RUSTSEC-2025-0057 |
| instant | RUSTSEC-2024-0384 |
| paste | RUSTSEC-2024-0436 |
| rustls-pemfile | RUSTSEC-2025-0134 |
| lru | RUSTSEC-2026-0002 |

---

## Verification Commands

```bash
# Run all tests
moon run :test

# Check clippy
moon run :clippy

# Security audit
moon run :security

# Coverage
moon run :coverage

# Full CI
moon run :ci
```

---

## Task Tracking

| Task | Status | Priority |
|------|--------|----------|
| Fix LLM routing | IN PROGRESS | P0 |
| Fix security vulnerabilities | PENDING | P0 |
| Fix broken contract test | DONE | P0 |
| Refactor oversized functions | DONE | P0 |
| Add missing error variant tests | PENDING | P1 |
| DDD: OrchestratorState types | PENDING | P1 |
| DDD: AgentState type-state | PENDING | P1 |
| DDD: QualityGateResult sum type | PENDING | P1 |
| Increase test coverage | PENDING | P1 |
| Fix code formatting | DONE | P0 |

---

## What's Already Good

- Restate SDK patterns correctly implemented
- 553 unit tests passing
- Zero panic/unwrap in production code
- Good DDD patterns in `beads/` and `tail/` modules
- Proper `ctx.run()` usage for determinism
