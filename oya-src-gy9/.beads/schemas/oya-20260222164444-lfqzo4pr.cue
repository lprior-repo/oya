
package validation

import "list"

// Validation schema for bead: oya-20260222164444-lfqzo4pr
// Title: github: add agent-driven pr creation and stack mapping
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260222164444-lfqzo4pr.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260222164444-lfqzo4pr"
  title: "github: add agent-driven pr creation and stack mapping"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "GitHub auth is available in environment",
      "Queue item has mapped branch/bookmark",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "PR URL is persisted in durable event payload",
      "Subsequent runs update existing PR rather than creating new",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "One queue item maps to one PR",
      "PR body includes bead id, run id, and test evidence",
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
      "Given done queue item with no PR when publish runs then PR is created and URL stored",
      "Given existing PR mapping when publish runs then PR is updated",
    ]

    // Required error path tests
    required_error_tests: [
      "Given missing gh auth when publish runs then deterministic terminal error is emitted",
      "Given gh API transient failure when publish runs then retryable handler error is returned",
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
//     timestamp: "2026-02-22T16:44:44Z"
//   }
// }