//! DAG Behavioral Tests - BDD Style
//!
//! Following BDD naming convention: given_<context>_when_<action>_then_<outcome>
//!
//! These tests document expected DAG behaviors through executable specifications.
//! Tests requiring unimplemented validation are marked with #[ignore].

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::dag::{BeadId, DependencyType, WorkflowDAG};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

// ============================================================================
// NODE MANAGEMENT (8 tests)
// ============================================================================

#[test]
fn given_empty_dag_when_add_node_then_node_exists_and_count_is_one() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();
    assert_eq!(dag.node_count(), 0, "Precondition: DAG should be empty");

    // WHEN: A node is added
    let bead_id = "bead-001".to_string();
    let result = dag.add_node(bead_id.clone());

    // THEN: Node exists and count is one
    assert!(result.is_ok(), "Adding node should succeed");
    assert_eq!(dag.node_count(), 1, "Node count should be 1");

    let nodes: Vec<&BeadId> = dag.nodes().collect();
    assert!(nodes.contains(&&bead_id), "Node should be retrievable");
}

#[test]
fn given_dag_with_node_when_remove_node_then_node_gone_and_count_decremented() {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    let bead_id = "bead-to-remove".to_string();
    let _ = dag.add_node(bead_id.clone());
    assert_eq!(dag.node_count(), 1, "Precondition: DAG should have 1 node");

    // WHEN: The node is removed
    let result = dag.remove_node(&bead_id);

    // THEN: Node is gone and count is decremented
    assert!(result.is_ok(), "Removing node should succeed");
    assert_eq!(dag.node_count(), 0, "Node count should be 0");
    let nodes: Vec<&BeadId> = dag.nodes().collect();
    assert!(!nodes.contains(&&bead_id), "Node should not exist");
}

#[test]
fn given_dag_with_edges_when_remove_node_then_all_connected_edges_removed() {
    // GIVEN: A DAG with edges connected to a node
    //   a --> b --> c
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    assert_eq!(dag.edge_count(), 2, "Precondition: DAG should have 2 edges");

    // WHEN: The middle node is removed
    let result = dag.remove_node(&"b".to_string());

    // THEN: All connected edges should be removed
    assert!(result.is_ok(), "Removing node should succeed");
    assert_eq!(dag.node_count(), 2, "Should have 2 nodes left");
    assert_eq!(dag.edge_count(), 0, "All edges to/from b should be removed");
}

#[test]
fn given_dag_with_node_when_add_duplicate_then_error_contains_node_id() {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    let bead_id = "duplicate-node".to_string();
    let first_result = dag.add_node(bead_id.clone());
    assert!(
        first_result.is_ok(),
        "Precondition: First add should succeed"
    );

    // WHEN: Adding the same node again
    let result = dag.add_node(bead_id.clone());

    // THEN: Error should be returned and contain the node ID
    assert!(result.is_err(), "Duplicate add should fail");
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.contains(&bead_id),
        "Error message '{}' should contain node ID '{}'",
        err,
        bead_id
    );
}

#[test]
fn given_empty_dag_when_remove_nonexistent_node_then_error_not_panic() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Attempting to remove a nonexistent node
    let result = dag.remove_node(&"ghost-node".to_string());

    // THEN: Should return error, not panic
    assert!(
        result.is_err(),
        "Removing nonexistent node should return error"
    );
}

#[test]
#[ignore = "add_node validation for empty string not yet implemented"]
fn given_empty_string_node_id_when_add_node_then_error_invalid_id() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding a node with empty string ID
    let result = dag.add_node("".to_string());

    // THEN: Should return error for invalid ID
    // Note: Current implementation allows empty strings - this test documents desired behavior
    assert!(result.is_err(), "Empty string ID should be rejected");
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.to_lowercase().contains("invalid"),
        "Error should indicate invalid ID"
    );
}

#[test]
#[ignore = "add_node validation for whitespace not yet implemented"]
fn given_whitespace_only_node_id_when_add_node_then_error_invalid_id() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding a node with whitespace-only ID
    let result = dag.add_node("   ".to_string());

    // THEN: Should return error for invalid ID
    // Note: Current implementation allows whitespace - this test documents desired behavior
    assert!(result.is_err(), "Whitespace-only ID should be rejected");
}

