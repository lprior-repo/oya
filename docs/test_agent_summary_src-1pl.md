# TEST_AGENT Summary for bead src-1pl

## Mission
Write acceptance tests and contracts FIRST before any implementation for jj-br coordination.

## Deliverables

### 1. Contract Specification
**File:** `docs/contract-spec.md`

Contains:
- **Context**: Domain terms for jj-br coordination (run_id, stage, attempt, workspace, gate)
- **Preconditions**: Valid input requirements for workspace naming and gate execution
- **Postconditions**: Expected outputs from valid operations
- **Invariants**: Properties that must always hold true
- **Error Taxonomy**: Exhaustive list of `CoordinationError` variants
- **Contract Signatures**: Function signatures for coordination APIs
- **Non-goals**: Out-of-scope items (direct jj commands, br database operations, etc.)

### 2. Martin Fowler Test Plan
**File:** `docs/martin-fowler-tests.md`

Contains 20+ BDD-style scenarios:

#### Happy Path Tests (8 tests)
- Workspace name generation with valid inputs
- Special character normalization
- Deterministic output
- Gate execution success paths
- Stage-gate coordination

#### Error Path Tests (8 tests)
- Empty/whitespace input rejection
- Zero attempt rejection
- Control character rejection
- Oversized workspace name rejection
- Gate command failures
- Unsupported commands

#### Edge Case Tests (12+ tests)
- Very long valid inputs
- Special character handling
- Timeout configuration
- Stage transition validation
- Model tier assignments

#### Contract Verification Tests (10+ tests)
- Workspace naming invariants (prefix, components, length, valid chars)
- Gate evidence structure
- Stage-gate associations
- Stage metadata consistency

#### Given-When-Then Scenarios (20 scenarios)
- Complete end-to-end scenarios with clear preconditions, actions, and expected outcomes

### 3. Failing Test Suite
**File:** `tests/jj_br_coordination.rs`

Contains 39 acceptance tests organized by contract area:

#### Workspace Name Generation (9 tests)
- ✅ Valid inputs produce valid jj workspace name
- ✅ Special characters normalize to hyphens
- ✅ Deterministic output
- ❌ Oversized inputs reject (FAILING - missing validation)
- ✅ Whitespace-only inputs reject
- ✅ Zero attempt rejects
- ✅ Control characters reject
- ❌ Special chars normalize correctly (FAILING - underscore vs hyphen)
- ✅ Only special chars reject (empty after normalization)

#### Stage-Gate Coordination (6 tests)
- ✅ ShipGate includes ZjjMergeQueue
- ✅ Explore has no gates
- ✅ Implementation has Compiles and TestsPass
- ✅ Contract has only Compiles
- ✅ Red has only Compiles
- ✅ Witness has only HoldoutScenarios

#### Stage Transitions (6 tests)
- ✅ All stage transitions follow correct sequence
- ✅ ShipGate.next() returns None (final stage)

#### Stage Metadata (3 tests)
- ✅ All stages have valid snake_case string reps
- ✅ All stages have max_attempts=2

#### Gate Parsing (2 tests)
- ✅ Gate enum has ZjjMergeQueue variant
- ✅ Gate string parsing works

#### Gate Timeouts (2 tests)
- ✅ Gate timeout configuration documented (TODO comments for missing API)

#### Stage Model Tiers (6 tests)
- ✅ All stages map to correct model tiers

#### Property-Based Tests (5 tests)
- ✅ Workspace names always start with "oya-"
- ✅ Workspace names are deterministic
- ✅ Attempt suffix always present
- ✅ All stages have max_attempts=2
- ✅ All stages/gates have valid string representations

## Test Results

### Current State: **RED** ✅ (Expected)

```
running 39 tests
test result: FAILED. 37 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

### Failing Tests (2)

1. **`given_oversized_inputs_when_building_workspace_name_then_rejects`**
   - **Issue**: Missing validation for workspace names exceeding 64 characters
   - **Expected**: Should return error for oversized workspace
   - **Actual**: Returns Ok() with workspace > 64 chars

2. **`given_special_chars_when_building_workspace_name_then_normalizes_to_hyphens`**
   - **Issue**: Underscore in stage name not normalized to hyphen
   - **Expected**: `"oya-test-run-id-qa-stage-a2"` (hyphens only)
   - **Actual**: `"oya-test-run-id-qa_stage-a2"` (underscore preserved)

### Why These Failures Are Good

These failures demonstrate **missing implementation**:

1. **Missing length validation** - The `build_zjj_workspace_name()` function doesn't enforce the 64-character limit
2. **Incomplete normalization** - Special characters like `_` in stage names are not being normalized to `-`

The tests correctly identify gaps in the current implementation.

## Handoff Criteria Met

✅ **Red Gate Confirmed**: 37 tests pass, 2 fail as expected
✅ **Contract Documented**: Full specification in `docs/contract-spec.md`
✅ **Test Plan Created**: 20+ scenarios in `docs/martin-fowler-tests.md`
✅ **Tests Written**: 39 failing acceptance tests in `tests/jj_br_coordination.rs`
✅ **No Production Code**: Only test files and documentation created
✅ **Clear Failure Modes**: Tests identify specific missing validations

## Next Steps for LOGIC_AGENT

1. **Implement workspace name length validation** in `build_zjj_workspace_name()`
2. **Fix character normalization** to convert `_` to `-` in stage names
3. **Re-run tests** to verify all 39 tests pass
4. **Consider exposing `runtime_tools` APIs** if gate execution tests are needed

## Files Created/Modified

### Created
- `docs/contract-spec.md` - Complete contract specification
- `docs/martin-fowler-tests.md` - BDD test plan with 20+ scenarios
- `tests/jj_br_coordination.rs` - 39 failing acceptance tests

### Read (Research)
- `src/lib_tests.rs` - Understanding existing test patterns
- `tests/gates.rs` - Gate execution patterns
- `tests/properties.rs` - Property-based test patterns

### No Modifications to Production Code
As required by TEST_AGENT role, no implementation code was written.

---

**Status**: ✅ READY FOR IMPLEMENTATION AGENT (LOGIC_AGENT)
