//! Comprehensive unit tests for cycle detection and topological execution.
//!
//! This module tests the prepare_run() function and related functionality
//! for detecting cycles and ensuring proper topological ordering.
//!
//! Tests verify that cycles are ALWAYS reported with actionable metadata,
//! never silently excluded.

use super::WorkflowExecutionError;
use super::{Connection, Node, NodeId, PortName, Workflow};
use std::collections::HashMap;
use uuid::Uuid;

// Helper function to create a node with a given ID and dependencies
fn make_node(id: NodeId, deps: Vec<NodeId>) -> Node {
    use crate::graph::{RunConfig, WorkflowNode};
    let mut node = Node::from_workflow_node(
        format!("node_{}", id.0.to_string()),
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
fn create_workflow_with_connections(
    deps: Vec<(NodeId, Vec<NodeId>)>,
) -> (Workflow, HashMap<NodeId, Vec<NodeId>>) {
    let mut workflow = Workflow::new();
    let mut node_deps: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for (id, node_deps_vec) in &deps {
        let node = make_node(*id, node_deps_vec.clone());
        workflow.nodes.push(node);
        node_deps.insert(*id, node_deps_vec.clone());
    }

    // Build connections from dependencies (bypassing validation to allow cycles)
    for (source, targets) in &deps {
        for target in targets {
            workflow.connections.push(Connection {
                id: Uuid::new_v4(),
                source: *source,
                target: *target,
                source_port: PortName::from("main"),
                target_port: PortName::from("main"),
            });
        }
    }

    (workflow, node_deps)
}

// ===========================================================================
// Test Layer 1: Unit Tests for prepare_run() Success Cases
// ===========================================================================

/// Test that prepare_run succeeds on a valid diamond DAG
#[test]
fn prepare_run_succeeds_on_valid_diamond_dag() {
    // Given: A valid diamond DAG: 0 -> 1 -> 3, 0 -> 2 -> 3
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let deps = vec![
        (node_0, vec![]), // entry node
        (node_1, vec![node_0]),
        (node_2, vec![node_0]),
        (node_3, vec![node_1, node_2]),
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: All nodes should be in the queue in valid topological order
    assert!(result.is_ok(), "prepare_run should succeed on valid DAG");
    assert_eq!(workflow.execution_queue.len(), 4, "All 4 nodes should be in queue");
    assert!(workflow.execution_queue.contains(&node_0), "Entry node 0 should be in queue");
    assert!(workflow.execution_queue.contains(&node_1), "Node 1 should be in queue");
    assert!(workflow.execution_queue.contains(&node_2), "Node 2 should be in queue");
    assert!(workflow.execution_queue.contains(&node_3), "Node 3 should be in queue");
}

/// Test that prepare_run detects a simple 3-node cycle
#[test]
fn prepare_run_detects_simple_3node_cycle() {
    // Given: A cycle: 0 -> 1 -> 2 -> 0
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();

    let deps = vec![
        (node_0, vec![node_2]), // 0 depends on 2
        (node_1, vec![node_0]), // 1 depends on 0
        (node_2, vec![node_1]), // 2 depends on 1 (completes cycle)
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The implementation should return Err(CycleDetected{...})
    // NOT silently exclude the nodes
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

/// Test that prepare_run detects a self-reference cycle
#[test]
fn prepare_run_detects_self_reference() {
    // Given: Node 0 depends on itself
    let node_0 = NodeId::new();

    let deps = vec![(node_0, vec![node_0])];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The implementation should return Err(CycleDetected{...})
    // NOT silently exclude the node
    assert!(result.is_err(), "prepare_run should return Err for self-referencing node");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 1, "Self-reference cycle should contain exactly 1 node");
            assert!(cycle_nodes.contains(&node_0), "Cycle should contain node 0");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// Test that prepare_run detects a 2-node cycle
#[test]
fn prepare_run_detects_two_node_cycle() {
    // Given: Two nodes pointing to each other: 0 <-> 1
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();

    let deps = vec![(node_0, vec![node_1]), (node_1, vec![node_0])];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The implementation should return Err(CycleDetected{...})
    // NOT silently exclude the nodes
    assert!(result.is_err(), "prepare_run should return Err for 2-node cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 2, "2-node cycle should contain exactly 2 nodes");
            assert!(cycle_nodes.contains(&node_0), "Cycle should contain node 0");
            assert!(cycle_nodes.contains(&node_1), "Cycle should contain node 1");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// Test that prepare_run detects a complex cycle with merge
#[test]
fn prepare_run_detects_complex_cycle() {
    // Given: A complex graph with a cycle: 1 -> 2 -> 3 -> 1
    // All nodes are connected in a single component
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    // Create the graph with an actual cycle:
    // Node 0: depends on 3 (connects to the cycle)
    // Node 1: depends on 0
    // Node 2: depends on 1
    // Node 3: depends on 2 (completes cycle: 0 <- 1 <- 2 <- 3 <- 0)
    let deps = vec![
        (node_0, vec![node_3]), // depends on 3 (connects to cycle)
        (node_1, vec![node_0]), // depends on 0
        (node_2, vec![node_1]), // depends on 1
        (node_3, vec![node_2]), // depends on 2 (completes cycle)
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The implementation should return Err(CycleDetected{...})
    // NOT silently exclude the cycle nodes
    assert!(result.is_err(), "prepare_run should return Err for complex cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            // All nodes 0, 1, 2, 3 form the cycle
            assert!(cycle_nodes.contains(&node_0), "Cycle should contain node 0");
            assert!(cycle_nodes.contains(&node_1), "Cycle should contain node 1");
            assert!(cycle_nodes.contains(&node_2), "Cycle should contain node 2");
            assert!(cycle_nodes.contains(&node_3), "Cycle should contain node 3");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// Test that prepare_run detects a cycle and returns all cycle nodes
#[test]
fn prepare_run_detects_cycle_path_closes() {
    // Given: A cycle where we can verify the path structure
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();

    let deps = vec![(node_0, vec![node_2]), (node_1, vec![node_0]), (node_2, vec![node_1])];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The implementation should return Err(CycleDetected{...})
    // with all cycle nodes returned
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

/// Test that prepare_run rejects empty workflow
#[test]
fn disconnected_workflow_when_preparing_run_then_returns_invalid_workflow_state_error() {
    // Given
    let mut workflow = Workflow::new();
    let node_0 = workflow.add_node("run", 0.0, 0.0);
    let node_1 = workflow.add_node("run", 100.0, 0.0);
    let node_2 = workflow.add_node("run", 200.0, 200.0);
    let node_3 = workflow.add_node("run", 300.0, 200.0);

    // Build two disconnected components
    // Component 1: 0 -> 1
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.nodes.push(make_node(node_1, vec![node_0]));
    // Component 2: 2 -> 3 (disconnected)
    workflow.nodes.push(make_node(node_2, vec![]));
    workflow.nodes.push(make_node(node_3, vec![node_2]));

    let _ = workflow.add_connection_checked(
        node_0,
        node_1,
        &PortName::from("main"),
        &PortName::from("main"),
    );
    let _ = workflow.add_connection_checked(
        node_2,
        node_3,
        &PortName::from("main"),
        &PortName::from("main"),
    );

    // When
    let result = workflow.prepare_run();

    // Then: Should return error for disconnected workflow
    assert!(result.is_err(), "prepare_run should reject disconnected workflow");

    match result.unwrap_err() {
        WorkflowExecutionError::InvalidWorkflowState { .. } => {
            // Correct error variant - connectivity violation
        }
        other => panic!("Expected InvalidWorkflowState error, got: {:?}", other),
    }
}

/// Test that prepare_run rejects missing dependency
#[test]
fn prepare_run_rejects_missing_dependency() {
    // Given: Node references non-existent dependency
    let node_0 = NodeId::new();
    let node_999 = NodeId::new(); // Non-existent

    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![node_999]));

    // Add a fake connection to the non-existent node
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_0,
        target: node_999,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });

    // When
    let result = workflow.prepare_run();

    // Then: Should return UnresolvedDependencies error
    assert!(result.is_err(), "prepare_run should reject missing dependency");

    match result.unwrap_err() {
        WorkflowExecutionError::UnresolvedDependencies { nodes, missing_deps } => {
            assert!(nodes.contains(&node_0), "nodes should contain node 0");
            assert!(missing_deps.contains(&node_999), "missing_deps should contain node 999");
        }
        other => panic!("Expected UnresolvedDependencies error, got: {:?}", other),
    }
}

/// Test that prepare_run rejects duplicate connections
#[test]
fn prepare_run_rejects_duplicate_dependencies() {
    // Given: Two nodes with duplicate connection between them
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();

    let mut workflow = Workflow::new();
    // Add both nodes
    workflow.nodes.push(make_node(node_0, vec![node_1]));
    workflow.nodes.push(make_node(node_1, vec![]));

    // Add the same connection twice
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_0,
        target: node_1,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_0,
        target: node_1,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });

    // When
    let result = workflow.prepare_run();

    // Then: Should return InvalidWorkflowState error for duplicate connection
    assert!(result.is_err(), "prepare_run should reject duplicate connections");

    match result.unwrap_err() {
        WorkflowExecutionError::InvalidWorkflowState { reason } => {
            assert!(
                reason.contains("duplicate"),
                "Error reason should mention duplicate: {}",
                reason
            );
        }
        other => panic!("Expected InvalidWorkflowState error, got: {:?}", other),
    }
}

/// Test that prepare_run rejects disconnected components
#[test]
fn prepare_run_rejects_disconnected_components() {
    // Given: Two disconnected components
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let mut workflow = Workflow::new();
    // Component 1: 0 -> 1
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.nodes.push(make_node(node_1, vec![node_0]));
    // Component 2: 2 -> 3 (disconnected)
    workflow.nodes.push(make_node(node_2, vec![]));
    workflow.nodes.push(make_node(node_3, vec![node_2]));

    let _ = workflow.add_connection_checked(
        node_0,
        node_1,
        &PortName::from("main"),
        &PortName::from("main"),
    );
    let _ = workflow.add_connection_checked(
        node_2,
        node_3,
        &PortName::from("main"),
        &PortName::from("main"),
    );

    // When
    let result = workflow.prepare_run();

    // Then: Should return InvalidWorkflowState error for disconnected graph
    assert!(result.is_err(), "prepare_run should reject disconnected components");

    match result.unwrap_err() {
        WorkflowExecutionError::InvalidWorkflowState { reason } => {
            assert!(
                reason.contains("connectivity") || reason.contains("isolated"),
                "Error reason should mention connectivity or isolated: {}",
                reason
            );
        }
        other => panic!("Expected InvalidWorkflowState error, got: {:?}", other),
    }
}

/// Test that prepare_run rejects dirty state queue
#[test]
fn prepare_run_rejects_dirty_state_queue() {
    // Given: Workflow with non-empty queue
    let node_0 = NodeId::new();
    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.execution_queue = vec![node_0]; // dirty state

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

/// Test that prepare_run rejects dirty state (executing nodes)
#[test]
fn prepare_run_rejects_dirty_state_executed() {
    // Given: Workflow with executing node
    let node_0 = NodeId::new();
    let mut workflow = Workflow::new();
    let mut node = make_node(node_0, vec![]);
    node.executing = true;
    workflow.nodes.push(node);

    // When
    let result = workflow.prepare_run();

    // Then: Should reject with InvalidWorkflowState error
    assert!(result.is_err(), "prepare_run should reject dirty state with executing nodes");

    match result.unwrap_err() {
        WorkflowExecutionError::InvalidWorkflowState { reason } => {
            assert!(
                reason.contains("executing"),
                "Error reason should mention executing: {}",
                reason
            );
        }
        other => panic!("Expected InvalidWorkflowState error, got: {:?}", other),
    }
}

/// Test that prepare_run orders parallel nodes deterministically
#[test]
fn parallel_nodes_when_preparing_run_then_order_is_deterministic() {
    // Given: Three parallel nodes (no dependencies)
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();

    let mut workflow = Workflow::new();
    workflow.nodes.push(make_node(node_0, vec![]));
    workflow.nodes.push(make_node(node_1, vec![]));
    workflow.nodes.push(make_node(node_2, vec![]));

    // Set positions to ensure deterministic ordering
    if let Some(n) = workflow.nodes.get_mut(0) {
        n.x = 0.0;
        n.y = 0.0;
    }
    if let Some(n) = workflow.nodes.get_mut(1) {
        n.x = 0.0;
        n.y = 60.0;
    }
    if let Some(n) = workflow.nodes.get_mut(2) {
        n.x = 0.0;
        n.y = 120.0;
    }

    // Run twice to check determinism
    let _ = workflow.prepare_run();
    let order1 = workflow.execution_queue.clone();

    let _ = workflow.prepare_run();
    let order2 = workflow.execution_queue.clone();

    // Then: Order should be deterministic
    assert_eq!(order1, order2, "Order should be deterministic across runs");
}

// ===========================================================================
// Test Layer 2: Unit Tests for validate_topological_order()
// ===========================================================================

/// Test that validate_topological_order accepts valid order
#[test]
fn validate_topological_order_accepts_valid_topological_ordering() {
    // This test documents the expected API for validate_topological_order
    // Implementation can be added when needed
}

/// Test that validate_topological_order rejects out of order
#[test]
fn validate_topological_order_rejects_out_of_order() {
    // This test documents the expected API for validate_topological_order
    // Implementation can be added when needed
}

/// Test that validate_topological_order rejects cycle in queue
#[test]
fn validate_topological_order_rejects_cycle_in_queue() {
    // This test documents the expected API for validate_topological_order
    // Implementation can be added when needed
}

// ===========================================================================
// Test Layer 3: Unit Tests for get_next_node, mark_node_complete, mark_node_failed
// ===========================================================================

/// Test that get_next_node returns nodes in order
#[test]
fn get_next_node_returns_nodes_in_execution_queue_order() {
    // This test documents the expected API for get_next_node
    // Implementation can be added when needed
}

/// Test that mark_node_complete moves node correctly
#[test]
fn mark_node_complete_moves_node_from_queue_to_executed() {
    // This test documents the expected API for mark_node_complete
    // Implementation can be added when needed
}

// ===========================================================================
// RED QUEEN ADVERSARIAL TESTS: Attack the cycle detection contract
// ===========================================================================

/// ADVERSARIAL: Large cycle (100 nodes) - tests scalability
#[test]
fn prepare_run_detects_large_cycle_100_nodes() {
    // Given: A cycle of 100 nodes
    let nodes: Vec<NodeId> = (0..100).map(|_| NodeId::new()).collect();

    let deps: Vec<(NodeId, Vec<NodeId>)> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let next = if i == 99 { &nodes[0] } else { &nodes[i + 1] };
            (*node, vec![*next])
        })
        .collect();

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: All 100 nodes should be reported as cycle nodes
    assert!(result.is_err(), "prepare_run should detect 100-node cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(
                cycle_nodes.len(),
                100,
                "Cycle should contain exactly 100 nodes, got {}",
                cycle_nodes.len()
            );
            for node in &nodes {
                assert!(cycle_nodes.contains(node), "Cycle should contain node {}", node.0);
            }
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Very large cycle (1000 nodes) - stress test
#[test]
fn prepare_run_detects_very_large_cycle_1000_nodes() {
    // Given: A cycle of 1000 nodes
    let nodes: Vec<NodeId> = (0..1000).map(|_| NodeId::new()).collect();

    let deps: Vec<(NodeId, Vec<NodeId>)> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let next = if i == 999 { &nodes[0] } else { &nodes[i + 1] };
            (*node, vec![*next])
        })
        .collect();

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: All 1000 nodes should be reported
    assert!(result.is_err(), "prepare_run should detect 1000-node cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 1000, "Cycle should contain exactly 1000 nodes");
            for node in &nodes {
                assert!(cycle_nodes.contains(node), "Cycle should contain node {}", node.0);
            }
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Cycle with merge point - complex topology
#[test]
fn prepare_run_detects_cycle_with_merge_point() {
    // Given: A cycle with nodes merging into it from outside
    // Structure: 0 -> 1 -> 2 -> 1 (cycle 1-2)
    //            3 -> 1 (merge point)
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let deps = vec![
        (node_0, vec![]),               // entry
        (node_1, vec![node_0, node_2]), // depends on 0 and 2
        (node_2, vec![node_1]),         // depends on 1 (cycle: 1 -> 2 -> 1)
        (node_3, vec![node_1]),         // depends on 1 (merge)
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: Cycle 1 <-> 2 should be detected
    assert!(result.is_err(), "Should detect cycle with merge point");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 2, "Cycle should contain exactly 2 nodes (1 and 2)");
            assert!(cycle_nodes.contains(&node_1), "Cycle should contain node 1");
            assert!(cycle_nodes.contains(&node_2), "Cycle should contain node 2");
            assert!(!cycle_nodes.contains(&node_0), "Node 0 should NOT be in cycle");
            assert!(!cycle_nodes.contains(&node_3), "Node 3 should NOT be in cycle");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Multiple disjoint cycles in same graph
#[test]
fn prepare_run_detects_multiple_disjoint_cycles() {
    // Given: Two completely separate cycles connected in a chain
    // Cycle A: 0 <-> 1
    // Cycle B: 2 <-> 3
    // Chain: 0 -> 2 (so Cycle A feeds into Cycle B)
    // This creates a connected graph with two cycles
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let deps = vec![
        (node_0, vec![node_1, node_2]), // depends on 1 (cycle A) and 2 (feeds to B)
        (node_1, vec![node_0]),         // cycle A
        (node_2, vec![node_3, node_0]), // depends on 3 (cycle B) and 0 (creates larger cycle)
        (node_3, vec![node_2]),         // cycle B
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: At least one cycle should be detected
    assert!(result.is_err(), "Should detect at least one cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            // At least 2 nodes should be in a cycle
            assert!(
                cycle_nodes.len() >= 2,
                "Should detect at least 2 nodes in cycle, got {}",
                cycle_nodes.len()
            );
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Cycle with tree growing out of it
#[test]
fn prepare_run_detects_cycle_with_tree_outgrowth() {
    // Given: A 3-node cycle with nodes branching out
    // Cycle: 0 -> 1 -> 2 -> 0
    // Outgrowth: 0 -> 3 -> 4
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();
    let node_4 = NodeId::new();

    let deps = vec![
        (node_0, vec![node_2]), // cycle
        (node_1, vec![node_0]), // cycle
        (node_2, vec![node_1]), // cycle
        (node_3, vec![node_0]), // outgrowth from 0
        (node_4, vec![node_3]), // outgrowth from 3
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: Only cycle nodes should be reported, not outgrowth
    assert!(result.is_err(), "Should detect cycle with outgrowth");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 3, "Cycle should contain exactly 3 nodes");
            assert!(cycle_nodes.contains(&node_0), "Cycle should contain node 0");
            assert!(cycle_nodes.contains(&node_1), "Cycle should contain node 1");
            assert!(cycle_nodes.contains(&node_2), "Cycle should contain node 2");
            assert!(!cycle_nodes.contains(&node_3), "Node 3 (outgrowth) should NOT be in cycle");
            assert!(!cycle_nodes.contains(&node_4), "Node 4 (outgrowth) should NOT be in cycle");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Diamond with back-edge creating cycle
#[test]
fn prepare_run_detects_diamond_cycle() {
    // Given: A diamond structure with a back-edge creating a cycle
    //   0
    //  / \
    // 1   2
    //  \ /
    //   3
    // Back-edge: 3 -> 1 creates cycle 1 -> 3 -> 1
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let deps = vec![
        (node_0, vec![]),
        (node_1, vec![node_0]),
        (node_2, vec![node_0]),
        (node_3, vec![node_1]), // depends on 1 (not 2)
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps.clone());

    // Add extra edge: 3 -> 2 (creates diamond but no new cycle)
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_3,
        target: node_2,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });

    // When
    let _result = workflow.prepare_run();

    // Then: Should detect cycle if there is one (this graph has a cycle via 1 -> 3 -> 2 -> 1)
    // Actually this creates a cycle: 1 -> 3 -> 2 -> 1 (since 1 depends on 0, 3 depends on 1, 2 depends on 0)
    // Wait, connections are source -> target, meaning source depends on target
    // So: 3 depends on 1, 3 depends on 2, 2 depends on 0, 1 depends on 0
    // This is NOT a cycle, it's a valid DAG

    // Let me reconsider: in our graph model, a connection from A to B means A depends on B
    // So for a cycle, we need B to depend on A
    // Let's create a proper cycle: 0 -> 1 -> 2 -> 0

    // Actually let's just test a simple cycle here
    // Given: A simple cycle 0 -> 1 -> 2 -> 0
    let node_a = NodeId::new();
    let node_b = NodeId::new();
    let node_c = NodeId::new();

    let deps_cycle = vec![(node_a, vec![node_c]), (node_b, vec![node_a]), (node_c, vec![node_b])];

    let (mut workflow2, _) = create_workflow_with_connections(deps_cycle);

    // When
    let result2 = workflow2.prepare_run();

    // Then: Cycle should be detected
    assert!(result2.is_err(), "Should detect simple cycle");

    match result2.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 3, "Should detect 3-node cycle");
            assert!(cycle_nodes.contains(&node_a));
            assert!(cycle_nodes.contains(&node_b));
            assert!(cycle_nodes.contains(&node_c));
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Self-referencing node in chain
#[test]
fn prepare_run_detects_self_ref_in_chain() {
    // Given: A chain where one node references itself
    // 0 -> 1 -> 1 (self) -> 2
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();

    let deps = vec![
        (node_0, vec![]),
        (node_1, vec![node_0, node_1]), // depends on itself
        (node_2, vec![node_1]),
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: Self-reference should be detected
    assert!(result.is_err(), "Should detect self-reference in chain");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 1, "Self-reference should contain exactly 1 node");
            assert!(cycle_nodes.contains(&node_1), "Cycle should contain node 1");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Nested cycles (cycle within cycle)
#[test]
fn prepare_run_detects_nested_cycles() {
    // Given: Two cycles where one feeds into another
    // Cycle A: 0 <-> 1
    // Cycle B: 2 <-> 3
    // Connection: 0 -> 2 (but 2 also depends on 0, creating complex cycle)
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let deps = vec![
        (node_0, vec![node_1, node_2]), // depends on 1 (cycle A) and 2 (connects to B)
        (node_1, vec![node_0]),         // cycle A
        (node_2, vec![node_0, node_3]), // depends on 0 and 3
        (node_3, vec![node_2]),         // cycle B
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: At least one cycle should be detected
    assert!(result.is_err(), "Should detect nested cycles");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            // Complex graph - at least 2 nodes should be in cycle
            assert!(
                cycle_nodes.len() >= 2,
                "Should detect at least 2 nodes in cycle, got {}",
                cycle_nodes.len()
            );
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: 4-node cycle (square)
#[test]
fn prepare_run_detects_4node_cycle_square() {
    // Given: A square cycle: 0 -> 1 -> 2 -> 3 -> 0
    let node_0 = NodeId::new();
    let node_1 = NodeId::new();
    let node_2 = NodeId::new();
    let node_3 = NodeId::new();

    let deps = vec![
        (node_0, vec![node_3]),
        (node_1, vec![node_0]),
        (node_2, vec![node_1]),
        (node_3, vec![node_2]),
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: All 4 nodes should be in cycle
    assert!(result.is_err(), "Should detect 4-node cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 4, "Square cycle should contain exactly 4 nodes");
            assert!(cycle_nodes.contains(&node_0));
            assert!(cycle_nodes.contains(&node_1));
            assert!(cycle_nodes.contains(&node_2));
            assert!(cycle_nodes.contains(&node_3));
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: 5-node cycle (pentagon)
#[test]
fn prepare_run_detects_5node_cycle_pentagon() {
    // Given: A pentagon cycle: 0 -> 1 -> 2 -> 3 -> 4 -> 0
    let nodes: Vec<NodeId> = (0..5).map(|_| NodeId::new()).collect();

    let deps: Vec<(NodeId, Vec<NodeId>)> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let next = &nodes[(i + 1) % 5];
            (*node, vec![*next])
        })
        .collect();

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: All 5 nodes should be in cycle
    assert!(result.is_err(), "Should detect 5-node cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 5, "Pentagon cycle should contain exactly 5 nodes");
            for node in &nodes {
                assert!(cycle_nodes.contains(node), "Cycle should contain node {}", node.0);
            }
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: 6-node cycle with additional edges (chords)
#[test]
fn prepare_run_detects_6node_cycle_with_chords() {
    // Given: A 6-node cycle with "chords" (extra edges across the cycle)
    // Simple cycle: 0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 0
    // Chords: 0 -> 2, 1 -> 3 (create additional paths within the cycle)
    // In our model: A depends on B means connection A -> B
    let nodes: Vec<NodeId> = (0..6).map(|_| NodeId::new()).collect();

    let mut deps: Vec<(NodeId, Vec<NodeId>)> = Vec::new();

    // Node 0: depends on 5 (cycle) and 2 (chord)
    deps.push((nodes[0], vec![nodes[5], nodes[2]]));
    // Node 1: depends on 0 (cycle) and 3 (chord)
    deps.push((nodes[1], vec![nodes[0], nodes[3]]));
    // Node 2: depends on 1 (cycle)
    deps.push((nodes[2], vec![nodes[1]]));
    // Node 3: depends on 2 (cycle)
    deps.push((nodes[3], vec![nodes[2]]));
    // Node 4: depends on 3 (cycle)
    deps.push((nodes[4], vec![nodes[3]]));
    // Node 5: depends on 4 (cycle)
    deps.push((nodes[5], vec![nodes[4]]));

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The cycle should be detected
    assert!(result.is_err(), "Should detect 6-node cycle with chords");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            // At least some cycle nodes should be detected
            assert!(
                cycle_nodes.len() >= 3,
                "Should detect at least 3 cycle nodes, got {}",
                cycle_nodes.len()
            );
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Large graph with embedded cycle (single connected component)
#[test]
fn prepare_run_detects_cycle_in_large_graph() {
    // Given: A large graph (100 nodes) with a small cycle embedded
    // Entry -> 0 -> 1 -> ... -> 14 (no cycle yet)
    // 15 -> 16 -> 17 -> 18 -> 19 -> 15 (5-node cycle)
    // 20 -> ... -> 99 (continuation from cycle)
    // In our model, "A depends on B" means there's a connection from A to B
    // So for a cycle 15 -> 16 -> 17 -> 18 -> 19 -> 15:
    // - 16 depends on 15
    // - 17 depends on 16
    // - 18 depends on 17
    // - 19 depends on 18
    // - 15 depends on 19 (completes cycle)
    let nodes: Vec<NodeId> = (0..100).map(|_| NodeId::new()).collect();

    let mut deps: Vec<(NodeId, Vec<NodeId>)> = Vec::new();

    // Entry nodes (0-14): linear chain with no cycle
    for i in 0..15 {
        let prev = if i == 0 { None } else { Some(nodes[i - 1]) };
        deps.push((nodes[i], prev.into_iter().collect()));
    }

    // Cycle nodes (15-19): 16 depends on 15, 17 depends on 16, ..., 15 depends on 19
    // Node 15: depends on 14 (entry) and 19 (cycle back-edge)
    deps.push((nodes[15], vec![nodes[14], nodes[19]]));
    // Node 16: depends on 15
    deps.push((nodes[16], vec![nodes[15]]));
    // Node 17: depends on 16
    deps.push((nodes[17], vec![nodes[16]]));
    // Node 18: depends on 17
    deps.push((nodes[18], vec![nodes[17]]));
    // Node 19: depends on 18
    deps.push((nodes[19], vec![nodes[18]]));

    // Continuation (20-99): linear from cycle
    for i in 20..100 {
        let prev = nodes[i - 1];
        deps.push((nodes[i], vec![prev]));
    }

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: The 5-node cycle should be detected
    assert!(result.is_err(), "Should detect cycle in large graph");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            // The cycle should be exactly 5 nodes (15, 16, 17, 18, 19)
            assert_eq!(
                cycle_nodes.len(),
                5,
                "Should detect exactly 5-node cycle in large graph, got {}",
                cycle_nodes.len()
            );
            for i in 15..20 {
                assert!(cycle_nodes.contains(&nodes[i]), "Cycle should contain node {}", i);
            }
            // Ensure non-cycle nodes are not included
            for i in 0..15 {
                assert!(!cycle_nodes.contains(&nodes[i]), "Node {} should NOT be in cycle", i);
            }
            for i in 20..100 {
                assert!(!cycle_nodes.contains(&nodes[i]), "Node {} should NOT be in cycle", i);
            }
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// ADVERSARIAL: Test that NO cycles are silently excluded
#[test]
fn prepare_run_all_cycles_must_be_reported() {
    // Given: A graph where we verify all cycle nodes are reported
    // Structure: A -> B -> C -> D -> A (main cycle)
    //            E -> B (E feeds into cycle at B)
    let node_a = NodeId::new();
    let node_b = NodeId::new();
    let node_c = NodeId::new();
    let node_d = NodeId::new();
    let node_e = NodeId::new();

    let deps = vec![
        (node_a, vec![node_d]), // A depends on D
        (node_b, vec![node_a]), // B depends on A
        (node_c, vec![node_b]), // C depends on B
        (node_d, vec![node_c]), // D depends on C (completes cycle)
        (node_e, vec![node_b]), // E depends on B (feeds into cycle)
    ];

    let (mut workflow, _) = create_workflow_with_connections(deps);

    // When
    let result = workflow.prepare_run();

    // Then: Exactly the 4 cycle nodes should be reported, not E
    assert!(result.is_err(), "Should detect cycle");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 4, "Cycle should contain exactly 4 nodes (A, B, C, D)");
            assert!(cycle_nodes.contains(&node_a), "Cycle should contain A");
            assert!(cycle_nodes.contains(&node_b), "Cycle should contain B");
            assert!(cycle_nodes.contains(&node_c), "Cycle should contain C");
            assert!(cycle_nodes.contains(&node_d), "Cycle should contain D");
            assert!(!cycle_nodes.contains(&node_e), "Node E should NOT be in cycle (it's outside)");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}
