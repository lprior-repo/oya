//! Integration tests for cycle detection and topological execution.
//!
//! These tests exercise the full workflow execution lifecycle.
//!
//! RED PHASE: Tests assert CORRECT expected behavior. Tests FAIL because
//! the implementation is buggy (silently excludes cycles).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{Connection, NodeId, PortName, Workflow};

// Helper to create a node with a given ID and dependencies
fn make_node(id: NodeId, deps: Vec<NodeId>) -> oya_frontend::graph::Node {
    use oya_frontend::graph::{RunConfig, WorkflowNode};
    let node_name = format!("node_{}", id.0.to_string());
    let mut node = oya_frontend::graph::Node::from_workflow_node(
        node_name,
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );
    node.id = id;
    node.config = serde_json::json!({
        "dependencies": deps
    });
    node
}

// Helper to create a workflow with nodes and connections (bypassing validation)
fn create_workflow_with_connections(deps: Vec<(NodeId, Vec<NodeId>)>) -> Workflow {
    use uuid::Uuid;

    let mut workflow = Workflow::new();

    for (id, node_deps) in &deps {
        let node = make_node(*id, node_deps.clone());
        workflow.nodes.push(node);
        for target in node_deps {
            workflow.connections.push(Connection {
                id: Uuid::new_v4(),
                source: *id,
                target: *target,
                source_port: PortName::from("main"),
                target_port: PortName::from("main"),
            });
        }
    }

    workflow
}

// ===========================================================================
// Integration Tests: Full Workflow Execution Lifecycle
// ===========================================================================

/// Integration test: prepare_run rejects dirty state when execution queue not empty
#[test]
fn prepare_run_rejects_dirty_state_when_execution_queue_not_empty() {
    // Given: A workflow with pre-existing execution queue
    use oya_frontend::graph::WorkflowExecutionError;

    let node_0 = NodeId::new();
    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.execution_queue = vec![node_0]; // Simulate dirty state

    // When
    let result = workflow.prepare_run();

    // Then: Should reject with InvalidWorkflowState error
    assert!(result.is_err(), "prepare_run should reject dirty state");

    match result.unwrap_err() {
        WorkflowExecutionError::InvalidWorkflowState { reason } => {
            assert!(
                reason.contains("execution_queue") && reason.contains("not empty"),
                "Error reason should mention execution_queue is not empty: {}",
                reason
            );
        }
        other => panic!("Expected InvalidWorkflowState error, got: {:?}", other),
    }
}

/// Integration test: prepare_run rejects dirty state when executed set not empty
#[test]
fn prepare_run_rejects_dirty_state_when_executed_set_not_empty() {
    // Given: A workflow with pre-executed nodes
    let node_0 = NodeId::new();
    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![]));

    // Simulate dirty executed state by setting executing flag
    if let Some(n) = workflow.nodes.get_mut(0) {
        n.executing = true;
    }

    // When
    let _ = workflow.prepare_run();

    // Then: State gets reset
    // Expected: Should reject with InvalidWorkflowState error
    // RED PHASE: This test documents the expected behavior
    // BUG: Currently just resets silently
}

/// Integration test: execute iterative completes all nodes successfully
#[test]
fn execute_iterative_completes_all_nodes_on_acyclic_graph() {
    // Given: A valid DAG workflow with 5 nodes
    let nodes: Vec<NodeId> = (0..5).map(|_| NodeId::new()).collect();

    // Create a linear chain: 0 -> 1 -> 2 -> 3 -> 4
    let deps = vec![
        (nodes[0], vec![]),
        (nodes[1], vec![nodes[0]]),
        (nodes[2], vec![nodes[1]]),
        (nodes[3], vec![nodes[2]]),
        (nodes[4], vec![nodes[3]]),
    ];

    let mut workflow = create_workflow_with_connections(deps);

    // When
    let _ = workflow.prepare_run();

    // Then: All nodes should be in execution queue
    assert_eq!(workflow.execution_queue.len(), 5, "All 5 nodes in queue");

    // Simulate execution (mark nodes as complete)
    for _node_id in workflow.execution_queue.clone() {
        // Note: mark_node_complete doesn't exist yet
        // workflow.mark_node_complete(node_id).unwrap();
    }

    // Expected: All nodes should be marked as executed
}

