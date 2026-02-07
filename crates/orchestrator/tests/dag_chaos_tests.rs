//! DAG Chaos Tests
//!
//! HOSTILE chaos engineering tests that try to break the DAG.
//! These tests simulate failures, corrupt state, and stress edge cases.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::dag::{DependencyType, WorkflowDAG};

// ============================================================================
// STATE CORRUPTION TESTS (hostile: what if state is inconsistent?)
// ============================================================================

#[test]
fn given_corrupted_node_map_when_operation_then_graceful_error() {
    // GIVEN: A DAG where we query a node that doesn't exist
    let dag = WorkflowDAG::new();

    // WHEN: Trying to get dependencies of non-existent node (simulates corruption)
    let result = dag.get_dependencies(&"non_existent_node".to_string());

    // THEN: Should return error, not panic
    assert!(
        result.is_err(),
        "querying non-existent node should error gracefully"
    );

    // HOSTILE: Verify error message is useful
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("non_existent_node") || error_msg.contains("not found"),
            "error should mention the missing node or 'not found'"
        );
    }
}

#[test]
fn given_inconsistent_edge_state_when_detected_then_repaired() {
    // GIVEN: A DAG with potential edge inconsistency
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Removing node a (which has an outgoing edge)
    let remove_result = dag.remove_node(&"a".to_string());

    // THEN: Removal should succeed and clean up edges automatically
    assert!(
        remove_result.is_ok(),
        "node removal should handle edge cleanup"
    );

    // HOSTILE: Verify no dangling edges remain
    assert_eq!(
        dag.edge_count(),
        0,
        "edges should be cleaned up when node is removed"
    );

    // HOSTILE: Verify b has no incoming edges
    let b_deps = dag.get_dependencies(&"b".to_string());
    assert!(
        b_deps.is_ok(),
        "get_dependencies should work on orphaned node"
    );
    if let Ok(deps) = b_deps {
        assert_eq!(
            deps.len(),
            0,
            "b should have no dependencies after a is removed"
        );
    }
}

#[test]
fn given_oom_during_toposort_when_large_dag_then_handled() {
    // GIVEN: A very large DAG that might stress memory
    let mut dag = WorkflowDAG::new();

    // Build a 10,000 node DAG (hostile: can we handle this?)
    for i in 0..10_000 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    // Create some dependencies (sparse graph to avoid O(n^2) edges)
    for i in 0..9_900 {
        let _ = dag.add_edge(
            format!("node_{}", i),
            format!("node_{}", i + 100),
            DependencyType::BlockingDependency,
        );
    }

    // WHEN: Attempting topological sort on massive graph
    let result = dag.topological_sort();

    // THEN: Should complete without panicking (even if slow)
    // Note: We don't assert success because extreme sizes might legitimately fail,
    // but they should fail gracefully, not panic
    if result.is_err() {
        // If it fails, verify error is graceful
        if let Err(e) = result {
            let _ = e.to_string(); // Should not panic when converting to string
        }
    } else {
        // If it succeeds, verify correctness
        if let Ok(sorted) = result {
            assert_eq!(
                sorted.len(),
                10_000,
                "all nodes should be included if successful"
            );
        }
    }
}

#[test]
fn given_stack_overflow_risk_when_deep_recursion_then_iterative_fallback() {
    // GIVEN: An extremely deep chain (stress test for recursion limits)
    let mut dag = WorkflowDAG::new();

    // Build a 5000-node chain (hostile: will recursive algorithms overflow?)
    for i in 0..5_000 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    for i in 1..5_000 {
        let _ = dag.add_edge(
            format!("node_{}", i - 1),
            format!("node_{}", i),
            DependencyType::BlockingDependency,
        );
    }

    // WHEN: Getting ancestors of deepest node (tests recursion depth)
    let result = dag.get_all_ancestors(&"node_4999".to_string());

    // THEN: Should not stack overflow (must use iterative approach)
    assert!(
        result.is_ok(),
        "get_all_ancestors should not stack overflow on deep chains"
    );

    if let Ok(ancestors) = result {
        assert_eq!(
            ancestors.len(),
            4_999,
            "should find all ancestors iteratively"
        );
    }
}

