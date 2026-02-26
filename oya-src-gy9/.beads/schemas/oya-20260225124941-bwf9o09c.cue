
package validation

import "list"

// Validation schema for bead: oya-20260225124941-bwf9o09c
// Title: lifecycle: Add Restate workflow handler with saga compensation
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260225124941-bwf9o09c.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260225124941-bwf9o09c"
  title: "lifecycle: Add Restate workflow handler with saga compensation"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "All types, errors, core, and interpreters complete",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "run() executes full lifecycle",
      "On TerminalError compensations run in reverse",
      "Phase state persisted in Restate",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "saga-compensation-ordered: compensations run in reverse order of registration",
      "effect-atomic-journal: each effect in separate ctx.run()",
      "terminal-no-retry: TerminalError never triggers retry",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "test_full_lifecycle_succeeds: valid bead_id and clean repo returns LifecycleResult with pr_number",
      "test_get_status_returns_state: workflow in progress returns LifecycleState with current phase",
    ]

    // Required error path tests
    required_error_tests: [
      "test_bead_not_found_compensates: nonexistent bead_id returns TerminalError with workspace cleaned",
      "test_compensation_reverse_order: compensations ABC, B, C registered run C, B, A",
    ]
  }

  // Code completion
  code_complete: {
    implementation_exists: string  // Path to implementation file
    tests_exist: string  // Path to test file
    ci_passing: bool & true
    no_unwrap_calls: bool & true  // Rust/functional constraint
    no_panics: bool & true  // Rust constraint
  }

  // Completion criteria
  completion: {
    all_sections_complete: bool & true
    documentation_updated: bool
    beads_closed: bool
    timestamp: string  // ISO8601 completion timestamp
  }
}

// Example implementation proof - create this file to validate completion:
//
// implementation.cue:
// package validation
//
// implementation: #BeadImplementation & {
//   contracts_verified: {
//     preconditions_checked: true
//     postconditions_verified: true
//     invariants_maintained: true
//     precondition_checks: [/* documented checks */]
//     postcondition_checks: [/* documented verifications */]
//     invariant_checks: [/* documented invariants */]
//   }
//   tests_passing: {
//     all_tests_pass: true
//     happy_path_tests: ["test_version_flag_works", "test_version_format", "test_exit_code_zero"]
//     error_path_tests: ["test_invalid_flag_errors", "test_no_flags_normal_behavior"]
//   }
//   code_complete: {
//     implementation_exists: "src/main.rs"
//     tests_exist: "tests/cli_test.rs"
//     ci_passing: true
//     no_unwrap_calls: true
//     no_panics: true
//   }
//   completion: {
//     all_sections_complete: true
//     documentation_updated: true
//     beads_closed: false
//     timestamp: "2026-02-25T12:49:41Z"
//   }
// }