#[test]
#[ignore = "add_node validation for length not yet implemented"]
fn given_10000_char_node_id_when_add_node_then_error_id_too_long() {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding a node with very long ID
    let long_id = "a".repeat(10000);
    let result = dag.add_node(long_id);

    // THEN: Should return error for ID too long
    // Note: Current implementation allows any length - this test documents desired behavior
    assert!(result.is_err(), "Very long ID should be rejected");
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.to_lowercase().contains("long") || err.to_lowercase().contains("length"),
        "Error should indicate ID too long"
    );
}

// ============================================================================
// EDGE MANAGEMENT (6 tests)
// ============================================================================

#[test]
fn given_two_nodes_when_add_blocking_edge_then_dependency_tracked() {
    // GIVEN: A DAG with two nodes
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("source".to_string());
    let _ = dag.add_node("target".to_string());

    // WHEN: Adding a blocking edge from source to target
    let result = dag.add_edge(
        "source".to_string(),
        "target".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Edge is created and dependency is tracked
    assert!(result.is_ok(), "Adding edge should succeed");
    assert_eq!(dag.edge_count(), 1, "Edge count should be 1");

    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 1, "Should have one edge");
    assert_eq!(edges[0].0, "source", "Edge source should match");
    assert_eq!(edges[0].1, "target", "Edge target should match");
    assert_eq!(
        *edges[0].2,
        DependencyType::BlockingDependency,
        "Edge type should be BlockingDependency"
    );
}

#[test]
fn given_edge_exists_when_remove_edge_then_dependency_no_longer_tracked() {
    // GIVEN: A DAG with an edge
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    assert_eq!(dag.edge_count(), 1, "Precondition: Should have 1 edge");

    // WHEN: Removing the edge
    let result = dag.remove_edge(&"a".to_string(), &"b".to_string());

    // THEN: Edge is removed and dependency no longer tracked
    assert!(result.is_ok(), "Removing edge should succeed");
    assert_eq!(dag.edge_count(), 0, "Edge count should be 0");
}

#[test]
fn given_missing_source_node_when_add_edge_then_error_source_not_found() {
    // GIVEN: A DAG with only target node
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("target".to_string());

    // WHEN: Adding edge from nonexistent source
    let result = dag.add_edge(
        "nonexistent-source".to_string(),
        "target".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Should return error indicating source not found
    assert!(
        result.is_err(),
        "Adding edge with missing source should fail"
    );
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.contains("nonexistent-source") || err.to_lowercase().contains("not found"),
        "Error '{}' should reference the missing source",
        err
    );
}

#[test]
fn given_missing_target_node_when_add_edge_then_error_target_not_found() {
    // GIVEN: A DAG with only source node
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("source".to_string());

    // WHEN: Adding edge to nonexistent target
    let result = dag.add_edge(
        "source".to_string(),
        "nonexistent-target".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Should return error indicating target not found
    assert!(
        result.is_err(),
        "Adding edge with missing target should fail"
    );
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.contains("nonexistent-target") || err.to_lowercase().contains("not found"),
        "Error '{}' should reference the missing target",
        err
    );
}

