
package validation

import "list"

// Validation schema for bead: intent-cli-20260201014210-6wxrwadt
// Title: dag: Implement Tarjan's algorithm for cycle detection
//
// This schema validates that implementation is complete.
// Use: cue vet intent-cli-20260201014210-6wxrwadt.cue implementation.cue

#BeadImplementation: {
  bead_id: "intent-cli-20260201014210-6wxrwadt"
  title: "dag: Implement Tarjan's algorithm for cycle detection"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "WorkflowDAG is implemented (from task-001)",
      "petgraph provides graph traversal utilities",
      "Graph has at least 1 node to detect cycles",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "All strongly connected components are found",
      "Cycles (SCCs with >1 node) are reported",
      "Acyclic graphs return empty cycle list",
      "Algorithm runs in O(V + E) time",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Every node belongs to exactly one SCC",
      "Self-loops are detected as 1-node SCCs",
      "Cycles are reported with all participating beads",
      "Detection is deterministic",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Detect cycle in simple loop (A→B→A)",
      "Detect multiple cycles in complex DAG",
      "Detect no cycles in valid DAG",
      "Detect self-loop (A→A)",
    ]

    // Required error path tests
    required_error_tests: [
      "Run on empty graph, verify empty result",
      "Run on single-node graph, verify no cycle",
      "Run on disconnected components, verify all checked",
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