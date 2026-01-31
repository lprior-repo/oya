# AI Ergonomics Beads & Test Suite - Complete Implementation

**Date:** 2026-01-30
**Epic:** intent-cli-72pl (AI Ergonomics Overhaul)
**Status:** Ready for implementation

---

## Executive Summary

Successfully created a complete decomposition of AI agent friction points discovered during dogfooding into 17 beads with proper dependencies, acceptance criteria, and comprehensive test coverage.

### Statistics
- **Total Beads Created:** 17 (1 epic + 16 sub-beads)
- **Critical (P0):** 4 beads
- **High Priority (P1):** 4 beads  
- **Medium Priority (P2):** 4 beads
- **Documentation (P2):** 2 beads
- **Test Files Created:** 6 comprehensive test suites
- **Testing Pattern:** ATDD, BDD, Error Case Matrix

---

## Epic: intent-cli-72pl

**Title:** AI Ergonomics Overhaul - Fix all friction points discovered during dogfooding

**Description:** Systematic fix of all AI agent friction points discovered during dogfooding. Critical issues block AI usage, high priority issues significantly impact UX. All beads include ATDD tests, BDD scenarios, and error case coverage.

---

## Phase 1: Research & Pattern Discovery (2 beads)

### intent-cli-vgsf: Document ATDD/BDD testing patterns from existing tests
- **Type:** task
- **Priority:** 2
- **Labels:** ergonomics,research,testing-patterns
- **Status:** open
- **Dependencies:** None

**User Journey:**
```gherkin
Scenario: Understand existing test infrastructure
  Given I examine test/integration_e2e_test.gleam
  And I look at test/exit_code_test.gleam
  When I analyze the patterns
  Then I can extract reusable ATDD/BDD patterns
  And patterns can be applied to new bead creation
```

### intent-cli-wwrl: Document bead template patterns for user journey testing
- **Type:** task
- **Priority:** 2
- **Labels:** ergonomics,research,bead-templates
- **Status:** open
- **Dependencies:** None

---

## Phase 2: Critical Issues (4 beads - Blocking AI Usage)

### intent-cli-ysa0: CRITICAL: Fix spec generation - interview creates invalid specs
- **Type:** bug
- **Priority:** 0 (Critical)
- **Labels:** ergonomics,critical,spec-generation,interview
- **Status:** open
- **Dependencies:** intent-cli-72pl

**User Journey:**
```gherkin
Scenario: AI agent follows standard workflow
  Given I start an interview with "intent interview --profile cli"
  When interview completes
  Then I can run "intent validate <spec>" successfully
  And I can run "intent quality <spec>" successfully
  And spec has a valid top-level "spec" field
```

**Acceptance Criteria (ATDD):**
- `interview_generates_valid_spec_structure_test()` - Spec has top-level spec field
- `interview_spec_includes_required_fields_test()` - Spec has features, commands, security sections
- `user_journey_interview_to_analysis_works_test()` - Complete workflow succeeds
- `user_journey_interview_to_quality_works_test()` - Quality command works

**Error Cases:**
- `invalid_interview_input_returns_exit_3_test()` - Invalid input returns exit 3
- `spec_without_top_level_field_fails_validation_test()` - Missing spec field fails validation

**Test File:** `test/intent/cli_ergonomics/spec_generation_test.gleam`

### intent-cli-l9yr: CRITICAL: Fix exit codes - invalid commands return 0
- **Type:** bug
- **Priority:** 0 (Critical)
- **Labels:** ergonomics,critical,exit-codes,error-handling
- **Status:** open
- **Dependencies:** intent-cli-72pl

**User Journey:**
```gherkin
Scenario: AI agent detects command failures programmatically
  Given I run "intent invalid-command"
  When command fails
  Then exit code should be non-zero (4 for invalid command)
  And I can check exit code with $?
  And my script can reliably detect failures
```

**Acceptance Criteria (ATDD):**
- `invalid_command_returns_exit_4_test()` - Invalid command returns exit 4
- `command_with_missing_args_returns_exit_4_test()` - Missing args returns exit 4
- `command_with_invalid_flag_returns_exit_4_test()` - Invalid flag returns exit 4
- `ai_agent_can_detect_failures_via_exit_codes_test()` - Exit codes are reliable

