
package validation

import "list"

// Validation schema for bead: oya-20260428190408-ydbtk1o4
// Title: slice-03: Create Finding on gate failure
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260428190408-ydbtk1o4.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260428190408-ydbtk1o4"
  title: "slice-03: Create Finding on gate failure"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "All earlier slice prerequisites for this command path are green.",
      "The worktree is clean or this bead explicitly owns its mutation scope.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Gate failure returns a finding id.",
      "moon run :ci remains green after the bead.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Oya uses Git-only VCS flow for branch and PR delivery.",
      "Failures are typed and sanitized before output or persistence.",
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
      "Given prerequisites are met, when oya verify --bead-id demo runs, then Gate failure returns a finding id.",
      "Given the bead completes, when moon run :ci runs, then CI remains green.",
    ]

    // Required error path tests
    required_error_tests: [
      "Given a prerequisite is missing, when the demo path runs, then a typed failure is emitted instead of a panic.",
      "Given evidence or command output contains sensitive details, when output is persisted or displayed, then secrets are redacted.",
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
//     timestamp: "2026-04-28T19:04:08Z"
//   }
// }