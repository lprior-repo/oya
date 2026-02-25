
package validation

import "list"

// Validation schema for bead: oya-20260225124941-ogvkyzxk
// Title: lifecycle: Add error taxonomy with terminal/transient classification
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260225124941-ogvkyzxk.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260225124941-ogvkyzxk"
  title: "lifecycle: Add error taxonomy with terminal/transient classification"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "task-001 types complete with FailureCategory enum",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "LifecycleError::is_terminal() returns true for BeadNotFound, ValidationFailed, MaxRetriesExceeded",
      "LifecycleError::is_terminal() returns false for JjCommandFailed, Timeout, IoError",
      "All variants have descriptive thiserror messages",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "terminal-no-retry: terminal errors never trigger automatic retry",
      "error-category-match: is_terminal() xor is_transient() always true",
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
      "test_error_is_terminal_bead_not_found: LifecycleError::BeadNotFound.is_terminal() returns true",
      "test_error_is_transient_jj_failed: LifecycleError::JjCommandFailed.is_terminal() returns false",
    ]

    // Required error path tests
    required_error_tests: [
      "test_error_classification_consistent: all variants return correct is_terminal() value",
      "test_error_category_match: is_terminal() xor is_transient() is always true",
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