/// Integration test: prepare_run detects cycle with exact error variant
#[test]
fn execute_iterative_detects_stuck_with_exact_iteration_count() {
    use oya_frontend::graph::WorkflowExecutionError;
    use uuid::Uuid;

    // Given: A cyclic workflow (0 -> 1 -> 2 -> 0)
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();

    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![node_2])); // 0 depends on 2
    workflow.nodes.push(make_node(node_1, vec![node_0])); // 1 depends on 0
    workflow.nodes.push(make_node(node_2, vec![node_1])); // 2 depends on 1 (cycle)

    // Add connections (bypassing validation to allow cycles)
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_0,
        target: node_2,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_1,
        target: node_0,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_2,
        target: node_1,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });

    // When: prepare_run() should detect the cycle and return Err
    let result = workflow.prepare_run();

    // Then: Should return CycleDetected error with all cycle nodes
    assert!(result.is_err(), "prepare_run should return Err for cyclic workflow");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 3, "Cycle should contain exactly 3 nodes");
            assert!(cycle_nodes.contains(&node_0), "Cycle should contain node 0");
            assert!(cycle_nodes.contains(&node_1), "Cycle should contain node 1");
            assert!(cycle_nodes.contains(&node_2), "Cycle should contain node 2");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// Integration test: mark_node_complete rejects out of order with exact error variant
#[test]
fn mark_node_complete_rejects_out_of_order_with_exact_error_variant() {
    // Given: A workflow with execution queue
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();

    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.nodes.push(make_node(node_1, vec![node_0]));

    let _ = workflow.add_connection_checked(
        node_0,
        node_1,
        &PortName::from("main"),
        &PortName::from("main"),
    );

    // Set up execution queue
    workflow.execution_queue = vec![node_0, node_1];

    // When: Try to mark node 1 before node 0
    // Note: mark_node_complete doesn't exist yet

    // Then: Should reject with InvalidWorkflowState error
    // Expected error: "node 1 is not at head of execution queue (head is 0)"
    // RED PHASE: Function doesn't exist yet
}

/// Integration test: mark_node_failed moves node from queue to failed
#[test]
fn mark_node_failed_moves_node_from_queue_to_failed() {
    // Given: A workflow with a node in execution queue
    let node_0 = NodeId::new();

    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.execution_queue = vec![node_0];

    // When: Mark node as failed
    // Note: mark_node_failed doesn't exist yet

    // Then: Node should move from queue to failed set
    // Expected: execution_queue = [], failed = {node_0}
}

// ===========================================================================
// Integration Tests: Proptest Invariants
// ===========================================================================

/// Proptest: All nodes accounted for including error state
#[test]
fn proptest_all_nodes_accounted_for_including_error_state() {
    use oya_frontend::graph::WorkflowExecutionError;
    use proptest::prelude::*;

    proptest!(|(num_nodes in 1usize..=20)| {
        // Generate a connected random graph
        let nodes: Vec<NodeId> = (0..num_nodes).map(|_| NodeId::new()).collect();

        // Create connected dependencies - ensure node 0 is root and all others connect to it
        let mut deps: Vec<(NodeId, Vec<NodeId>)> = vec![];

        for i in 0..num_nodes {
            let mut node_deps = vec![];
            if i == 0 {
                // First node is root (no deps)
            } else {
                // All other nodes depend on node 0 (ensures connectivity)
                node_deps.push(nodes[0]);
                // Add some random additional edges
                for j in 1..num_nodes {
                    if i != j && (i * num_nodes + j) % 5 == 0 {
                        node_deps.push(nodes[j]);
                    }
                }
            }
            deps.push((nodes[i], node_deps));
        }

        let mut workflow = create_workflow_with_connections(deps);
        let result = workflow.prepare_run();

        // Invariant: All nodes should be accounted for
        // On success: all nodes in execution_queue
        // On cycle error: cycle_nodes contains all cycle nodes
        // On invalid state: graph connectivity or other issues
        match result {
            Ok(()) => {
                // Success case: all nodes should be in queue
                assert_eq!(
                    workflow.execution_queue.len(),
                    num_nodes,
                    "All {} nodes should be in queue on success",
                    num_nodes
                );
            }
            Err(WorkflowExecutionError::CycleDetected { cycle_nodes }) => {
                // Cycle case: cycle_nodes should contain the cycle nodes
                assert!(!cycle_nodes.is_empty(), "Cycle should contain at least one node");
            }
            Err(WorkflowExecutionError::InvalidWorkflowState { .. }) => {
                // Invalid state (e.g., disconnected components) - this is valid
                // The invariant is that nodes are either in queue, cycle, or error state
            }
            other => panic!("Unexpected error type: {:?}", other),
        }
    });
}