**Error Cases:**
- Error case matrix for all exit code scenarios:
  - Invalid command → exit 4
  - Missing args → exit 4
  - Invalid flag → exit 4
  - File not found → exit 3
  - Validation error → exit 3
  - Usage error → exit 4

**Test File:** `test/intent/cli_ergonomics/exit_codes_test.gleam`

### intent-cli-izw2: CRITICAL: Fix schema introspection - manual drift from CLI
- **Type:** bug
- **Priority:** 0 (Critical)
- **Labels:** ergonomics,critical,schema,introspection
- **Status:** open
- **Dependencies:** intent-cli-72pl

**User Journey:**
```gherkin
Scenario: AI agent discovers command capabilities programmatically
  Given I run "intent ai schema --command=lint --type=input"
  When I inspect the schema
  Then all documented flags should be valid
  And flags that don't exist should not be documented
```

**Acceptance Criteria (ATDD):**
- `schema_introspection_matches_cli_implementation_test()` - Schema flags match CLI
- `documented_flags_work_correctly_test()` - Documented flags actually work
- `schema_doesnt_document_nonexistent_flags_test()` - Non-existent flags not documented

**Error Cases:**
- Schema drift detection tests for all documented vs implemented flags
- Flag type validation (bool vs string)
- Missing field documentation

**Test File:** `test/intent/cli_ergonomics/schema_sync_test.gleam`

### intent-cli-1il0: CRITICAL: Add global --json flag for consistent machine output
- **Type:** feature
- **Priority:** 0 (Critical)
- **Labels:** ergonomics,critical,json-output,ai-friendly
- **Status:** open
- **Dependencies:** intent-cli-72pl

**User Journey:**
```gherkin
Scenario: AI agent requests machine-readable output consistently
  Given I run any command with "--json" flag
  When command executes
  Then output should be structured JSON
  And output should include all required fields (success, errors, metadata, next_actions)
  And exit code should indicate success/failure properly
```

**Acceptance Criteria (ATDD):**
- `json_flag_works_for_all_commands_test()` - All commands accept --json flag
- `json_output_has_required_fields_test()` - JSON has all required fields
- `json_mode_not_human_mode_test()` - JSON mode differs from human mode

**Error Cases:**
- JSON flag accepted but output not valid JSON
- JSON flag ignored on some commands
- JSON mode doesn't include metadata
- JSON mode missing required fields

**Test File:** `test/intent/cli_ergonomics/json_flag_test.gleam`

---

## Phase 3: High Priority Issues (4 beads)

### intent-cli-7byr: Add spec_path to beads output for workflow chaining
- **Type:** feature
- **Priority:** 1 (High)
- **Labels:** ergonomics,high-priority,beads-output,workflow
- **Status:** open
- **Dependencies:** intent-cli-ysa0

**User Journey:**
```gherkin
Scenario: AI agent chains beads command to analysis commands
  Given I run "intent beads <session-id> --json"
  When I extract data from response
  Then I should find a spec_path field
  And I can use that path in subsequent commands
  And workflow is complete without manual file lookup
```

**Acceptance Criteria (ATDD):**
- `beads_output_includes_spec_path_test()` - spec_path in output
- `spec_path_points_to_valid_file_test()` - spec_path is valid file
- `workflow_chaining_works_test()` - Complete workflow succeeds

**Test File:** `test/intent/cli_ergonomics/beads_output_test.gleam`

### intent-cli-apqe: Document session storage model clearly
- **Type:** task
- **Priority:** 1 (High)
- **Labels:** ergonomics,high-priority,sessions,documentation
- **Status:** open
- **Dependencies:** intent-cli-72pl

### intent-cli-tt9w: Add --dry-run mode to all commands for safe exploration
- **Type:** feature
- **Priority:** 1 (High)
- **Labels:** ergonomics,high-priority,dry-run,safe-exploration
- **Status:** open
- **Dependencies:** intent-cli-72pl

### intent-cli-9kk0: Standardize argument patterns across all commands
- **Type:** chore
- **Priority:** 1 (High)
- **Labels:** ergonomics,high-priority,argument-consistency,ux
- **Status:** open
- **Dependencies:** intent-cli-72pl

---

## Phase 4: Medium Priority Issues (4 beads)

### intent-cli-huoz: Fix inconsistent --help flag behavior
- **Type:** bug
- **Priority:** 2 (Medium)
- **Labels:** ergonomics,medium-priority,help-consistency
- **Status:** open
- **Dependencies:** intent-cli-9kk0