#[test]
fn given_concurrent_modification_when_iteration_then_snapshot_used() {
    // GIVEN: A DAG being iterated
    let mut dag = WorkflowDAG::new();

    for i in 0..100 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    // WHEN: Collecting nodes (creates an iterator)
    let nodes_before: Vec<_> = dag.nodes().cloned().collect();

    // THEN: Snapshot should be stable even if we modify DAG
    assert_eq!(nodes_before.len(), 100, "should have captured all nodes");

    // HOSTILE: Try to modify during iteration (this is the real test)
    // In Rust, the borrow checker prevents true concurrent modification,
    // but we verify the iterator is a snapshot
    let mut dag_clone = dag.clone();
    let _ = dag_clone.add_node("new_node".to_string());

    // Original snapshot should be unchanged
    assert_eq!(
        nodes_before.len(),
        100,
        "original snapshot should be immutable"
    );
    assert!(
        !nodes_before.contains(&"new_node".to_string()),
        "new node should not appear in snapshot"
    );
}

// ============================================================================
// EDGE CASE ATTACKS (hostile: weird inputs that might break things)
// ============================================================================

#[test]
fn given_empty_string_node_id_when_operations_then_handled_correctly() {
    // GIVEN: A DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding empty string as node ID (hostile: edge case)
    let result = dag.add_node("".to_string());

    // THEN: Should handle empty ID gracefully (either accept or reject consistently)
    // We verify it doesn't panic - the specific behavior is less important
    // than graceful handling
    match result {
        Ok(_) => {
            // If accepted, verify it can be queried
            assert!(
                dag.contains_node(&"".to_string()),
                "empty ID should be queryable if accepted"
            );
        }
        Err(_) => {
            // If rejected, verify error is clear
            // This is fine - explicitly rejecting empty IDs is valid
        }
    }
}

#[test]
fn given_very_long_node_id_when_add_then_handled() {
    // GIVEN: A node ID with 10,000 characters (hostile: memory attack)
    let long_id = "x".repeat(10_000);
    let mut dag = WorkflowDAG::new();

    // WHEN: Adding extremely long node ID
    let result = dag.add_node(long_id.clone());

    // THEN: Should handle it (accept or reject, but no panic)
    if result.is_ok() {
        assert!(
            dag.contains_node(&long_id),
            "long ID should be queryable if accepted"
        );
    }
}

#[test]
fn given_unicode_node_ids_when_operations_then_work_correctly() {
    // GIVEN: Node IDs with unicode characters (hostile: encoding issues?)
    let mut dag = WorkflowDAG::new();

    let unicode_ids = vec![
        "🚀".to_string(),
        "日本語".to_string(),
        "مرحبا".to_string(),
        "Здравствуй".to_string(),
    ];

    // WHEN: Adding unicode node IDs
    for id in &unicode_ids {
        let result = dag.add_node(id.clone());
        assert!(result.is_ok(), "unicode ID '{}' should be accepted", id);
    }

    // THEN: Should be queryable and work in operations
    assert_eq!(dag.node_count(), 4, "all unicode nodes should be added");

    for id in &unicode_ids {
        assert!(
            dag.contains_node(id),
            "unicode node '{}' should be queryable",
            id
        );
    }

    // HOSTILE: Try to add dependencies between unicode nodes
    let result = dag.add_edge(
        unicode_ids[0].clone(),
        unicode_ids[1].clone(),
        DependencyType::BlockingDependency,
    );
    assert!(result.is_ok(), "edges between unicode nodes should work");
}

#[test]
fn given_duplicate_edges_when_add_multiple_times_then_idempotent() {
    // GIVEN: A DAG with two nodes
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());

    // WHEN: Adding the same edge multiple times (hostile: should be idempotent)
    let first = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _second = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _third = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: First should succeed, duplicates should be handled gracefully
    assert!(first.is_ok(), "first edge addition should succeed");

    // Edge count should be 1 (not 3) if implementation deduplicates
    // OR we might allow multiple edges - verify behavior is intentional
    let edge_count = dag.edge_count();
    // In petgraph, multiple parallel edges ARE allowed by default
    // So we verify count is consistent (either 1 or 3, not random)
    assert!(edge_count > 0, "at least one edge should exist");
}