#[test]
#[ignore = "self-loop validation not yet implemented in add_edge"]
fn given_same_source_and_target_when_add_edge_then_error_self_loop_forbidden() {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("self-node".to_string());

    // WHEN: Adding an edge from node to itself
    let result = dag.add_edge(
        "self-node".to_string(),
        "self-node".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Should return error indicating self-loop forbidden
    // Note: Current implementation allows self-loops - this documents desired behavior
    assert!(result.is_err(), "Self-loop should be rejected");
    let err = result.err().map(|e| e.to_string()).unwrap_or_default();
    assert!(
        err.to_lowercase().contains("self") || err.to_lowercase().contains("loop"),
        "Error should indicate self-loop forbidden"
    );
}

#[test]
#[ignore = "duplicate edge validation not yet implemented"]
fn given_existing_edge_when_add_duplicate_then_error_edge_exists() {
    // GIVEN: A DAG with an existing edge
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Adding the same edge again
    let result = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Should return error indicating edge already exists
    // Note: Current implementation allows duplicate edges - this documents desired behavior
    assert!(result.is_err(), "Duplicate edge should be rejected");
}

// ============================================================================
// DEPENDENCY BEHAVIORS (6 tests)
// ============================================================================

#[test]
fn given_node_with_no_incoming_edges_when_get_dependencies_then_empty_vec() {
    // GIVEN: A DAG with a node that has no incoming edges
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("root".to_string());
    let _ = dag.add_node("child".to_string());
    let _ = dag.add_edge(
        "root".to_string(),
        "child".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Getting dependencies of the root node (no incoming edges)
    let deps = dag.get_dependencies(&"root".to_string());

    // THEN: Should return empty vec
    assert!(deps.is_ok(), "get_dependencies should succeed");
    assert!(
        deps.map(|d| d.is_empty()).unwrap_or(false),
        "Root node should have no dependencies"
    );
}

#[test]
fn given_node_with_one_blocking_dep_when_get_dependencies_then_returns_that_dep() {
    // GIVEN: A DAG where B depends on A
    //   A --> B
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Getting dependencies of B
    let deps = dag.get_dependencies(&"b".to_string());

    // THEN: Should return A
    assert!(deps.is_ok(), "get_dependencies should succeed");
    let deps = deps.unwrap_or_default();
    assert_eq!(deps.len(), 1, "B should have one dependency");
    assert!(
        deps.contains(&"a".to_string()),
        "B's dependency should be A"
    );
}

#[test]
fn given_node_with_five_blocking_deps_when_get_dependencies_then_returns_all_five() {
    // GIVEN: A DAG where F depends on A, B, C, D, E
    //   A -\
    //   B --\
    //   C ----> F
    //   D --/
    //   E -/
    let mut dag = WorkflowDAG::new();
    let deps_names = ["a", "b", "c", "d", "e"];
    for name in &deps_names {
        let _ = dag.add_node(name.to_string());
    }
    let _ = dag.add_node("f".to_string());

    for name in &deps_names {
        let _ = dag.add_edge(
            name.to_string(),
            "f".to_string(),
            DependencyType::BlockingDependency,
        );
    }

    // WHEN: Getting dependencies of F
    let deps = dag.get_dependencies(&"f".to_string());

    // THEN: Should return all five dependencies
    assert!(deps.is_ok(), "get_dependencies should succeed");
    let deps = deps.unwrap_or_default();
    assert_eq!(deps.len(), 5, "F should have 5 dependencies");
    for name in &deps_names {
        assert!(
            deps.contains(&name.to_string()),
            "F should depend on {}",
            name
        );
    }
}

#[test]
fn given_nonexistent_node_when_get_dependencies_then_error_not_panic() {
    // GIVEN: An empty DAG
    let dag = WorkflowDAG::new();

    // WHEN: Getting dependencies of nonexistent node
    let result = dag.get_dependencies(&"ghost".to_string());

    // THEN: Should return error, not panic
    assert!(
        result.is_err(),
        "Getting dependencies of nonexistent node should error"
    );
}

#[test]
fn given_chain_a_to_b_to_c_when_get_all_ancestors_of_c_then_returns_a_and_b() {
    // GIVEN: A linear chain A --> B --> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Getting all ancestors of C
    let ancestors = dag.get_all_ancestors(&"c".to_string());

    // THEN: Should return A and B
    assert!(ancestors.is_ok(), "get_all_ancestors should succeed");
    let ancestors = ancestors.unwrap_or_default();
    assert_eq!(ancestors.len(), 2, "C should have 2 ancestors");
    assert!(ancestors.contains("a"), "A should be an ancestor");
    assert!(ancestors.contains("b"), "B should be an ancestor");
}

#[test]
fn given_diamond_a_to_bc_to_d_when_get_all_ancestors_of_d_then_no_duplicates() {
    // GIVEN: A diamond pattern
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Getting all ancestors of D
    let ancestors = dag.get_all_ancestors(&"d".to_string());

    // THEN: Should return A, B, C with no duplicates (HashSet guarantees this)
    assert!(ancestors.is_ok(), "get_all_ancestors should succeed");
    let ancestors = ancestors.unwrap_or_default();
    assert_eq!(
        ancestors.len(),
        3,
        "D should have exactly 3 unique ancestors"
    );
    assert!(ancestors.contains("a"), "A should be an ancestor");
    assert!(ancestors.contains("b"), "B should be an ancestor");
    assert!(ancestors.contains("c"), "C should be an ancestor");
}

// ============================================================================
// READY DETECTION (6 tests)
// ============================================================================

#[test]
fn given_dag_with_roots_when_nothing_completed_then_only_roots_are_ready() {
    // GIVEN: A DAG with root nodes and children
    //   A --> C
    //   B --> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Getting ready nodes with no completions
    let completed: HashSet<BeadId> = HashSet::new();
    let ready = dag.get_ready_nodes(&completed);

    // THEN: Only root nodes (A and B) should be ready
    assert_eq!(ready.len(), 2, "Only 2 root nodes should be ready");
    assert!(ready.contains(&"a".to_string()), "A should be ready");
    assert!(ready.contains(&"b".to_string()), "B should be ready");
    assert!(!ready.contains(&"c".to_string()), "C should not be ready");
}

#[test]
fn given_diamond_when_root_completes_then_children_become_ready_parallel() {
    // GIVEN: A diamond DAG: A -> B, C -> D
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut dag = WorkflowDAG::new();
    let a = "a".to_string();
    let b = "b".to_string();
    let c = "c".to_string();
    let d = "d".to_string();

    dag.add_node(a.clone()).ok();
    dag.add_node(b.clone()).ok();
    dag.add_node(c.clone()).ok();
    dag.add_node(d.clone()).ok();

    dag.add_edge(a.clone(), b.clone(), DependencyType::BlockingDependency)
        .ok();
    dag.add_edge(a.clone(), c.clone(), DependencyType::BlockingDependency)
        .ok();
    dag.add_edge(b.clone(), d.clone(), DependencyType::BlockingDependency)
        .ok();
    dag.add_edge(c.clone(), d.clone(), DependencyType::BlockingDependency)
        .ok();

    // WHEN: Root bead A completes
    let mut completed: HashSet<BeadId> = HashSet::new();
    completed.insert(a.clone());
    let ready = dag.get_ready_nodes(&completed);

    // THEN: Both B and C become ready (parallel execution)
    assert_eq!(ready.len(), 2, "Both B and C should be ready in parallel");
    assert!(ready.contains(&b), "B should be ready after A completes");
    assert!(ready.contains(&c), "C should be ready after A completes");
    assert!(
        !ready.contains(&a),
        "A should not be ready (already completed)"
    );
    assert!(
        !ready.contains(&d),
        "D should not be ready (waiting for B and C)"
    );
}

#[test]
fn given_linear_chain_when_first_completed_then_second_becomes_ready() {
    // GIVEN: A linear chain A --> B --> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: A is completed
    let mut completed: HashSet<BeadId> = HashSet::new();
    completed.insert("a".to_string());
    let ready = dag.get_ready_nodes(&completed);

    // THEN: B should become ready
    assert!(
        ready.contains(&"b".to_string()),
        "B should be ready after A completes"
    );
    assert!(
        !ready.contains(&"a".to_string()),
        "A should not be ready (already completed)"
    );
    assert!(
        !ready.contains(&"c".to_string()),
        "C should not be ready (B not done)"
    );
}

#[test]
fn given_diamond_when_left_branch_only_completes_then_join_not_ready() {
    // GIVEN: A diamond pattern
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Only A and B are completed (left branch)
    let mut completed: HashSet<BeadId> = HashSet::new();
    completed.insert("a".to_string());
    completed.insert("b".to_string());
    let ready = dag.get_ready_nodes(&completed);

    // THEN: D should NOT be ready (C not done)
    assert!(ready.contains(&"c".to_string()), "C should be ready");
    assert!(
        !ready.contains(&"d".to_string()),
        "D should NOT be ready (C not done)"
    );
}

#[test]
fn given_diamond_when_both_branches_complete_then_join_becomes_ready() {
    // GIVEN: A diamond pattern (same as above)
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Both A, B, and C are completed
    let mut completed: HashSet<BeadId> = HashSet::new();
    completed.insert("a".to_string());
    completed.insert("b".to_string());
    completed.insert("c".to_string());
    let ready = dag.get_ready_nodes(&completed);

    // THEN: D should become ready
    assert!(
        ready.contains(&"d".to_string()),
        "D should be ready after both branches complete"
    );
}

#[test]
fn given_preferred_order_edge_when_dep_incomplete_then_node_still_ready() {
    // GIVEN: A DAG with preferred order edge (soft dependency)
    //   A --[preferred]--> B
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::PreferredOrder,
    );

    // WHEN: Nothing is completed
    let completed: HashSet<BeadId> = HashSet::new();
    let ready = dag.get_ready_nodes(&completed);

    // THEN: B should still be ready (PreferredOrder doesn't block)
    assert!(ready.contains(&"a".to_string()), "A should be ready");
    assert!(
        ready.contains(&"b".to_string()),
        "B should be ready (PreferredOrder doesn't block)"
    );
}

#[test]
fn given_mix_of_blocking_and_preferred_when_only_blocking_done_then_ready() {
    // GIVEN: A DAG with mixed edge types
    //   A --[blocking]--> C
    //   B --[preferred]--> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::PreferredOrder,
    );

    // WHEN: Only blocking dependency (A) is completed
    let mut completed: HashSet<BeadId> = HashSet::new();
    completed.insert("a".to_string());
    let ready = dag.get_ready_nodes(&completed);

    // THEN: C should be ready (blocking done, preferred doesn't matter)
    assert!(
        ready.contains(&"c".to_string()),
        "C should be ready when blocking dep done"
    );
}

