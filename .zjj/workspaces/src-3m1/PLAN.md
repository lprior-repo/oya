# PLAN: src-s6d - Contract CUE Artifact Enforcement

## Summary
Enforce strict output validation for the "Contract" stage. The system must ensure exactly one canonical CUE artifact is generated. If zero, multiple, or non-CUE artifacts are produced, the stage must fail.

## Context
- **Bead:** `src-s6d` (contract: require per-bead cue artifact generation)
- **Goal:** Ensure deterministic governance boundary via typed CUE contracts.
- **Constraint:** Zero unwrap/panic. Functional Rust only.

## Current State
- `src/stage_executor.rs` likely handles stage execution and artifact collection.
- Current behavior likely accepts any number of artifacts or doesn't validate specific file extensions for the Contract stage.

## Phase 1: Research & Discovery
1.  **Analyze `src/stage_executor.rs`**:
    - Identify `execute_stage` or equivalent function.
    - Locate where `StageOutput` or artifacts are processed.
2.  **Analyze `src/orchestrator_types.rs`**:
    - Check `StageResult` and `Artifact` definitions.
    - specific `StageType::Contract` enum variant existence.

## Phase 2: Tests (TEST_AGENT)
**File:** `src/stage_executor/tests.rs` (or `src/stage_executor.rs` if inline)

1.  **Test: `test_contract_stage_zero_artifacts_fails`**
    - Setup: Mock Contract stage returning empty artifact list.
    - Expect: `Err(StageError::MissingContractArtifact)`

2.  **Test: `test_contract_stage_multiple_artifacts_fails`**
    - Setup: Mock Contract stage returning 2 CUE files.
    - Expect: `Err(StageError::AmbiguousContractArtifact)`

3.  **Test: `test_contract_stage_non_cue_artifact_fails`**
    - Setup: Mock Contract stage returning `contract.json`.
    - Expect: `Err(StageError::InvalidContractArtifactType)`

4.  **Test: `test_contract_stage_valid_single_cue_success`**
    - Setup: Mock Contract stage returning `contract.cue`.
    - Expect: `Ok(_)` with canonical path verified.

## Phase 3: Implementation (LOGIC_AGENT)
**File:** `src/stage_executor.rs`

1.  **Modify `validate_stage_output` (or equivalent)**:
    - Add matching on `StageType::Contract`.
    - Implement functional check:
      ```rust
      // Pseudo-code
      let contracts: Vec<_> = artifacts.iter().filter(|a| a.path.ends_with(".cue")).collect();
      match contracts.len() {
          0 => Err("No CUE contract found"),
          1 => Ok(contracts[0]),
          _ => Err("Multiple CUE contracts found"),
      }
      ```
2.  **Update Error Types**:
    - Add necessary variants to `StageError` in `src/orchestrator_types.rs` (if needed) or reuse existing validation errors.

## Quality Gates
1.  **Red Gate**: Tests written and failing. (Verified by `moon run :test`)
2.  **Green Gate**: Implementation makes tests pass. Zero clippy warnings.
3.  **Integration**: `moon run :ci` passes.

## Verification Commands
```bash
# Run specific tests
moon run :test -- --package oya --lib stage_executor

# Full CI
moon run :ci
```
