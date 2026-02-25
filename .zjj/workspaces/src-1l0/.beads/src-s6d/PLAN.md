# PLAN: src-s6d - Contract Stage CUE Artifact Validation

## Overview
Enforce canonical CUE contract artifact requirement for the Contract stage. Exactly one `.cue` file must be generated at the canonical path `.beads/schemas/oya-{timestamp}-{suffix}.cue` before advancing to AcceptanceTest.

## Current State
- `src/beads/cue_artifact.rs` has `CueArtifact::generate()` and `relative_path()` 
- `src/beads/cue_artifact.rs` has `validate_cue_artifact_requirement()` (unused)
- `src/pipeline/executor.rs` builds `StageArtifact` after stage execution
- `src/stage_executor.rs` executes stages and returns `StageExecution`
- No validation occurs that Contract stage produces exactly one CUE file
- Contract output goes into `StageOutputData.contract_document` but path is not validated

## Canonical Path Contract
- Pattern: `.beads/schemas/oya-{YYYYMMDDHHMMSS}-{bead_suffix}.cue`
- Example: `.beads/schemas/oya-20260220154435-htwdhr6l.cue`
- ONE file per bead (deterministic, no ambiguity)

## Implementation Tasks

### Phase 1: Add Contract Artifact Validation Type
**File**: `src/beads/cue_artifact.rs`

1. Add new error variant to `CueArtifactError`:
   - `NoContractArtifact { expected_path: String }`
   - `MultipleContractArtifacts { found_paths: Vec<String> }`
   - `NonCanonicalContractPath { actual: String, expected: String }`

2. Add validation function:
   ```rust
   pub fn validate_contract_stage_artifact(
       bead_id: &BeadId,
       schema_dir: &Path,
   ) -> Result<String, CueArtifactError>
   ```
   - Scan `.beads/schemas/` for files matching pattern `oya-*-{suffix}.cue`
   - Return error if zero or more than one match
   - Return canonical path on success

3. Add helper to extract canonical path from existing files:
   ```rust
   fn find_canonical_contract(bead_suffix: &str, schema_dir: &Path) -> Result<PathBuf, CueArtifactError>
   ```

### Phase 2: Integrate Validation into Stage Execution
**File**: `src/stage_executor.rs`

1. Import cue_artifact validation:
   ```rust
   use crate::beads::cue_artifact::{validate_contract_stage_artifact, CueArtifactError};
   ```

2. Add validation after Contract stage completes in `execute_prompt_stage`:
   ```rust
   if request.stage == Stage::Contract && opencode_ok {
       let schema_dir = request.repo_root.join(".beads/schemas");
       let contract_path = validate_contract_stage_artifact(
           &BeadId::new(/* bead_id */),
           &schema_dir,
       ).map_err(|e| OyaError(format!("contract validation failed: {}", e)))?;
   }
   ```

3. Store canonical path in `StageExecution`:
   - Add field: `pub contract_path: Option<String>`
   - Populate only for Contract stage

### Phase 3: Persist Canonical Path in StageArtifact
**File**: `src/orchestrator_types.rs`

1. Add field to `StageArtifact`:
   ```rust
   pub contract_path: Option<String>,
   ```

2. Update `build_stage_artifact` in `src/pipeline/executor.rs` to include contract path

### Phase 4: Add Tests
**File**: `src/beads/cue_artifact.rs` (append to existing tests)

1. `test_validate_contract_stage_artifact_single_match` - One CUE file returns path
2. `test_validate_contract_stage_artifact_no_match` - Zero files returns error
3. `test_validate_contract_stage_artifact_multiple_match` - Two files returns error
4. `test_validate_contract_stage_artifact_non_canonical_rejected` - Wrong path rejected

**File**: `tests/contract_verify.rs` (add integration tests)

1. `test_contract_stage_requires_cue_artifact` - Contract stage fails without CUE
2. `test_contract_stage_rejects_multiple_cue_artifacts` - Ambiguity rejected

## Test Strategy

### Unit Tests (in cue_artifact.rs)
- Single CUE file validation passes
- Zero CUE files returns `NoContractArtifact` error
- Multiple CUE files returns `MultipleContractArtifacts` error
- Non-canonical path returns `NonCanonicalContractPath` error
- Error messages include expected path for debugging

### Integration Tests (in contract_verify.rs)
- Contract stage execution fails without artifact
- Contract stage execution succeeds with single canonical artifact
- Error message is actionable (tells user where file should be)

## Quality Gates
1. `moon run :test` - All tests pass (RED first, then GREEN)
2. `moon run :clippy` - No warnings
3. `moon run :fmt` - Code formatted

## Verification Commands
```bash
moon run :ci
```

## Dependencies
- Uses existing `src/beads/cue_artifact.rs` module
- Requires `.beads/schemas/` directory to exist (created by planner)

## Risk Mitigation
- Validation must not break existing valid workflows
- Error messages must include expected canonical path
- Must handle case where `.beads/schemas/` doesn't exist
- Must not race with concurrent schema file creation
