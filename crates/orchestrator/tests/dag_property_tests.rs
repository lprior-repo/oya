//! DAG Property-Based Tests
//!
//! Using property testing to verify DAG invariants hold across random inputs.
//! HOSTILE: Generate random graphs and verify properties that MUST always be true.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use orchestrator::dag::{BeadId, DependencyType, WorkflowDAG};
use std::collections::HashSet;

// NOTE: These tests would use proptest in production, but we'll implement
// deterministic versions that test the same properties with carefully chosen inputs

// ============================================================================
// INVARIANT TESTS (properties that MUST always hold)
// ============================================================================

#[test]
fn property_toposort_respects_all_edges() {
    // PROPERTY: In any topological sort, for every edge (u,v), u appears before v

    // Test with multiple graph structures
    let test_cases = vec![
        // Case 1: Linear chain
        vec![
            ("a", vec!["b"]),
            ("b", vec!["c"]),
            ("c", vec!["d"]),
        ],
        // Case 2: Diamond
        vec![
            ("a", vec!["b", "c"]),
            ("b", vec!["d"]),
            ("c", vec!["d"]),
        ],
        // Case 3: Complex fan-out/fan-in
        vec![
            ("a", vec!["b", "c", "d"]),
            ("b", vec!["e"]),
            ("c", vec!["e"]),
            ("d", vec!["e"]),
        ],
    ];

    for (case_idx, edges) in test_cases.iter().enumerate() {
        let mut dag = WorkflowDAG::new();

        // Build graph
        let mut all_nodes = HashSet::new();
        for (from, tos) in edges {
            all_nodes.insert(*from);
            for to in tos {
                all_nodes.insert(*to);
            }
        }

        for node in &all_nodes {
            let _ = dag.add_node(node.to_string());
        }

        for (from, tos) in edges {
            for to in tos {
                let _ = dag.add_edge(from.to_string(), to.to_string(), DependencyType::BlockingDependency);
            }
        }

        // PROPERTY: Topological sort must respect ALL edges
        let result = dag.topological_sort();
        assert!(result.is_ok(), "Case {}: toposort should succeed on acyclic graph", case_idx);

        if let Ok(sorted) = result {
            // Verify every edge is respected in the ordering
            for (from, tos) in edges {
                let from_pos = sorted.iter().position(|x| x == &from.to_string());
                assert!(from_pos.is_some(), "Case {}: node {} should be in toposort", case_idx, from);

                for to in tos {
                    let to_pos = sorted.iter().position(|x| x == &to.to_string());
                    assert!(to_pos.is_some(), "Case {}: node {} should be in toposort", case_idx, to);

                    assert!(
                        from_pos < to_pos,
                        "Case {}: edge {}->{} violated in toposort: pos {} vs {}",
                        case_idx, from, to, from_pos.unwrap_or(999), to_pos.unwrap_or(999)
                    );
                }
            }
        }
    }
}

#[test]
fn property_ready_nodes_have_no_incomplete_deps() {
    // PROPERTY: A node is ready IFF all its BlockingDependency predecessors are completed

    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());

    // Dependencies: a,b -> c -> d
    let _ = dag.add_edge("a".to_string(), "c".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("c".to_string(), "d".to_string(), DependencyType::BlockingDependency);

    // Test various completion states
    let test_cases = vec![
        (vec![], vec!["a", "b"]),                    // Nothing complete -> roots ready
        (vec!["a"], vec!["b"]),                       // One dep -> other root still ready
        (vec!["a", "b"], vec!["c"]),                  // Both deps -> c ready
        (vec!["a", "b", "c"], vec!["d"]),             // Chain -> d ready
        (vec!["a", "b", "c", "d"], vec![]),           // All complete -> none ready
    ];

    for (completed_names, expected_ready_names) in test_cases {
        let completed: HashSet<BeadId> = completed_names.iter().map(|s| s.to_string()).collect();
        let ready = dag.get_ready_nodes(&completed);

        let expected_ready: HashSet<String> = expected_ready_names.iter().map(|s| s.to_string()).collect();
        let actual_ready: HashSet<String> = ready.into_iter().collect();

        assert_eq!(
            actual_ready, expected_ready,
            "PROPERTY VIOLATED: Ready nodes don't match expected. Completed: {:?}",
            completed_names
        );

        // HOSTILE: Verify EVERY ready node truly has all deps satisfied
        for ready_node in &actual_ready {
            let deps_result = dag.get_dependencies(&ready_node);
            assert!(deps_result.is_ok(), "get_dependencies should work for ready node");

            if let Ok(deps) = deps_result {
                for dep in deps {
                    // PROPERTY: Every dependency must be completed
                    assert!(
                        completed.contains(&dep),
                        "PROPERTY VIOLATED: Ready node {} has incomplete dependency {}",
                        ready_node, dep
                    );
                }
            }
        }
    }
}