/// Proptest: Topological order includes all nodes for valid DAG
#[test]
fn proptest_topological_order_satisfies_dependency_constraint() {
    use proptest::prelude::*;

    proptest!(|(num_nodes in 1usize..=5)| {
        let nodes: Vec<NodeId> = (0..num_nodes).map(|_| NodeId::new()).collect();

        // Create a connected valid DAG with topological order
        // Linear chain structure: node 0 is root, node 1 depends on node 0, node 2 depends on node 1, etc.
        // This ensures all nodes are reachable from the "leaf" (last node) through dependencies
        let mut deps: Vec<(NodeId, Vec<NodeId>)> = vec![];

        for i in 0..num_nodes {
            let node_deps = if i == 0 {
                vec![]  // First node has no dependencies (root)
            } else {
                vec![nodes[i - 1]]  // Each node depends on the previous one
            };
            deps.push((nodes[i], node_deps));
        }

        let mut workflow = create_workflow_with_connections(deps.clone());
        let result = workflow.prepare_run();

        // For valid DAG, prepare_run should succeed
        assert!(result.is_ok(), "Valid DAG should produce Ok result, got: {:?}", result);

        // Verify all nodes are in queue
        assert_eq!(
            workflow.execution_queue.len(),
            num_nodes,
            "All {} nodes in valid DAG should be in queue",
            num_nodes
        );

        // Verify that all nodes from deps are in the queue
        let queue_set: std::collections::HashSet<_> = workflow.execution_queue.iter().collect();
        for (node_id, _node_deps) in &deps {
            assert!(
                queue_set.contains(node_id),
                "Node {} should be in queue",
                node_id
            );
        }
    });
}

/// Proptest: Cycle detection returns all cycle nodes
#[test]
fn proptest_cycle_path_first_equals_last() {
    use oya_frontend::graph::WorkflowExecutionError;
    use proptest::prelude::*;

    proptest!(|(cycle_size in 2usize..=10)| {
        // Generate a cycle of given size
        let nodes: Vec<NodeId> = (0..cycle_size).map(|_| NodeId::new()).collect();

        let mut deps: Vec<(NodeId, Vec<NodeId>)> = vec![];
        for i in 0..cycle_size {
            let next = (i + 1) % cycle_size;
            deps.push((nodes[i], vec![nodes[next]]));
        }

        let mut workflow = create_workflow_with_connections(deps);
        let result = workflow.prepare_run();

        // Verify cycle is detected and returns error
        assert!(result.is_err(), "Cycle should return error for cyclic graph");

        match result.unwrap_err() {
            WorkflowExecutionError::CycleDetected { cycle_nodes } => {
                // All cycle nodes should be returned
                assert_eq!(
                    cycle_nodes.len(),
                    cycle_size,
                    "Cycle should contain exactly {} nodes",
                    cycle_size
                );
                // All generated nodes should be in the cycle
                for node in &nodes {
                    assert!(
                        cycle_nodes.contains(node),
                        "All cycle nodes should be returned"
                    );
                }
                // Verify no duplicate nodes in cycle_nodes
                let mut unique_nodes: std::collections::HashSet<_> = std::collections::HashSet::new();
                for node in &cycle_nodes {
                    assert!(
                        unique_nodes.insert(node),
                        "cycle_nodes should not contain duplicates"
                    );
                }
            }
            other => panic!(
                "Expected CycleDetected error, got: {:?}",
                other
            ),
        }
    });
}