### intent-cli-ptal: Implement or remove ghost commands from schema
- **Type:** bug
- **Priority:** 2 (Medium)
- **Labels:** ergonomics,medium-priority,schema,missing-commands
- **Status:** open
- **Dependencies:** intent-cli-izw2

### intent-cli-qrvp: Add command introspection for AI agents
- **Type:** feature
- **Priority:** 2 (Medium)
- **Labels:** ergonomics,medium-priority,introspection,ai-friendly
- **Status:** open
- **Dependencies:** intent-cli-izw2

### intent-cli-vvs1: Clarify external tool dependencies in AGENTS.md
- **Type:** task
- **Priority:** 2 (Medium)
- **Labels:** ergonomics,medium-priority,documentation,external-tools
- **Status:** open
- **Dependencies:** intent-cli-apqe

---

## Phase 5: Documentation & Validation (2 beads)

### intent-cli-cz2h: Create comprehensive AI_ERGONOMICS_V2.md documentation
- **Type:** chore
- **Priority:** 2 (Medium)
- **Labels:** ergonomics,documentation,ergonomics-assessment
- **Status:** open
- **Dependencies:** intent-cli-ysa0,intent-cli-l9yr,intent-cli-izw2,intent-cli-1il0,intent-cli-7byr,intent-cli-apqe,intent-cli-tt9w,intent-cli-9kk0,intent-cli-huoz,intent-cli-ptal

### intent-cli-9bbp: Validate all ergonomics fixes with test suite
- **Type:** task
- **Priority:** 2 (Medium)
- **Labels:** ergonomics,validation,testing,regression-check
- **Status:** open
- **Dependencies:** intent-cli-cz2h

---

## Test Files Created

All test files follow ATDD, BDD, and error case testing patterns:

### 1. test/intent/cli_ergonomics/test_helpers.gleam
**Purpose:** Common utilities for all ergonomics tests

**Key Types:**
- `TestSession` - Command execution context
- `TestResult` - Execution outcome with validation
- `ValidationResult` - Validation results with reasons

**Key Functions:**
- `execute_intent()` - Run intent CLI command and capture output
- `validate_json_structure()` - Validate JSON has required fields
- `validate_exit_code()` - Validate exit code matches expected

### 2. test/intent/cli_ergonomics/spec_generation_test.gleam
**Tests for:** intent-cli-ysa0 (CRITICAL: Fix spec generation)

**ATDD Tests:**
- `interview_generates_valid_spec_structure_test()`
- `interview_spec_includes_required_fields_test()`

**BDD User Journey Tests:**
- `user_journey_interview_to_analysis_works_test()`
- `user_journey_interview_to_quality_works_test()`

**Error Case Tests:**
- `invalid_interview_input_returns_exit_3_test()`
- `spec_without_top_level_field_fails_validation_test()`

### 3. test/intent/cli_ergonomics/exit_codes_test.gleam
**Tests for:** intent-cli-l9yr (CRITICAL: Fix exit codes)

**ATDD Tests:**
- `invalid_command_returns_exit_4_test()`
- `command_with_missing_args_returns_exit_4_test()`
- `command_with_invalid_flag_returns_exit_4_test()`
- `successful_command_returns_exit_0_test()`

**BDD User Journey Tests:**
- `ai_agent_can_detect_failures_via_exit_codes_test()`

**Error Case Tests:**
- `exit_code_error_matrix_test()` - Comprehensive error code matrix

### 4. test/intent/cli_ergonomics/schema_sync_test.gleam
**Tests for:** intent-cli-izw2 (CRITICAL: Fix schema introspection)

**ATDD Tests:**
- `schema_introspection_matches_cli_implementation_test()`
- `documented_flags_work_correctly_test()`
- `schema_doesnt_document_nonexistent_flags_test()`

**Helper Functions:**
- `get_schema_for_command()` - Get schema for specific command
- `parse_schema()` - Parse schema JSON
- `extract_flags_from_schema()` - Extract flags from schema
- `cli_has_flag()` - Check if CLI has specific flag

### 5. test/intent/cli_ergonomics/json_flag_test.gleam
**Tests for:** intent-cli-1il0 (CRITICAL: Add global --json flag)

**ATDD Tests:**
- `json_flag_works_for_all_commands_test()` - Test all commands accept --json
- `json_output_has_required_fields_test()` - Validate JSON structure
- `json_mode_not_human_mode_test()` - Verify JSON vs human mode differ

