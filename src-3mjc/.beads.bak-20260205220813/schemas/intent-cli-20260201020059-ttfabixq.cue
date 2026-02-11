
package validation

import "list"

// Validation schema for bead: intent-cli-20260201020059-ttfabixq
// Title: web: Implement axum REST API with tower middleware
//
// This schema validates that implementation is complete.
// Use: cue vet intent-cli-20260201020059-ttfabixq.cue implementation.cue

#BeadImplementation: {
  bead_id: "intent-cli-20260201020059-ttfabixq"
  title: "web: Implement axum REST API with tower middleware"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "axum 0.7 and tower 0.5 available",
      "SchedulerActor can receive bead creation requests",
      "StateManagerActor can query bead status",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "REST API listening on :8080",
      "All 5 endpoints working (create, query, cancel, health, list)",
      "CORS configured for Tauri origin",
      "Tower middleware active (compression, tracing)",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "All endpoints return JSON",
      "All operations are idempotent where applicable",
      "Error responses follow RFC 7807 Problem Details",
      "All handlers return Result<Json<T>, Error>",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(5)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "POST /api/workflows, verify 201 Created with bead ID",
      "GET /api/beads/:id, verify 200 OK with status",
      "POST /api/beads/:id/cancel, verify 200 OK",
      "GET /api/health, verify 200 OK",
      "GET /api/workflows, verify 200 OK with list",
    ]

    // Required error path tests
    required_error_tests: [
      "GET /api/beads/nonexistent, verify 404 Not Found",
      "POST /api/workflows with invalid JSON, verify 400 Bad Request",
      "POST /api/beads/:id/cancel already completed, verify error",
      "Request without CORS headers from unknown origin, verify rejected",
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
//     timestamp: "2026-02-01T02:00:59Z"
//   }
// }