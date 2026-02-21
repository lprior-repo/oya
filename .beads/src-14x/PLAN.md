# PLAN: src-14x - Stage-Scoped Write Allowlists

## Overview
Enforce stage-scoped write allowlists to prevent unauthorized file modifications during stage execution.

## Current State
- `src/runtime_tools/write_allowlist.rs` exists but is dead code (`#[allow(dead_code)]`)
- No validation occurs when OpenCode stages write files
- Cross-stage contamination is possible

## Implementation Tasks

### Phase 1: Update Write Allowlist Configuration
**File**: `src/runtime_tools/write_allowlist.rs`

1. Update `StageWriteConfig::contract_config()`:
   - Change `allowed_dirs: vec![PathBuf::from("docs")]` 
   - To: `allowed_dirs: vec![PathBuf::from(".beads/contracts")]`
   - Add pattern: `*.cue` for contract files

2. Update `StageWriteConfig::acceptance_test_config()`:
   - Keep `tests/` directory
   - Keep `*_test.rs`, `tests.rs`, `mod.rs` patterns
   - Add `proptest` module patterns if needed

3. Verify `StageWriteConfig::implementation_config()`:
   - Keep `src/`, `benches/`
   - Ensure `.beads/` is NOT in allowed dirs (prevents workflow orchestration bypass)

### Phase 2: Integrate Validation into Stage Execution
**File**: `src/stage_executor.rs`

1. Import write_allowlist in stage_executor:
   ```rust
   use crate::runtime_tools::write_allowlist::{validate_write_path, StageWriteConfig};
   ```

2. Add validation function:
   ```rust
   fn validate_stage_writes(
       stage: &Stage,
       paths: &[PathBuf],
       workspace_root: &Path,
   ) -> Result<(), OyaError>
   ```

3. Call validation before file persistence operations

### Phase 3: Enable Module Exports
**File**: `src/runtime_tools.rs`

1. Remove `#[allow(dead_code)]` from `mod write_allowlist`
2. Ensure exports are properly used

### Phase 4: Add Integration Tests
**File**: `src/runtime_tools/write_allowlist.rs` (add to existing tests)

1. Test Contract stage only allows `.beads/contracts/<bead_id>.cue`
2. Test AcceptanceTest stage blocks src/ writes
3. Test Implementation stage blocks `.beads/` writes
4. Test path traversal rejection with `../` sequences
5. Test cross-stage contamination prevention

## Test Strategy

### Unit Tests (in write_allowlist.rs)
- `test_contract_stage_allows_beads_contracts_directory` - Allow `.beads/contracts/foo.cue`
- `test_contract_stage_blocks_docs_outside_contracts` - Block `docs/README.md`
- `test_implementation_stage_blocks_beads_directory` - Block `.beads/state.json`
- `test_acceptance_test_stage_blocks_src_writes` - Block `src/lib.rs`

### Integration Tests (new file or add to lib_tests.rs)
- Test full stage execution with valid file writes
- Test rejection of invalid cross-stage writes
- Test error messages are actionable

## Quality Gates
1. `moon run :test` - All tests pass
2. `moon run :clippy` - No warnings
3. `moon run :fmt` - Code formatted

## Verification Commands
```bash
moon run :ci
```

## Dependencies
- None (uses existing write_allowlist module)

## Risk Mitigation
- Validation must not break existing valid workflows
- Error messages must clearly indicate which paths are allowed
- Path normalization must handle symlinks safely (current implementation uses string comparison)