#[test]
fn property_ancestors_and_descendants_are_inverse() {
    // PROPERTY: If B is an ancestor of A, then A must be a descendant of B

    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());

    let _ = dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("c".to_string(), "d".to_string(), DependencyType::BlockingDependency);

    // Test for each node
    for node in ["a", "b", "c", "d"] {
        let ancestors_result = dag.get_all_ancestors(&node.to_string());
        let descendants_result = dag.get_all_descendants(&node.to_string());

        assert!(ancestors_result.is_ok(), "get_all_ancestors should work");
        assert!(descendants_result.is_ok(), "get_all_descendants should work");

        if let (Ok(ancestors), Ok(descendants)) = (ancestors_result, descendants_result) {
            // PROPERTY: For every ancestor X of node N, N must be in descendants of X
            for ancestor in &ancestors {
                let ancestor_descendants_result = dag.get_all_descendants(ancestor);
                assert!(ancestor_descendants_result.is_ok());

                if let Ok(ancestor_descendants) = ancestor_descendants_result {
                    assert!(
                        ancestor_descendants.contains(&node.to_string()),
                        "PROPERTY VIOLATED: {} is ancestor of {}, but {} is not in descendants of {}",
                        ancestor, node, node, ancestor
                    );
                }
            }

            // PROPERTY: For every descendant Y of node N, N must be in ancestors of Y
            for descendant in &descendants {
                let descendant_ancestors_result = dag.get_all_ancestors(descendant);
                assert!(descendant_ancestors_result.is_ok());

                if let Ok(descendant_ancestors) = descendant_ancestors_result {
                    assert!(
                        descendant_ancestors.contains(&node.to_string()),
                        "PROPERTY VIOLATED: {} is descendant of {}, but {} is not in ancestors of {}",
                        descendant, node, node, descendant
                    );
                }
            }
        }
    }
}

#[test]
fn property_removing_node_decreases_count_by_one() {
    // PROPERTY: Removing a node from a graph with N nodes results in N-1 nodes

    let test_sizes = vec![1, 2, 5, 10, 50];

    for size in test_sizes {
        let mut dag = WorkflowDAG::new();

        // Build graph with 'size' nodes
        for i in 0..size {
            let _ = dag.add_node(format!("node_{}", i));
        }

        assert_eq!(dag.node_count(), size, "Initial size should match");

        // Remove each node one by one
        for i in 0..size {
            let before_count = dag.node_count();
            let remove_result = dag.remove_node(&format!("node_{}", i));

            // PROPERTY: If removal succeeds, count must decrease by exactly 1
            if remove_result.is_ok() {
                let after_count = dag.node_count();
                assert_eq!(
                    after_count,
                    before_count - 1,
                    "PROPERTY VIOLATED: Removing node_{} did not decrease count by 1 (before: {}, after: {})",
                    i, before_count, after_count
                );
            }
        }

        assert_eq!(dag.node_count(), 0, "All nodes should be removed");
    }
}

#[test]
fn property_cycle_detection_is_consistent_with_toposort() {
    // PROPERTY: has_cycle() == true IFF topological_sort() fails

    let test_cases = vec![
        // Acyclic graphs
        (vec![("a", "b"), ("b", "c")], false),
        (vec![("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")], false),
        (vec![], false),  // Empty graph
        // Cyclic graphs
        (vec![("a", "b"), ("b", "c"), ("c", "a")], true),  // 3-cycle
        (vec![("a", "b"), ("b", "a")], true),              // 2-cycle
    ];

    for (edges, should_have_cycle) in test_cases {
        let mut dag = WorkflowDAG::new();

        // Build nodes
        let mut nodes = HashSet::new();
        for (from, to) in &edges {
            nodes.insert(from.to_string());
            nodes.insert(to.to_string());
        }

        for node in &nodes {
            let _ = dag.add_node(node.clone());
        }

        // Add edges
        for (from, to) in &edges {
            let _ = dag.add_edge(from.to_string(), to.to_string(), DependencyType::BlockingDependency);
        }

        let has_cycle = dag.has_cycle();
        let toposort_result = dag.topological_sort();

        // PROPERTY: has_cycle and toposort must agree
        assert_eq!(
            has_cycle, should_have_cycle,
            "PROPERTY VIOLATED: has_cycle() returned {} but expected {} for edges {:?}",
            has_cycle, should_have_cycle, edges
        );

        assert_eq!(
            toposort_result.is_err(), should_have_cycle,
            "PROPERTY VIOLATED: toposort success/failure doesn't match has_cycle for edges {:?}",
            edges
        );
    }
}