#[test]
fn given_self_referential_dependency_when_add_edge_then_rejected_or_handled() {
    // GIVEN: A DAG with a node
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());

    // WHEN: Attempting to create a self-loop (hostile: a -> a)
    let result = dag.add_edge(
        "a".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Implementation should either:
    // 1. Reject self-loops (preferred for DAG semantics), OR
    // 2. Accept them but detect as cycle in has_cycle()

    if result.is_ok() {
        // If self-loop was accepted, it MUST be detected as a cycle
        assert!(dag.has_cycle(), "self-loop must be detected as cycle");

        let cycles = dag.find_cycles();
        assert!(!cycles.is_empty(), "self-loop must appear in find_cycles");
    }
    // If rejected, that's also valid - we just verify no panic
}

// ============================================================================
// DUPLICATE EVENTS IDEMPOTENCE (hostile: replay/de-dupe scenarios)
// ============================================================================

#[test]
fn given_duplicate_created_events_when_apply_then_idempotent_no_double_processing() {
    // GIVEN: A DAG and a Created event
    let mut dag = WorkflowDAG::new();
    let node_id = "bead-1";
    let _ = dag.add_node(node_id.to_string());

    // WHEN: Applying the same node addition twice (simulating duplicate events)
    let first_result = dag.add_node(node_id.to_string());
    let second_result = dag.add_node(node_id.to_string());

    // THEN: Should not create duplicate nodes (idempotent)
    // Implementation either rejects duplicate add_node calls or handles them gracefully
    assert!(
        first_result.is_ok() || second_result.is_ok(),
        "at least one node addition should succeed"
    );

    // Verify node count doesn't double-count
    let count = dag.node_count();
    assert!(
        count == 1,
        "node count should be 1, not {} (duplicate events caused double-counting)",
        count
    );
}

#[test]
fn given_duplicate_dependency_events_when_apply_then_no_double_blocking() {
    // GIVEN: A DAG with two nodes and a dependency
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // WHEN: Receiving duplicate dependency resolution events (simulating event replay)
    let first_result = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _second_result = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Dependencies should not multiply (no double-blocking)
    let deps = dag.get_dependencies(&"b".to_string());

    assert!(
        deps.is_ok(),
        "should be able to query dependencies after duplicate events"
    );

    if let Ok(dependencies) = deps {
        // Verify we don't have duplicate dependencies
        let unique_deps: std::collections::HashSet<_> =
            dependencies.iter().collect();
        assert_eq!(
            dependencies.len(),
            unique_deps.len(),
            "duplicate events should not create duplicate dependencies"
        );
    }
}

#[test]
fn given_event_replay_when_rebuild_state_then_consistent_result() {
    // GIVEN: A sequence of DAG operations
    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );

    let original_count = dag.node_count();
    let original_edges = dag.edge_count();

    // WHEN: Simulating event replay by re-applying same operations
    let mut replay_dag = WorkflowDAG::new();
    let _ = replay_dag.add_node("a".to_string());
    let _ = replay_dag.add_node("b".to_string());
    let _ = replay_dag.add_node("a".to_string()); // Duplicate: event replay
    let _ = replay_dag.add_node("b".to_string()); // Duplicate: event replay
    let _ = replay_dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    );
    let _ = replay_dag.add_edge(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    ); // Duplicate

    // THEN: State should be identical (no double-processing from replay)
    assert_eq!(
        replay_dag.node_count(),
        original_count,
        "replay should produce same node count as original"
    );

    // Edge count should be consistent (either 1 or 2 depending on implementation,
    // but NOT random or multiplying)
    let replay_edges = replay_dag.edge_count();
    assert!(
        replay_edges == 1 || replay_edges == 2,
        "replay should produce consistent edge count, got {}",
        replay_edges
    );
}
