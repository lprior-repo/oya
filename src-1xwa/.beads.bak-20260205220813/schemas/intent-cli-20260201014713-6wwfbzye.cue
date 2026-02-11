
package validation

import "list"

// Validation schema for bead: intent-cli-20260201014713-6wwfbzye
// Title: zjj: Implement WorkspaceManager for isolated jj workspaces
//
// This schema validates that implementation is complete.
// Use: cue vet intent-cli-20260201014713-6wwfbzye.cue implementation.cue

#BeadImplementation: {
  bead_id: "intent-cli-20260201014713-6wwfbzye"
  title: "zjj: Implement WorkspaceManager for isolated jj workspaces"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "jj binary is available in PATH",
      "Repository has .jj/ directory (jj init already run)",
      "Workspace names are unique (UUID-based)",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "WorkspaceManager can create workspace",
      "Workspace path is returned for bead execution",
      "Workspace is destroyed after use",
      "No orphaned workspaces remain (verified by periodic cleanup)",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Each workspace has unique name (UUID)",
      "Workspace directory exists while in use",
      "No two beads share a workspace simultaneously",
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
      "Create workspace, verify directory exists",
      "Execute simple command in workspace",
      "Destroy workspace, verify directory removed",
      "Create 10 workspaces concurrently, verify all isolated",
    ]

    // Required error path tests
    required_error_tests: [
      "Create workspace with invalid name, verify error",
      "Destroy non-existent workspace, verify error logged but not crash",
      "jj command fails, verify error returned",
      "Workspace directory locked, verify retry or error",
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