#[test]
fn property_no_orphaned_beads_after_operations() {
    // PROPERTY: After any sequence of operations, every bead in the graph must be reachable

    let mut dag = WorkflowDAG::new();

    // Build initial graph
    for i in 0..10 {
        let _ = dag.add_node(format!("node_{}", i));
    }

    // Add dependencies
    for i in 0..9 {
        let _ = dag.add_edge(
            format!("node_{}", i),
            format!("node_{}", i + 1),
            DependencyType::BlockingDependency
        );
    }

    // Perform operations
    let _ = dag.remove_node(&"node_5".to_string());

    // PROPERTY: Verify all remaining nodes are reachable from roots or are roots themselves
    let roots = dag.get_roots();
    let mut reachable = HashSet::new();

    for root in &roots {
        reachable.insert(root.clone());
        let descendants_result = dag.get_all_descendants(root);
        if let Ok(descendants) = descendants_result {
            reachable.extend(descendants);
        }
    }

    // Every node in the graph should be reachable
    let all_nodes: HashSet<String> = dag.nodes().cloned().collect();

    for node in &all_nodes {
        assert!(
            reachable.contains(node) || dag.get_dependents(node).map(|deps| deps.is_empty()).unwrap_or(false),
            "PROPERTY VIOLATED: Node {} is orphaned (not reachable from any root)",
            node
        );
    }
}

#[test]
fn property_completed_beads_never_appear_in_ready_list() {
    // PROPERTY: Once a bead is marked completed, it should NEVER appear in ready list

    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency);

    let mut completed = HashSet::new();

    // Simulate workflow execution
    let execution_order = vec!["a", "b", "c"];

    for node in execution_order {
        let ready_before = dag.get_ready_nodes(&completed);

        // Node should be ready before we mark it complete
        assert!(
            ready_before.contains(&node.to_string()),
            "Node {} should be ready before completion",
            node
        );

        // Mark complete
        completed.insert(node.to_string());

        // PROPERTY: Node should NEVER appear in ready list after completion
        let ready_after = dag.get_ready_nodes(&completed);
        assert!(
            !ready_after.contains(&node.to_string()),
            "PROPERTY VIOLATED: Completed node {} still appears in ready list",
            node
        );

        // PROPERTY: This should hold for ALL future checks
        for _ in 0..3 {
            let ready_check = dag.get_ready_nodes(&completed);
            assert!(
                !ready_check.contains(&node.to_string()),
                "PROPERTY VIOLATED: Completed node {} reappeared in ready list on subsequent check",
                node
            );
        }
    }
}

#[test]
fn property_subgraph_preserves_local_structure() {
    // PROPERTY: Edges in subgraph must exist in original graph with same properties

    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());
    let _ = dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("b".to_string(), "c".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("c".to_string(), "d".to_string(), DependencyType::PreferredOrder);

    // Create subgraph
    let subgraph_result = dag.subgraph(&["b".to_string(), "c".to_string()]);
    assert!(subgraph_result.is_ok());

    if let Ok(sub) = subgraph_result {
        // PROPERTY: Every edge in subgraph must exist in original
        let sub_edges: Vec<_> = sub.edges().collect();
        let original_edges: Vec<_> = dag.edges().collect();

        for (sub_from, sub_to, sub_type) in &sub_edges {
            let found = original_edges.iter().any(|(orig_from, orig_to, orig_type)| {
                sub_from == orig_from && sub_to == orig_to && sub_type == orig_type
            });

            assert!(
                found,
                "PROPERTY VIOLATED: Subgraph edge {}->{} (type {:?}) not found in original graph",
                sub_from, sub_to, sub_type
            );
        }
    }
}

#[test]
fn property_get_dependencies_and_get_dependents_are_inverse() {
    // PROPERTY: If B is in get_dependencies(A), then A must be in get_dependents(B)

    let mut dag = WorkflowDAG::new();
    let _ = dag.add_node("a".to_string());
    let _ = dag.add_node("b".to_string());
    let _ = dag.add_node("c".to_string());
    let _ = dag.add_node("d".to_string());

    let _ = dag.add_edge("a".to_string(), "b".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("a".to_string(), "c".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("b".to_string(), "d".to_string(), DependencyType::BlockingDependency);
    let _ = dag.add_edge("c".to_string(), "d".to_string(), DependencyType::BlockingDependency);

    for node in ["a", "b", "c", "d"] {
        let deps_result = dag.get_dependencies(&node.to_string());
        assert!(deps_result.is_ok());

        if let Ok(deps) = deps_result {
            for dep in deps {
                // PROPERTY: If dep is a dependency of node, then node must be a dependent of dep
                let dependents_result = dag.get_dependents(&dep);
                assert!(dependents_result.is_ok());

                if let Ok(dependents) = dependents_result {
                    assert!(
                        dependents.contains(&node.to_string()),
                        "PROPERTY VIOLATED: {} is in dependencies of {}, but {} is not in dependents of {}",
                        dep, node, node, dep
                    );
                }
            }
        }
    }
}
