
package validation

import "list"

// Validation schema for bead: intent-cli-20260201014210-qnjb0bbj
// Title: merge-queue: Implement PR lifecycle management with bead integration
//
// This schema validates that implementation is complete.
// Use: cue vet intent-cli-20260201014210-qnjb0bbj.cue implementation.cue

#BeadImplementation: {
  bead_id: "intent-cli-20260201014210-qnjb0bbj"
  title: "merge-queue: Implement PR lifecycle management with bead integration"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "BeadWorkerActor can execute test beads (from Stream B)",
      "Git/jj operations available for rebase and merge",
      "SurrealDB table for PR tracking exists",
      "GitHub API or git commands available for PR operations",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "PR status accurately reflects current state",
      "Test beads are created for all queued PRs",
      "Successful PRs are merged automatically",
      "Failed PRs are marked and author notified",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "PR can only transition: pending→testing→merging→merged or failed",
      "Test bead completion triggers merge attempt",
      "Merge conflicts always trigger rebase + retest",
      "All operations are idempotent (safe to retry)",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Add PR to queue, verify test bead created",
      "Complete test bead successfully, verify PR merged",
      "Add 3 PRs, verify sequential testing and merging",
      "Query PR status, verify accurate state",
    ]

    // Required error path tests
    required_error_tests: [
      "Test bead fails, verify PR marked failed",
      "Merge conflict detected, verify rebase + retest",
      "Merge operation fails, verify retry with backoff",
      "Kill MergeQueueActor, verify supervisor restarts and rebuilds state",
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
//     timestamp: "2026-02-01T01:42:10Z"
//   }
// }