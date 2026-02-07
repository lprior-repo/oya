//! Dependency validation tests for add_dependency method
//!
//! Tests for comprehensive validation in add_dependency including:
//! - Self-loop detection
//! - Node existence validation
//! - Duplicate edge detection
//! - Cycle detection (using Tarjan or DFS)
//!
//! All tests follow zero-panic principles with Result-based error handling.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use crate::dag::{DagError, DagResult, DependencyType, WorkflowDAG};

// ============================================================================
// ADD_DEPENDENCY VALIDATION TESTS
// ============================================================================

/// Test that add_dependency successfully creates a valid dependency relationship
#[test]
fn given_two_nodes_when_add_dependency_then_success() -> DagResult<()> {
    // GIVEN: Two independent nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;

    // WHEN: Adding a dependency from a to b
    let result = dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Dependency is created successfully
    assert!(result.is_ok(), "Adding dependency should succeed");
    assert_eq!(dag.edge_count(), 1, "Should have exactly one edge");

    // Verify the edge is queryable
    let deps = dag.get_dependencies(&"task-b".to_string())?;
    assert_eq!(deps, vec!["task-a".to_string()]);

    Ok(())
}

/// Test that self-loops are rejected
#[test]
fn given_node_when_add_dependency_to_self_then_self_loop_error() -> DagResult<()> {
    // GIVEN: A single node
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;

    // WHEN: Attempting to add a self-loop
    let result = dag.add_dependency(
        "task-a".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Self-loop error is returned
    assert!(result.is_err(), "Self-loop should be rejected");

    match result {
        Err(DagError::SelfLoopDetected(node)) => {
            assert_eq!(node, "task-a", "Error should contain the node ID");
        }
        _ => panic!("Expected SelfLoopDetected error, got {:?}", result),
    }

    // Verify no edge was created
    assert_eq!(dag.edge_count(), 0, "No edge should be created");

    Ok(())
}

/// Test that dependency with non-existent source node fails
#[test]
fn given_dag_when_add_dependency_with_missing_source_then_node_not_found() -> DagResult<()> {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-b".to_string())?;

    // WHEN: Attempting to add dependency from non-existent source
    let result = dag.add_dependency(
        "missing-source".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: NodeNotFound error is returned
    assert!(result.is_err(), "Missing source should be rejected");

    match result {
        Err(DagError::NodeNotFound(node)) => {
            assert_eq!(node, "missing-source", "Error should contain source ID");
        }
        _ => panic!("Expected NodeNotFound error, got {:?}", result),
    }

    Ok(())
}

/// Test that dependency with non-existent target node fails
#[test]
fn given_dag_when_add_dependency_with_missing_target_then_node_not_found() -> DagResult<()> {
    // GIVEN: A DAG with one node
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;

    // WHEN: Attempting to add dependency to non-existent target
    let result = dag.add_dependency(
        "task-a".to_string(),
        "missing-target".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: NodeNotFound error is returned
    assert!(result.is_err(), "Missing target should be rejected");

    match result {
        Err(DagError::NodeNotFound(node)) => {
            assert_eq!(node, "missing-target", "Error should contain target ID");
        }
        _ => panic!("Expected NodeNotFound error, got {:?}", result),
    }

    Ok(())
}

/// Test that duplicate dependencies are rejected
#[test]
fn given_edge_when_add_duplicate_dependency_then_edge_already_exists() -> DagResult<()> {
    // GIVEN: A DAG with an existing edge
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Attempting to add the same dependency again
    let result = dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: EdgeAlreadyExists error is returned
    assert!(result.is_err(), "Duplicate edge should be rejected");

    match result {
        Err(DagError::EdgeAlreadyExists(from, to)) => {
            assert_eq!(from, "task-a", "Error should contain source ID");
            assert_eq!(to, "task-b", "Error should contain target ID");
        }
        _ => panic!("Expected EdgeAlreadyExists error, got {:?}", result),
    }

    // Verify only one edge exists
    assert_eq!(dag.edge_count(), 1, "Should still have exactly one edge");

    Ok(())
}

/// Test that simple two-node cycle is detected
#[test]
fn given_edge_when_add_reverse_creating_cycle_then_cycle_detected() -> DagResult<()> {
    // GIVEN: A DAG with edge a -> b
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Attempting to add edge b -> a (creating a cycle)
    let result = dag.add_dependency(
        "task-b".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: CycleDetected error is returned
    assert!(result.is_err(), "Cycle should be detected");

    match result {
        Err(DagError::CycleDetected(nodes)) => {
            assert!(
                nodes.contains(&"task-a".to_string()),
                "Cycle should contain task-a"
            );
            assert!(
                nodes.contains(&"task-b".to_string()),
                "Cycle should contain task-b"
            );
        }
        _ => panic!("Expected CycleDetected error, got {:?}", result),
    }

    // Verify only one edge exists (the second was not added)
    assert_eq!(dag.edge_count(), 1, "Should still have exactly one edge");

    Ok(())
}

/// Test that three-node cycle is detected
#[test]
fn given_chain_when_add_edge_creating_three_node_cycle_then_cycle_detected() -> DagResult<()> {
    // GIVEN: A DAG with chain a -> b -> c
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_node("task-c".to_string())?;
    dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-b".to_string(),
        "task-c".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Attempting to add edge c -> a (creating a cycle)
    let result = dag.add_dependency(
        "task-c".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: CycleDetected error is returned
    assert!(result.is_err(), "Cycle should be detected");

    match result {
        Err(DagError::CycleDetected(nodes)) => {
            assert_eq!(nodes.len(), 3, "All three nodes should be in cycle");
            assert!(
                nodes.contains(&"task-a".to_string()),
                "Cycle should contain task-a"
            );
            assert!(
                nodes.contains(&"task-b".to_string()),
                "Cycle should contain task-b"
            );
            assert!(
                nodes.contains(&"task-c".to_string()),
                "Cycle should contain task-c"
            );
        }
        _ => panic!("Expected CycleDetected error, got {:?}", result),
    }

    Ok(())
}

/// Test that complex cycle is detected
#[test]
fn given_diamond_when_add_edge_closing_loop_then_cycle_detected() -> DagResult<()> {
    // GIVEN: A DAG with diamond structure
    //     a
    //    / \
    //   b   c
    //    \ /
    //     d
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_node("task-c".to_string())?;
    dag.add_node("task-d".to_string())?;

    dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-a".to_string(),
        "task-c".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-b".to_string(),
        "task-d".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-c".to_string(),
        "task-d".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Attempting to add edge d -> a (creating a cycle)
    let result = dag.add_dependency(
        "task-d".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: CycleDetected error is returned
    assert!(result.is_err(), "Cycle should be detected");

    match result {
        Err(DagError::CycleDetected(_)) => {
            // Cycle detected - success
        }
        _ => panic!("Expected CycleDetected error, got {:?}", result),
    }

    Ok(())
}

/// Test that valid acyclic dependencies are accepted
#[test]
fn given_complex_dag_when_add_valid_dependency_then_success() -> DagResult<()> {
    // GIVEN: A complex DAG with multiple independent chains
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_node("task-c".to_string())?;
    dag.add_node("task-d".to_string())?;

    dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-c".to_string(),
        "task-d".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Adding a valid cross-edge (b -> c)
    let result = dag.add_dependency(
        "task-b".to_string(),
        "task-c".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Dependency is created successfully
    assert!(result.is_ok(), "Valid cross-edge should succeed");
    assert_eq!(dag.edge_count(), 3, "Should have three edges");

    // Verify the chain a -> b -> c -> d
    let deps_b = dag.get_dependencies(&"task-b".to_string())?;
    assert_eq!(deps_b, vec!["task-a".to_string()]);

    let deps_c = dag.get_dependencies(&"task-c".to_string())?;
    assert_eq!(deps_c, vec!["task-b".to_string()]);

    Ok(())
}

/// Test that dependency type is preserved
#[test]
fn given_nodes_when_add_preferred_order_dependency_then_type_preserved() -> DagResult<()> {
    // GIVEN: Two nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;

    // WHEN: Adding a PreferredOrder dependency
    let result = dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::PreferredOrder,
    );

    // THEN: Dependency is created with correct type
    assert!(result.is_ok(), "Adding PreferredOrder should succeed");

    let edges: Vec<_> = dag.edges().collect();
    assert_eq!(edges.len(), 1);
    assert_eq!(*edges[0].2, DependencyType::PreferredOrder);

    Ok(())
}

/// Test that adding multiple dependencies from same source works
#[test]
fn given_node_when_add_multiple_dependencies_then_all_created() -> DagResult<()> {
    // GIVEN: A DAG with one source node and two target nodes
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_node("task-c".to_string())?;

    // WHEN: Adding multiple dependencies from the same source
    dag.add_dependency(
        "task-a".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-a".to_string(),
        "task-c".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // THEN: Both dependencies are created
    assert_eq!(dag.edge_count(), 2);

    let deps = dag.get_dependents(&"task-a".to_string())?;
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&"task-b".to_string()));
    assert!(deps.contains(&"task-c".to_string()));

    Ok(())
}

/// Test that DAG remains acyclic after valid additions
#[test]
fn given_dag_when_add_valid_edges_then_still_acyclic() -> DagResult<()> {
    // GIVEN: An empty DAG
    let mut dag = WorkflowDAG::new();

    // WHEN: Building a valid tree structure
    //       root
    //      / | \
    //     a  b  c
    //     |  |
    //     d  e
    dag.add_node("root".to_string())?;
    dag.add_node("task-a".to_string())?;
    dag.add_node("task-b".to_string())?;
    dag.add_node("task-c".to_string())?;
    dag.add_node("task-d".to_string())?;
    dag.add_node("task-e".to_string())?;

    dag.add_dependency(
        "root".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "root".to_string(),
        "task-b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "root".to_string(),
        "task-c".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-a".to_string(),
        "task-d".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "task-b".to_string(),
        "task-e".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // THEN: DAG should remain acyclic
    assert!(!dag.has_cycle(), "Valid tree structure should be acyclic");

    // Should be able to topological sort
    let sorted = dag.topological_sort();
    assert!(sorted.is_ok(), "Should be able to topological sort");

    Ok(())
}

/// Test cycle detection with longer chain
#[test]
fn given_long_chain_when_add_edge_creating_cycle_then_cycle_detected() -> DagResult<()> {
    // GIVEN: A DAG with a long chain
    // a -> b -> c -> d -> e
    let mut dag = WorkflowDAG::new();
    for name in &["a", "b", "c", "d", "e"] {
        dag.add_node(name.to_string())?;
    }

    dag.add_dependency(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "c".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "d".to_string(),
        "e".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Attempting to add edge e -> a (creating a cycle)
    let result = dag.add_dependency(
        "e".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Cycle is detected
    assert!(result.is_err(), "Cycle should be detected");

    match result {
        Err(DagError::CycleDetected(nodes)) => {
            assert_eq!(nodes.len(), 5, "All five nodes should be in cycle");
        }
        _ => panic!("Expected CycleDetected error, got {:?}", result),
    }

    Ok(())
}

/// Test that transitive cycle is detected
#[test]
fn given_dag_when_add_dependency_creating_transitive_cycle_then_cycle_detected()
-> DagResult<()> {
    // GIVEN: A DAG with structure
    // a -> b -> c
    //      |
    //      v
    //      d
    let mut dag = WorkflowDAG::new();
    for name in &["a", "b", "c", "d"] {
        dag.add_node(name.to_string())?;
    }

    dag.add_dependency(
        "a".to_string(),
        "b".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "b".to_string(),
        "c".to_string(),
        DependencyType::BlockingDependency,
    )?;
    dag.add_dependency(
        "b".to_string(),
        "d".to_string(),
        DependencyType::BlockingDependency,
    )?;

    // WHEN: Adding edge d -> a (creating transitive cycle through b)
    let result = dag.add_dependency(
        "d".to_string(),
        "a".to_string(),
        DependencyType::BlockingDependency,
    );

    // THEN: Cycle is detected
    assert!(result.is_err(), "Transitive cycle should be detected");

    match result {
        Err(DagError::CycleDetected(_)) => {
            // Success - cycle detected
        }
        _ => panic!("Expected CycleDetected error, got {:?}", result),
    }

    Ok(())
}

/// Test zero-panic property: all errors are returned, never panic
#[test]
fn given_various_invalid_inputs_when_add_dependency_then_never_panics() {
    let test_cases = vec![
        // Self-loop cases
        ("self-loop", "self-loop", "self-loop"),
        // Non-existent nodes
        ("missing-a", "missing-b", "both missing"),
        ("exists-a", "missing-b", "target missing"),
        ("missing-a", "exists-b", "source missing"),
    ];

    for (source, target, desc) in test_cases {
        let mut dag = WorkflowDAG::new();

        // Add nodes that "exist"
        let _ = dag.add_node("exists-a".to_string());
        let _ = dag.add_node("exists-b".to_string());

        // Attempt invalid operation - should never panic
        let result =
            dag.add_dependency(source.to_string(), target.to_string(), DependencyType::BlockingDependency);

        // Should always return an error, never panic
        assert!(result.is_err(), "Test case '{}' should return error", desc);
    }
}

/// Test that error messages are descriptive
#[test]
fn given_invalid_dependency_when_error_then_message_is_descriptive() -> DagResult<()> {
    let mut dag = WorkflowDAG::new();
    dag.add_node("task-a".to_string())?;

    // Test self-loop error message
    let result = dag.add_dependency(
        "task-a".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    );
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            msg.contains("task-a"),
            "Error message should contain node ID: {}",
            msg
        );
        assert!(
            msg.contains("self-loop") || msg.contains("Self-loop"),
            "Error message should mention self-loop: {}",
            msg
        );
    }

    // Test missing node error message
    let result = dag.add_dependency(
        "missing".to_string(),
        "task-a".to_string(),
        DependencyType::BlockingDependency,
    );
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            msg.contains("missing") || msg.contains("not found"),
            "Error message should mention missing node: {}",
            msg
        );
    }

    Ok(())
}
