
package validation

import "list"

// Validation schema for bead: oya-20260222164723-mgkugh6l
// Title: queue: implement single-merge worker ttl locks freshness and conflict cascade
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260222164723-mgkugh6l.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260222164723-mgkugh6l"
  title: "queue: implement single-merge worker ttl locks freshness and conflict cascade"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "all queue records pass schema validation",
      "worker_id is unique",
      "freshness base revision recorded per item",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "claim transitions queued->claimed with lock ownership",
      "successful merge transitions claimed->merging->done",
      "stale item transitions through rebase and revalidation",
      "conflict decisions are appended to immutable audit log",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "max one merging item globally",
      "lock owner required for release",
      "manual child conflict override dominates propagated strategy",
      "queue ordering deterministic by priority then created_at then id",
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
      "Given two ready items different priority when polling then higher priority claimed first",
      "Given equal priority and earlier created_at when polling then FIFO tie-breaker wins",
      "Given stale item with clean rebase when freshness guard runs then item returns Ready and merges",
    ]

    // Required error path tests
    required_error_tests: [
      "Given valid lock held by worker A when worker B claims then lock_denied error returned",
      "Given rebase conflict when freshness guard runs then decision is Conflict and no merge occurs",
      "Given parent conflict propagation incompatible for child then child marked manual_required",
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