# PLAN: Acceptance - Enforce Red Gate Before Implementation Dispatch

## Summary
Implement a "red gate" that blocks implementation stage dispatch until acceptance tests are confirmed RED (failing). Ensures TDD discipline by requiring tests to fail before implementation begins.

## Gap Analysis
The `StageName` and `Gate` enums in `src/types/pipeline.rs` are missing variants that tests expect:
- Test at `tests/pipeline_logic.rs:148-155` expects `StageName::AcceptanceTest` and `Gate::AcceptanceTestsAreRed`
- Production code references `Plan`, `Tdd15`, `Qa`, `RedQueen`, `GptReview` stages

**This bead focuses ONLY on**: AcceptanceTest stage + AcceptanceTestsAreRed gate.

## Implementation Steps

### Step 1: Extend Gate Enum
**File**: `src/types/pipeline.rs:115-120`
```rust
pub enum Gate {
    Compiles,
    TestsPass,
    MoonCi,
    ZjjMergeQueue,
    CueArtifactGenerated,  // already referenced in gates()
    AcceptanceTestsAreRed, // NEW
}
```
- Add `AcceptanceTestsAreRed` variant
- Add `as_str()` match arm: `"acceptance_tests_are_red"`
- Add `TryFrom<&str>` match arm

### Step 2: Extend StageName Enum  
**File**: `src/types/pipeline.rs:20-24`
```rust
pub enum StageName {
    Contract,
    AcceptanceTest,    // NEW
    Implementation,
    ShipGate,
}
```
- Add `AcceptanceTest` variant
- Update `as_str()`: `"acceptance_test"`
- Update `next()`: Contract -> AcceptanceTest -> Implementation -> ShipGate
- Update `model_for_stage()`: AcceptanceTest -> Fast
- Update `gates()`: AcceptanceTest -> `[Gate::Compiles, Gate::AcceptanceTestsAreRed]`

### Step 3: Add CueArtifactGenerated to Gate
**File**: `src/types/pipeline.rs:115-120`
- Add `CueArtifactGenerated` variant (referenced in Contract gates)
- Add `as_str()`: `"cue_artifact_generated"`
- Add `TryFrom` match arm

### Step 4: Red Gate Execution Logic
**File**: `src/runtime_tools/gates.rs`
- In `parse_gate_command_parts()`, map `AcceptanceTestsAreRed` to `moon run :test`
- Gate passes when `exit_code != 0` (tests fail = RED = gate passes)

### Step 5: Failure Routing
**File**: `src/runtime_tools/gates.rs:133-157`
- Add mapping in `gate_failure_mapping()`:
  ```rust
  (&Stage::AcceptanceTest, &Gate::AcceptanceTestsAreRed) => {
      Some((FailureCategory::TestsUnexpectedlyGreen, Stage::AcceptanceTest))
  }
  ```

### Step 6: Stage Runtime
**File**: `src/stage_runtime.rs:60-72`
- Add AcceptanceTest to `stage_success()`:
  ```rust
  Stage::AcceptanceTest => ("Tests compile and are RED", Some(Stage::Implementation)),
  ```

## Test Strategy

### Existing Tests (will compile after implementation)
- `tests/pipeline_logic.rs:148-155`: `given_acceptance_test_stage_when_checking_gates_then_compiles_and_red_required`

### New Tests to Write
- `test_acceptance_tests_are_red_gate_passes_on_failure`: exit_code=1 means passed
- `test_acceptance_tests_are_red_gate_fails_on_success`: exit_code=0 means failed
- `test_green_acceptance_routes_back_to_acceptance_test`: failure routing

## Quality Gates
```
moon run :check   # Must pass
moon run :test    # Must pass
moon run :clippy  # Zero warnings
```

## Constraints
- Result<T,E> throughout, no unwrap/expect/panic
- Functions <= 40 lines, <= 5 args
- No #[allow(...)] suppressions

## Files Modified
1. `src/types/pipeline.rs` - Gate + StageName enums
2. `src/runtime_tools/gates.rs` - Gate execution + failure mapping
3. `src/stage_runtime.rs` - Stage success mapping
