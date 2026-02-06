
package validation

import "list"

// Validation schema for bead: intent-cli-20260201014713-j3tktekl
// Title: sticky: Implement sticky worker assignment with soft/hard modes
//
// This schema validates that implementation is complete.
// Use: cue vet intent-cli-20260201014713-j3tktekl.cue implementation.cue

#BeadImplementation: {
  bead_id: "intent-cli-20260201014713-j3tktekl"
  title: "sticky: Implement sticky worker assignment with soft/hard modes"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "ProcessPoolActor tracks worker state",
      "SurrealDB worker_assignment table exists",
      "BeadId and WorkerId types are defined",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Worker assignments are persisted in SurrealDB",
      "Soft sticky prefers previous worker, falls back if unavailable",
      "Hard sticky waits for previous worker or times out",
      "Assignment history is queryable",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Soft sticky always assigns to a worker (fallback guaranteed)",
      "Hard sticky either assigns to previous worker or errors",
      "Assignment timestamps are accurate",
      "All operations return Result<T, Error>",
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
      "Assign bead to worker, verify stored in DB",
      "Retry bead (soft sticky), worker available, verify same worker assigned",
      "Retry bead (soft sticky), worker unavailable, verify fallback to different worker",
      "Retry bead (hard sticky), worker available, verify same worker assigned",
    ]

    // Required error path tests
    required_error_tests: [
      "Retry bead (hard sticky), worker unavailable, verify timeout error after 30s",
      "Previous worker unhealthy, verify soft sticky falls back",
      "DB write fails, verify error but execution continues",
      "Query assignment for non-existent bead, verify error",
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
//     timestamp: "2026-02-01T01:47:13Z"
//   }
// }