### 6. test/intent/cli_ergonomics/beads_output_test.gleam
**Tests for:** intent-cli-7byr (Add spec_path to beads output)

**ATDD Tests:**
- `beads_output_includes_spec_path_test()` - Validate spec_path in output
- `spec_path_points_to_valid_file_test()` - Validate spec_path is valid
- `workflow_chaining_works_test()` - Test complete workflow

---

## Implementation Workflow

### Phase 1: Research (Can start now)
1. **intent-cli-vgsf** - Document ATDD/BDD patterns
   - Examine `test/integration_e2e_test.gleam`
   - Examine `test/exit_code_test.gleam`
   - Extract reusable patterns
   - Document in code or separate doc

2. **intent-cli-wwrl** - Document bead templates
   - Examine `src/intent/bead_templates.gleam`
   - Map BeadRecord to user journey needs
   - Document acceptance criteria translation
   - Update `docs/bead_quick_reference.md`

### Phase 2: Critical (Depends on Phase 1)
3. **intent-cli-ysa0** - Fix spec generation
   - Update `src/intent/interview.gleam` spec generation
   - Ensure top-level `spec` field exists
   - Populate all required sections
   - Write valid CUE structure

4. **intent-cli-l9yr** - Fix exit codes
   - Audit all command definitions in `src/intent.gleam`
   - Update error paths to use correct exit codes
   - Add validation tests

5. **intent-cli-izw2** - Fix schema introspection
   - Remove manual schema maintenance
   - Create `generate_schemas` command
   - Auto-generate from CLI structure

6. **intent-cli-1il0** - Add global --json flag
   - Add global flag in `src/intent.gleam` main()
   - Update all commands to check `--json` flag
   - Ensure `json_output` module usage
   - Add JSON mode validation tests

### Phase 3: High Priority (Depends on Phase 2)
7. **intent-cli-7byr** - Add spec_path to beads
   - Update `src/intent/bead_templates.gleam`
   - Include spec_path in JSON response
   - Add validation tests

8. **intent-cli-apqe** - Document session storage
   - Clarify storage model (database vs JSONL)
   - Update `AGENTS.md`
   - Update `README.md`

9. **intent-cli-tt9w** - Add --dry-run mode
   - Add `--dry-run` flag to all commands
   - Implement validation without side effects
   - Add tests for dry-run behavior

10. **intent-cli-9kk0** - Standardize argument patterns
   - Audit command definitions
   - Prefer flags over positional args
   - Update command help text
   - Add consistency validation

### Phase 4: Medium Priority (Depends on Phase 3)
11. **intent-cli-huoz** - Fix --help flag
    - Standardize help flag behavior
    - Ensure all commands accept `--help`
    - Update help text

12. **intent-cli-ptal** - Fix ghost commands
    - Implement or remove `compact`/`prototext`
    - Update schema documentation
    - Add validation tests

13. **intent-cli-qrvp** - Add command introspection
    - Create `intent flags <command>` command
    - Return available flags in JSON format
    - Add introspection tests

14. **intent-cli-vvs1** - Clarify external tools
    - Document bd, bv, zjj, jj in `AGENTS.md`
    - Mark as optional vs required
    - Add installation instructions

### Phase 5: Documentation (Depends on all implementation)
15. **intent-cli-cz2h** - Create AI_ERGONOMICS_V2.md
    - Document all friction points
    - Include user journey tests
    - Include ATDD test coverage matrix
    - Include error case matrices
    - Update `docs/ai_ergonomics_report.md`

16. **intent-cli-9bbp** - Validate fixes
    - Run complete test suite
    - Execute all ATDD tests
    - Execute BDD user journey tests
    - Execute error case tests
    - Ensure no regressions

---

## Dependency Graph