// ============================================================================
// CYCLE DETECTION (4 tests)
// ============================================================================

#[test]
fn given_acyclic_dag_when_check_cycle_then_returns_false() {
    // GIVEN: An acyclic DAG
    //   A --> B --> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Checking for cycles
    let has_cycle = dag.has_cycle();

    // THEN: Should return false
    assert!(!has_cycle, "Acyclic DAG should not have cycles");
}

#[test]
fn given_direct_cycle_a_to_b_to_a_when_check_cycle_then_returns_true() {
    // GIVEN: A DAG with a direct cycle A --> B --> A
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Checking for cycles
    let has_cycle = dag.has_cycle();

    // THEN: Should return true
    assert!(has_cycle, "DAG with A->B->A should have cycle");
}

#[test]
fn given_self_loop_when_check_cycle_then_returns_true() {
    // GIVEN: A DAG with a self-loop A --> A
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    // Note: add_edge currently allows self-loops
    let _ = dag.add_edge(
        "a".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Checking for cycles
    let has_cycle = dag.has_cycle();

    // THEN: Should return true (self-loop is a cycle)
    assert!(has_cycle, "Self-loop should be detected as cycle");
}

#[test]
fn given_complex_cycle_when_find_cycles_then_returns_participating_nodes() {
    // GIVEN: A DAG with a complex cycle
    //   A --> B --> C --> D --> B (cycle: B -> C -> D -> B)
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "d".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Finding cycle nodes
    let cycles = dag.find_cycles();

    // THEN: Should return B, C, D (participating in cycle)
    assert!(!cycles.is_empty(), "Should find at least one cycle");

    // Flatten all cycles to check participating nodes
    let cycle_nodes: HashSet<BeadId> = cycles.into_iter().flatten().collect();
    assert!(cycle_nodes.contains("b"), "B should be in cycle");
    assert!(cycle_nodes.contains("c"), "C should be in cycle");
    assert!(cycle_nodes.contains("d"), "D should be in cycle");
    // A is not in the cycle itself
    assert!(!cycle_nodes.contains("a"), "A should NOT be in cycle");
}

// ============================================================================
// TOPOLOGICAL ORDERING (3 tests)
// ============================================================================

#[test]
fn given_linear_chain_when_topological_sort_then_order_is_preserved() {
    // GIVEN: A linear chain A --> B --> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Performing topological sort
    let sorted = dag.topological_sort();

    // THEN: Order should be A before B before C
    assert!(sorted.is_ok(), "topological_sort should succeed");
    let sorted = sorted.unwrap_or_default();

    let pos_a = sorted.iter().position(|x| x == "a");
    let pos_b = sorted.iter().position(|x| x == "b");
    let pos_c = sorted.iter().position(|x| x == "c");

    assert!(pos_a.is_some() && pos_b.is_some() && pos_c.is_some());
    assert!(pos_a < pos_b, "A should come before B");
    assert!(pos_b < pos_c, "B should come before C");
}

#[test]
fn given_kahn_sort_on_linear_chain_then_same_order() {
    // GIVEN: A linear chain A --> B --> C
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Performing Kahn's topological sort
    let sorted = dag.topological_sort_kahn();

    // THEN: Order should be A, B, C
    assert!(sorted.is_ok(), "topological_sort_kahn should succeed");
    let sorted = sorted.unwrap_or_default();
    assert_eq!(sorted.len(), 3, "Should have 3 nodes");
    assert_eq!(sorted[0], "a", "First should be A");
    assert_eq!(sorted[1], "b", "Second should be B");
    assert_eq!(sorted[2], "c", "Third should be C");
}

#[test]
fn given_dag_with_cycle_when_topological_sort_then_error() {
    // GIVEN: A DAG with a cycle
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Performing topological sort
    let sorted = dag.topological_sort();

    // THEN: Should return error
    assert!(
        sorted.is_err(),
        "topological_sort on cyclic graph should error"
    );
}

// ============================================================================
// CRITICAL PATH (2 tests)
// ============================================================================

#[test]
fn given_parallel_paths_when_critical_path_then_returns_longest() {
    // GIVEN: A DAG with parallel paths of different lengths
    //   A (1s) --> B (5s)
    //   A (1s) --> C (2s)
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "a".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    let mut weights = HashMap::new();
    weights.insert("a".to_string(), Duration::from_secs(1));
    weights.insert("b".to_string(), Duration::from_secs(5));
    weights.insert("c".to_string(), Duration::from_secs(2));

    // WHEN: Computing critical path
    let critical = dag.critical_path(&weights);

    // THEN: Should return A -> B (total 6s) not A -> C (total 3s)
    assert!(critical.is_ok(), "critical_path should succeed");
    let critical = critical.unwrap_or_default();
    assert!(
        critical.contains(&"a".to_string()),
        "A should be on critical path"
    );
    assert!(
        critical.contains(&"b".to_string()),
        "B should be on critical path"
    );
}

#[test]
fn given_empty_dag_when_critical_path_then_empty_vec() {
    // GIVEN: An empty DAG
    let dag = WorkflowDAG::new();
    let weights = HashMap::new();

    // WHEN: Computing critical path
    let critical = dag.critical_path(&weights);

    // THEN: Should return empty vec
    assert!(
        critical.is_ok(),
        "critical_path should succeed on empty DAG"
    );
    assert!(
        critical.map(|p| p.is_empty()).unwrap_or(false),
        "Critical path of empty DAG should be empty"
    );
}

// ============================================================================
// SUBGRAPH OPERATIONS (2 tests)
// ============================================================================

#[test]
fn given_dag_when_extract_subgraph_then_only_specified_nodes_included() {
    // GIVEN: A DAG A --> B --> C --> D
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Extracting subgraph with only A and B
    let subgraph = dag.subgraph(&["a".to_string(), "b".to_string()]);

    // THEN: Should contain only A and B with edge between them
    assert!(subgraph.is_ok(), "subgraph should succeed");
    let subgraph = subgraph.unwrap_or_default();
    assert_eq!(subgraph.node_count(), 2, "Should have 2 nodes");
    assert_eq!(subgraph.edge_count(), 1, "Should have 1 edge");
    assert!(subgraph.contains_node(&"a".to_string()), "Should contain A");
    assert!(subgraph.contains_node(&"b".to_string()), "Should contain B");
    assert!(
        !subgraph.contains_node(&"c".to_string()),
        "Should not contain C"
    );
}

#[test]
fn given_node_when_induced_subgraph_then_contains_ancestors_and_descendants() {
    // GIVEN: A DAG A --> B --> C --> D
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_node("e".to_string()); // disconnected node
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Getting induced subgraph around B
    let induced = dag.induced_subgraph(&"b".to_string());

    // THEN: Should contain A (ancestor), B, C, D (descendants) but not E
    assert!(induced.is_ok(), "induced_subgraph should succeed");
    let induced = induced.unwrap_or_default();
    assert_eq!(induced.node_count(), 4, "Should have 4 nodes (a,b,c,d)");
    assert!(induced.contains_node(&"a".to_string()), "Should contain A");
    assert!(induced.contains_node(&"b".to_string()), "Should contain B");
    assert!(induced.contains_node(&"c".to_string()), "Should contain C");
    assert!(induced.contains_node(&"d".to_string()), "Should contain D");
    assert!(
        !induced.contains_node(&"e".to_string()),
        "Should not contain E (disconnected)"
    );
}

// ============================================================================
// CONNECTIVITY (2 tests)
// ============================================================================

#[test]
fn given_connected_dag_when_check_connected_then_returns_true() {
    // GIVEN: A connected DAG
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = dag.add_edge(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Checking connectivity
    let is_connected = dag.is_connected();

    // THEN: Should return true
    assert!(is_connected, "Connected DAG should return true");
}

#[test]
fn given_disconnected_dag_when_check_connected_then_returns_false() {
    // GIVEN: A disconnected DAG (two separate components)
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string()); // disconnected from a-b
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    // c has no edges

    // WHEN: Checking connectivity
    let is_connected = dag.is_connected();

    // THEN: Should return false
    assert!(!is_connected, "Disconnected DAG should return false");
}

// ============================================================================
// SELF-LOOP VALIDATION (1 test)
// ============================================================================

#[test]
fn given_dag_with_no_self_loops_when_validate_then_ok() {
    // GIVEN: A DAG without self-loops
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Validating no self-loops
    let result = dag.validate_no_self_loops();

    // THEN: Should return Ok
    assert!(
        result.is_ok(),
        "DAG without self-loops should pass validation"
    );
}
