
package validation

import "list"

// Validation schema for bead: oya-20260225124941-0abec40q
// Title: lifecycle: Add domain types with smart constructors
//
// This schema validates that implementation is complete.
// Use: cue vet oya-20260225124941-0abec40q.cue implementation.cue

#BeadImplementation: {
  bead_id: "oya-20260225124941-0abec40q"
  title: "lifecycle: Add domain types with smart constructors"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "src/lifecycle/ directory exists",
      "Cargo.toml has im, tap, serde, thiserror dependencies",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "BeadId::parse() rejects empty strings with BeadIdError::Empty",
      "BeadId::parse() rejects strings >64 chars with BeadIdError::TooLong",
      "BeadId::parse() rejects non-alphanumeric-hyphen with BeadIdError::InvalidChars",
      "Phase enum has is_terminal() method returning true only for Completed and Failed",
      "WorkspaceName::to_path() returns repo_root.join(\"workspaces\").join(self.0)",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "phase-terminal-exclusive: only Completed and Failed phases are terminal",
      "workspace-path-derived: workspace path is always ./workspaces/oya-{bead_id}",
      "bookmark-matches-bead: bookmark name always equals bead_id",
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
      "test_bead_id_parse_valid: given \"src-abc123\" when BeadId::parse() then Ok(BeadId(\"src-abc123\"))",
      "test_bead_id_normalizes_case: given \"SRC-ABC123\" when BeadId::parse() then Ok(BeadId(\"src-abc123\"))",
      "test_phase_is_terminal_completed: given Phase::Completed when is_terminal() then true",
    ]

    // Required error path tests
    required_error_tests: [
      "test_bead_id_parse_empty: given \"\" when BeadId::parse() then Err(BeadIdError::Empty)",
      "test_bead_id_parse_too_long: given 65-char string when BeadId::parse() then Err(BeadIdError::TooLong(65))",
      "test_bead_id_parse_invalid_chars: given \"src@abc#123\" when BeadId::parse() then Err(BeadIdError::InvalidChars)",
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