
package validation

import "list"

// Validation schema for bead: oya-20260225124941-ukjkxyqm
// Title: lifecycle: Add pure phase transition functions
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260225124941-ukjkxyqm.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260225124941-ukjkxyqm"
  title: "lifecycle: Add pure phase transition functions"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "task-001 types complete",
      "task-002 errors complete",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "handle_ci_failure enforces MAX_ATTEMPTS=3",
      "All transitions return Vector with at least one Log effect",
      "bookmark name always equals bead_id",
      "No mut keyword in file",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "attempt-bounded: attempt counter never exceeds MAX_ATTEMPTS",
      "bookmark-matches-bead: bookmark name derived from bead_id",
      "zero-mut-in-core: no mut keyword in core.rs",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "test_start_lifecycle_returns_claiming: start_lifecycle returns Phase::Claiming",
      "test_transition_to_isolating_creates_workspace_effect: returns JjWorkspaceAdd effect",
      "test_handle_ci_failure_retry: attempt=1 returns Ok with attempt=2",
    ]

    // Required error path tests
    required_error_tests: [
      "test_handle_ci_failure_exhausted: attempt=3 returns Err(ValidationFailed)",
      "test_core_no_mut: grep mut core.rs returns zero matches",
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