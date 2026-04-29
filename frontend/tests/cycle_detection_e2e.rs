//! E2E tests for cycle detection and topological execution.
//!
//! These tests simulate user-facing scenarios with realistic workflows.
//!
//! RED PHASE: Tests assert CORRECT expected behavior. Tests FAIL because
//! the implementation is buggy (silently excludes cycles).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{NodeId, PortName, Workflow};

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

// Helper to create a workflow with nodes and connections
fn create_workflow_with_connections(deps: Vec<(NodeId, Vec<NodeId>)>) -> Workflow {
    let mut workflow = Workflow::new();

    for (id, node_deps) in &deps {
        let node = make_node(*id, node_deps.clone());
        workflow.nodes.push(node);
        for target in node_deps {
            let _ = workflow.add_connection_checked(
                *id,
                *target,
                &PortName::from("main"),
                &PortName::from("main"),
            );
        }
    }

    workflow
}

// ===========================================================================
// E2E Tests: User-Facing Scenarios
// ===========================================================================

/// E2E: Workflow with cycle reports error not silent failure
#[test]
fn e2e_workflow_with_cycle_reports_error_not_silent_failure() {
    // Given: A user-created workflow with an accidental cycle
    // Scenario: User creates a circular dependency: A -> B -> C -> A
    // This means: A depends on B, B depends on C, C depends on A
    // No node can run first because each one waits for another

    use oya_frontend::graph::WorkflowExecutionError;

    let node_a = NodeId::new();
    let node_b = NodeId::new();
    let node_c = NodeId::new();

    let mut workflow = Workflow::new();
    // Create actual cycle: A depends on B, B depends on C, C depends on A
    workflow.nodes.push(make_node(node_a, vec![node_b])); // A needs B
    workflow.nodes.push(make_node(node_b, vec![node_c])); // B needs C
    workflow.nodes.push(make_node(node_c, vec![node_a])); // C needs A (cycle!)

    // Manually add connections to create a cycle (bypassing validation)
    // This simulates a workflow that was loaded from a file with existing cycles
    use oya_frontend::graph::Connection;
    use uuid::Uuid;

    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_b,
        target: node_a,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    }); // B -> A
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_c,
        target: node_b,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    }); // C -> B
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_a,
        target: node_c,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    }); // A -> C (completes cycle)

    // When: User tries to run the workflow
    let result = workflow.prepare_run();

    // Then: The implementation should detect the cycle and report it with error metadata
    // NOT silently exclude the nodes
    assert!(result.is_err(), "prepare_run should return Err for cyclic workflow");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            assert_eq!(cycle_nodes.len(), 3, "Cycle should contain exactly 3 nodes");
            assert!(cycle_nodes.contains(&node_a), "Cycle should contain node A");
            assert!(cycle_nodes.contains(&node_b), "Cycle should contain node B");
            assert!(cycle_nodes.contains(&node_c), "Cycle should contain node C");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

/// E2E: Workflow without cycles completes successfully
#[test]
fn e2e_workflow_without_cycles_completes_successfully() {
    // Given: A legitimate workflow pipeline
    // Scenario: Data ingestion -> Transformation -> Storage

    let ingest = NodeId::new();
    let transform = NodeId::new();
    let store = NodeId::new();

    let deps = vec![
        (ingest, vec![]),          // Entry point
        (transform, vec![ingest]), // Depends on ingest
        (store, vec![transform]),  // Depends on transform
    ];

    let mut workflow = create_workflow_with_connections(deps);

    // When: User runs the workflow
    let _ = workflow.prepare_run();

    // Then: All nodes should be in the execution queue
    // RED PHASE: This test PASSES because the implementation works correctly for DAGs
    assert_eq!(workflow.execution_queue.len(), 3, "All 3 nodes should be in queue");
    assert!(workflow.execution_queue.contains(&ingest), "Ingest should be in queue");
    assert!(workflow.execution_queue.contains(&transform), "Transform should be in queue");
    assert!(workflow.execution_queue.contains(&store), "Store should be in queue");
}

/// E2E: Workflow with partial cycle reports exact cycle nodes
#[test]
fn e2e_workflow_with_partial_cycle_reports_exact_cycle_nodes() {
    // Given: A workflow where some nodes are in a cycle but others are not
    // Scenario: Node A is independent, but B -> C -> D -> B is a cycle
    // Cycle: B depends on C, C depends on D, D depends on B

    use oya_frontend::graph::WorkflowExecutionError;

    let node_a = NodeId::new();
    let node_b = NodeId::new();
    let node_c = NodeId::new();
    let node_d = NodeId::new();

    let mut workflow = Workflow::new();
    // Node A is independent (entry)
    workflow.nodes.push(make_node(node_a, vec![]));
    // Node B depends on A (B needs A to complete first)
    workflow.nodes.push(make_node(node_b, vec![node_a]));
    // Cycle: C depends on B, D depends on C, B depends on D
    workflow.nodes.push(make_node(node_c, vec![node_b])); // C needs B
    workflow.nodes.push(make_node(node_d, vec![node_c])); // D needs C

    // Manually add connections to create the graph (including cycle)
    use oya_frontend::graph::Connection;
    use uuid::Uuid;

    // A -> B (A must complete before B)
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_a,
        target: node_b,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });
    // B -> C
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_b,
        target: node_c,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });
    // C -> D
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_c,
        target: node_d,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });
    // D -> B (completes the cycle: B -> C -> D -> B)
    workflow.connections.push(Connection {
        id: Uuid::new_v4(),
        source: node_d,
        target: node_b,
        source_port: PortName::from("main"),
        target_port: PortName::from("main"),
    });

    // When: User tries to run
    let result = workflow.prepare_run();

    // Then: The implementation should detect the cycle and report it
    // NOT silently exclude the cycle nodes
    //
    // Correct behavior (what the test asserts):
    // - prepare_run should return Err(CycleDetected{cycle_nodes: [B, C, D], ...})
    // - No nodes should be silently excluded
    //
    // Note: We don't support partial execution - all nodes must be accounted for
    assert!(result.is_err(), "prepare_run should return Err for cyclic workflow");

    match result.unwrap_err() {
        WorkflowExecutionError::CycleDetected { cycle_nodes } => {
            // Nodes B, C, D form the cycle
            assert!(cycle_nodes.contains(&node_b), "Cycle should contain node B");
            assert!(cycle_nodes.contains(&node_c), "Cycle should contain node C");
            assert!(cycle_nodes.contains(&node_d), "Cycle should contain node D");
            // Node A should NOT be in the cycle
            assert!(!cycle_nodes.contains(&node_a), "Node A should not be in cycle");
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}
