
package validation

import "list"

// Validation schema for bead: oya-20260222164723-4eruuay2
// Title: contracts: define queue lock and merge-decision type contracts
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260222164723-4eruuay2.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260222164723-4eruuay2"
  title: "contracts: define queue lock and merge-decision type contracts"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Serialized queue records include id bead_id workspace priority freshness_base_rev state",
      "scheduler receives current main revision",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Selection returns Ready Blocked Stale Conflict or Merged variants",
      "invalid records produce parse errors with field context",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "priority in 1..=10",
      "freshness_base_rev is 40-hex SHA",
      "only one merging item allowed globally",
      "lock expiry is strictly greater than acquisition",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Given valid queue record when parsing then QueueItem is created",
      "Given dependencies complete when selecting then MergeDecision::Ready is returned",
      "Given fresh base revision when checking freshness then Fresh outcome is returned",
    ]

    // Required error path tests
    required_error_tests: [
      "Given non-hex revision when parsing then parse error names freshness_base_rev",
      "Given priority 0 or 11 when parsing then bounds error is returned",
      "Given lock release by non-owner then ownership error is returned",
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
//     timestamp: "2026-02-22T16:47:23Z"
//   }
// }