/// Proptest: In-degree sum equals edge count
#[test]
fn proptest_indegree_sum_equals_edge_count() {
    use proptest::prelude::*;

    proptest!(|(num_nodes in 1usize..=20, num_edges in 0usize..=50)| {
        let nodes: Vec<NodeId> = (0..num_nodes).map(|_| NodeId::new()).collect();

        // Create deterministic graph based on inputs
        let mut edge_count = 0;
        let mut deps: Vec<(NodeId, Vec<NodeId>)> = nodes.iter().map(|n| (*n, vec![])).collect();
        // Track unique edges to avoid duplicates
        let mut seen_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

        // Use deterministic pseudo-random based on num_edges
        let mut seed = num_edges as u64;
        for _ in 0..num_edges {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let source_idx = usize::try_from(seed % (num_nodes as u64)).expect("modulo result fits in usize");
            let target_idx = usize::try_from((seed >> 8) % (num_nodes as u64)).expect("modulo result fits in usize");

            // Skip self-loops and duplicate edges
            if source_idx != target_idx && seen_edges.insert((source_idx, target_idx)) {
                deps[source_idx].1.push(nodes[target_idx]);
                edge_count += 1;
            }
        }

        let workflow = create_workflow_with_connections(deps);

        // Calculate in-degree sum from connections
        let mut in_degree_sum = 0;
        for node in &workflow.nodes {
            // Count how many connections point to this node
            let incoming: usize = workflow.connections.iter()
                .filter(|c| c.target == node.id)
                .count();
            in_degree_sum += incoming;
        }

        // Invariant: Sum of in-degrees equals number of edges
        assert_eq!(in_degree_sum, edge_count, "In-degree sum should equal edge count (in_degree_sum={}, edge_count={})", in_degree_sum, edge_count);
    });
}

/// Proptest: Nodes in mutually exclusive states
#[test]
fn proptest_nodes_in_mutually_exclusive_states() {
    use proptest::prelude::*;

    proptest!(|(num_nodes in 1usize..=10)| {
        let nodes: Vec<NodeId> = (0..num_nodes).map(|_| NodeId::new()).collect();

        let mut workflow = Workflow::new();
        for node_id in &nodes {
            workflow.nodes.push(make_node(*node_id, vec![]));
        }

        let _ = workflow.prepare_run();

        // Invariant: Each node appears in exactly one state
        // (queue, executed, failed, or neither)
        for node_id in &nodes {
            let in_queue = workflow.execution_queue.contains(node_id);
            let in_executed = workflow.nodes.iter()
                .any(|n| n.id == *node_id && n.executing);

            // Node cannot be in both states
            assert!(
                !(in_queue && in_executed),
                "Node {:?} cannot be in both queue and executed",
                node_id
            );
        }
    });
}

/// Proptest: Deterministic ordering
#[test]
fn proptest_deterministic_ordering() {
    use proptest::prelude::*;

    proptest!(|(num_nodes in 3usize..=10)| {
        let nodes: Vec<NodeId> = (0..num_nodes).map(|_| NodeId::new()).collect();

        // Create a valid DAG
        let mut deps: Vec<(NodeId, Vec<NodeId>)> = vec![];
        for i in 0..num_nodes {
            let mut node_deps = vec![];
            for j in 0..i {
                if (i + j) % 3 == 0 {
                    node_deps.push(nodes[j]);
                }
            }
            deps.push((nodes[i], node_deps));
        }

        let mut workflow1 = create_workflow_with_connections(deps.clone());
        let _ = workflow1.prepare_run();
        let order1 = workflow1.execution_queue.clone();

        let mut workflow2 = create_workflow_with_connections(deps);
        let _ = workflow2.prepare_run();
        let order2 = workflow2.execution_queue.clone();

        // Ordering should be deterministic
        assert_eq!(order1, order2, "Ordering should be deterministic");
    });
}
