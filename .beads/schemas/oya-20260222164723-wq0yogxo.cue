
package validation

import "list"

// Validation schema for bead: oya-20260222164723-wq0yogxo
// Title: orchestrator: hard-remove zjj from landing workspace and ship-gate execution
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260222164723-wq0yogxo.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260222164723-wq0yogxo"
  title: "orchestrator: hard-remove zjj from landing workspace and ship-gate execution"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "run_id and bead_id exist",
      "moon and br binaries resolve in PATH",
      "current repo root is discoverable",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "landing telemetry contains only moon and br commands",
      "workspace lifecycle events are deterministic and zjj-free",
      "ship gate outputs do not reference zjj command names",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No subprocess invocation uses program=zjj",
      "Same stage input produces same command sequence on replay",
      "Failure category for missing workspace is terminal and explicit",
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
      "Given completed implementation stage when landing executes then commands are [moon run :ci, br close <id>, br sync --flush-only] in order",
      "Given repo-root execution when workspace metadata absent then stage succeeds and records repo-root path",
      "Given merge policy enforce when ship gate runs then only cue artifact and moon-based checks are evaluated",
    ]

    // Required error path tests
    required_error_tests: [
      "Given moon binary missing when landing executes then terminal failure is emitted and pipeline halts",
      "Given br close returns non-zero when landing executes then stage records failure and no further landing commands run",
      "Given command template accidentally includes zjj token then safety check rejects execution",
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