# PLAN: src-2nf - Shrink Canonical Stage Graph to Five Stages

## Status: In Progress

## Fixed Issue (Attempt 2)
- **Problem**: Compile failed with `unresolved import 'oya::runtime_tools'` in `workflow_runner.rs`
- **Root Cause**: `workflow_runner.rs` (binary module) used `oya::runtime_tools` but `runtime_tools` wasn't exported from lib.rs
- **Fix**: Changed to `crate::runtime_tools` since both are in the same binary crate

## Target Architecture

**Five Stages (reduced from 9):**
1. `Contract` - Planning + design contract (merges Plan → Contract)
2. `AcceptanceTest` - RED test writing (unchanged)
3. `Implementation` - GREEN implementation (absorbs Tdd15)
4. `Qa` - All quality gates combined (merges Qa + RedQueen + GptReview)
5. `ShipGate` - Final shipping gate (unchanged)

## Files to Modify

| File | Lines | Changes |
|------|-------|---------|
| `src/types/pipeline.rs` | 20-92 | StageName enum, as_str(), next(), model_for_stage(), gates() |
| `src/types/pipeline.rs` | 94-111 | TryFrom<&str> for StageName |
| `src/types/pipeline.rs` | 383-422 | passed_stage_transition() |
| `src/stage_runtime.rs` | 29-55 | stage_prompt() match arms |
| `src/stage_runtime.rs` | 60-72 | stage_success() match arms |
| `src/runtime_tools/workspace.rs` | 9-18 | stage_uses_workspace() - remove Tdd15, RedQueen, GptReview |
| `src/runtime_tools/gates.rs` | 146-165 | gate_failure_mapping() - update Stage variants |
| `tests/state_machine.rs` | 35-45 | test_all_stage_transitions test cases |
| `tests/state_machine.rs` | 72-88 | test_test_failed_retries (uses Tdd15) |
| `tests/state_machine.rs` | 92-118 | test_max_attempts_exceeded (uses Tdd15) |
| `tests/state_machine.rs` | 122-138 | test_compile_failed_is_retryable (uses Tdd15) |
| `tests/state_machine.rs` | 186-197 | test_complete_pipeline_simulation stages |

## Implementation Steps

### Phase 1: Tests (RED)

1. **Update `tests/state_machine.rs`**
   - Change `Tdd15` references to `Implementation`
   - Remove `Plan`, `RedQueen`, `GptReview` from transition test cases
   - New expected transitions:
     - `(Contract, Some(AcceptanceTest))`
     - `(AcceptanceTest, Some(Implementation))`
     - `(Implementation, Some(Qa))`
     - `(Qa, Some(ShipGate))`
     - `(ShipGate, None)`

2. **Add new tests**
   - `test_removed_stage_string_rejected` - verify parsing "plan"/"tdd15"/"red_queen"/"gpt_review" fails
   - `test_contract_is_first_stage` - Contract has no predecessor in transitions
   - `test_shipgate_is_terminal` - ShipGate.next() returns None

### Phase 2: Implementation (GREEN)

1. **Update `StageName` enum** (`src/types/pipeline.rs:20-30`)
   ```rust
   pub enum StageName {
       Contract,
       AcceptanceTest,
       Implementation,
       Qa,
       ShipGate,
   }
   ```

2. **Update `as_str()`** (line 33-45)
   - Remove: `plan`, `tdd15`, `red_queen`, `gpt_review`
   - Keep: `contract`, `acceptance_test`, `implementation`, `qa`, `ship_gate`

3. **Update `next()`** (line 47-59)
   ```rust
   Contract => Some(AcceptanceTest),
   AcceptanceTest => Some(Implementation),
   Implementation => Some(Qa),
   Qa => Some(ShipGate),
   ShipGate => None,
   ```

4. **Update `model_for_stage()`** (line 61-73)
   - Contract: Fast
   - AcceptanceTest: Balanced
   - Implementation: Balanced
   - Qa: Capable (needs powerful model for adversarial + review)
   - ShipGate: Best

5. **Update `gates()`** (line 79-91)
   - Contract: `[Compiles]`
   - AcceptanceTest: `[Compiles, AcceptanceTestsAreRed]`
   - Implementation: `[Compiles, TestsPass]`
   - Qa: `[TestsPass, EdgeCases, NoVulnerabilities, ClippyClean, Security]`
   - ShipGate: `[MoonCi, ZjjMergeQueue]`

6. **Update `TryFrom<&str>`** (line 94-111)
   - Remove: `plan`, `tdd15`, `red_queen`, `gpt_review`
   - Unknown stages return error (no silent mapping)

7. **Update `passed_stage_transition()`** (line 383-422)
   - Remove: Plan, Tdd15, RedQueen, GptReview branches
   - Update: Contract → AcceptanceTest, AcceptanceTest → Implementation, Implementation → Qa, Qa → ShipGate

8. **Update `stage_prompt()`** (`src/stage_runtime.rs:29-55`)
   - Remove: Plan, Tdd15, RedQueen, GptReview match arms
   - Update Contract prompt to include planning task

9. **Update `stage_success()`** (`src/stage_runtime.rs:60-72`)
   - Remove: Plan, Tdd15, RedQueen, GptReview match arms

10. **Update `stage_uses_workspace()`** (`src/runtime_tools/workspace.rs:9-18`)
    - Remove: Tdd15, RedQueen, GptReview from matches!()

11. **Update `gate_failure_mapping()`** (`src/runtime_tools/gates.rs:146-165`)
    - Remove Plan, Tdd15, RedQueen, GptReview branches
    - Map to new 5-stage model

## Test Strategy & Quality Gates

### Gate 1: Tests Written (RED)
- All tests compile
- `test_all_stage_transitions` uses 5 stages
- `test_removed_stage_string_rejected` fails (expects error)
- `moon run :test` shows failures

### Gate 2: Tests Pass (GREEN)
- `moon run :test` passes all tests
- `moon run :check` passes (clippy clean)
- `moon run :ci` passes full pipeline

### Gate 3: E2E Validation
```bash
moon run :ci
```

## Verification Commands
```bash
moon run :test
moon run :check
moon run :ci
```

## Files Modified Summary
- `src/types/pipeline.rs` - StageName enum and transition logic
- `src/stage_runtime.rs` - Stage prompts and success handlers
- `src/runtime_tools/workspace.rs` - Workspace stage filter
- `src/runtime_tools/gates.rs` - Gate failure mapping
- `tests/state_machine.rs` - Updated test expectations

## Risk Assessment
- **Medium risk**: Test file heavily references removed stages
- **State migration**: Existing runs with old stage names fail - acceptable for dev runtime
- **No external API breakage**: Stage names are internal to orchestrator

## Validation Checklist

- [x] Compile error fixed (`crate::runtime_tools` path)
- [ ] `StageName` has exactly 5 variants
- [ ] All transitions: Contract → AcceptanceTest → Implementation → Qa → ShipGate
- [ ] ShipGate.next() returns None
- [ ] Parsing "plan" returns error (not silent mapping)
- [ ] `moon run :ci` passes