```
intent-cli-72pl (Epic)
├── Phase 1 (Research - no dependencies)
│   ├── intent-cli-vgsf
│   └── intent-cli-wwrl
│
├── Phase 2 (Critical - depends on epic)
│   ├── intent-cli-ysa0 (spec-fix)
│   ├── intent-cli-l9yr (exit-codes)
│   ├── intent-cli-izw2 (schema-sync)
│   └── intent-cli-1il0 (json-flag)
│
├── Phase 3 (High - depends on epic or specific critical)
│   ├── intent-cli-7byr (beads-spec-path) ──> intent-cli-ysa0
│   ├── intent-cli-apqe (session-storage-docs) ──> intent-cli-72pl
│   ├── intent-cli-tt9w (dry-run-mode) ──> intent-cli-72pl
│   └── intent-cli-9kk0 (arg-consistency) ──> intent-cli-72pl
│
├── Phase 4 (Medium - depends on specific)
│   ├── intent-cli-huoz (help-consistency) ──> intent-cli-9kk0
│   ├── intent-cli-ptal (ghost-commands) ──> intent-cli-izw2
│   ├── intent-cli-qrvp (command-introspection) ──> intent-cli-izw2
│   └── intent-cli-vvs1 (external-tools) ──> intent-cli-apqe
│
└── Phase 5 (Documentation - depends on all implementation)
    ├── intent-cli-cz2h (docs-v2) ──> all critical & high priority
    └── intent-cli-9bbp (validate-fixes) ──> intent-cli-cz2h
```

---

## Testing Strategy

### ATDD (Acceptance Test-Driven Development)
All beads include ATDD tests following the pattern:
```gleam
pub fn <feature>_test() {
  // Given: [Setup conditions]
  // When: [Execute feature]
  // Then: [Validate outcome]
}
```

### BDD (Behavior-Driven Development)
All beads include BDD user journey scenarios:
```gherkin
Scenario: <User Story>
  Given <Preconditions>
  When <Action>
  Then <Expected Outcome>
```

### Error Case Matrix
All critical and high priority beads include error case coverage:
```gleam
pub fn error_case_matrix_test() {
  let error_cases = [
    (scenario: "X", expected: Y, description: "..."),
    (scenario: "A", expected: Z, description: "..."),
  ]
  
  error_cases |> list.each(|> test_error_case)
}
```

---

## Success Criteria

All beads are considered complete when:

1. **Acceptance Criteria Met:**
   - All ATDD tests pass
   - All BDD scenarios verified
   - Error cases handled correctly

2. **Test Coverage:**
   - Unit tests exist for all critical functionality
   - Integration tests cover user journeys
   - Error cases prevent regressions

3. **Documentation Updated:**
   - Code is documented
   - AGENTS.md updated with clear instructions
   - API docs reflect changes

4. **No Regressions:**
   - All existing tests still pass
   - No new bugs introduced
   - Exit codes remain consistent

---

## Next Steps

### Immediate (Ready to Start):
1. Run `bd ready` to check for unblocked work
2. Start with Phase 1 research beads
   ```bash
   bd update intent-cli-vgsf --status in_progress
   bd update intent-cli-wwrl --status in_progress
   ```

### After Phase 1 Complete:
3. Begin Phase 2 critical beads (highest impact)
   ```bash
   bd update intent-cli-ysa0 --status in_progress
   bd update intent-cli-l9yr --status in_progress
   ```

### After Critical Beads Complete:
4. Work Phase 3 high priority beads in parallel (where dependencies allow)
5. Work Phase 4 medium priority beads

### After All Implementation Complete:
6. Create AI_ERGONOMICS_V2.md documentation
7. Run comprehensive test suite
8. Verify no regressions

---

## Notes for AI Agents

### When Working on These Beads:

1. **Always start with test file:** Write the ATDD test first, then implement the feature
2. **Follow BDD scenarios:** Each bead includes user journey tests
3. **Test error cases:** Ensure all error paths are covered
4. **Update dependencies correctly:** Use `bd update --deps` to link related work
5. **Mark status appropriately:** in_progress when starting, completed when done

### Testing Before Closing Beads:

1. Run unit tests: `gleam test`
2. Run integration tests: `./test/run_integration_tests.sh`
3. Verify exit codes
4. Check for regressions

### Session Completion:

When all implementation beads for a phase are complete:
1. Update bead status to completed with close reason
2. Move to next phase beads
3. Commit changes with `.beads/issues.jsonl`

---

## Summary

This is a **complete, work-ready decomposition** of all AI agent friction points discovered during dogfooding. Every bead includes:

- **Clear description** of the problem
- **User journey scenarios** in BDD format
- **ATDD acceptance criteria** with specific test names
- **Error case coverage** for robustness
- **Proper dependencies** linking related work
- **Test files** ready for implementation

All beads are ready to be claimed and implemented following TDD best practices.
