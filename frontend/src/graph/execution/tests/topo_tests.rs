use super::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// build_execution_queue — topological ordering
// ---------------------------------------------------------------------------

#[test]
fn given_linear_chain_when_building_queue_then_order_follows_dependencies() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    let c = workflow.add_node("run", 20.0, 0.0);

    add_connection(&mut workflow, a, b);
    add_connection(&mut workflow, b, c);

    let queue = prepare_and_get_queue(&mut workflow);
    let order = queue.expect("linear chain should produce a valid queue");

    let pos_a = order.iter().position(|&id| id == a).expect("a in queue");
    let pos_b = order.iter().position(|&id| id == b).expect("b in queue");
    let pos_c = order.iter().position(|&id| id == c).expect("c in queue");

    assert!(pos_a < pos_b, "a must come before b: {pos_a} < {pos_b}");
    assert!(pos_b < pos_c, "b must come before c: {pos_b} < {pos_c}");
}

#[test]
fn given_diamond_dependency_when_building_queue_then_converging_nodes_preserve_ordering() {
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    let c = workflow.add_node("run", 10.0, 100.0);
    let d = workflow.add_node("run", 20.0, 0.0);

    add_connection(&mut workflow, a, b);
    add_connection(&mut workflow, a, c);
    add_connection(&mut workflow, b, d);
    add_connection(&mut workflow, c, d);

    let queue = prepare_and_get_queue(&mut workflow);
    let order = queue.expect("diamond should produce a valid queue");

    let pos_a = order.iter().position(|&id| id == a).expect("a");
    let pos_b = order.iter().position(|&id| id == b).expect("b");
    let pos_c = order.iter().position(|&id| id == c).expect("c");
    let pos_d = order.iter().position(|&id| id == d).expect("d");

    // A must come before B and C; B and C must come before D
    assert!(pos_a < pos_b, "a before b");
    assert!(pos_a < pos_c, "a before c");
    assert!(pos_b < pos_d, "b before d");
    assert!(pos_c < pos_d, "c before d");
}

#[test]
fn given_cycle_when_building_queue_then_cycle_detected_error_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);

    // A -> B -> A  (cycle)
    add_connection(&mut workflow, a, b);
    add_connection(&mut workflow, b, a);

    let result = prepare_and_get_queue(&mut workflow);
    assert!(
        matches!(result, Err(WorkflowExecutionError::CycleDetected { .. })),
        "expected CycleDetected, got {result:?}"
    );
}

#[test]
fn given_self_loop_when_preparing_run_then_cycle_detected_error_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    add_connection(&mut workflow, a, a);

    let result = prepare_and_get_queue(&mut workflow);
    assert!(
        matches!(result, Err(WorkflowExecutionError::CycleDetected { .. })),
        "self-loop should be detected as cycle, got {result:?}"
    );
}

#[test]
fn given_empty_workflow_when_preparing_run_then_empty_workflow_error_is_returned() {
    let mut workflow = Workflow::new();
    let result = workflow.prepare_run();

    assert_eq!(result, Err(WorkflowExecutionError::EmptyWorkflow));
}

#[test]
fn given_single_node_when_building_queue_then_queue_contains_that_node() {
    let mut workflow = Workflow::new();
    let node = workflow.add_node("run", 0.0, 0.0);

    let queue = prepare_and_get_queue(&mut workflow);
    let order = queue.expect("single node should succeed");

    assert_eq!(order.len(), 1);
    assert_eq!(order[0], node);
}

#[test]
fn given_multiple_roots_when_building_queue_then_all_roots_appear_before_dependents() {
    // Two independent roots feeding into a shared sink.
    //   A  B
    //    \ /
    //     C
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    let c = workflow.add_node("run", 20.0, 0.0);

    add_connection(&mut workflow, a, c);
    add_connection(&mut workflow, b, c);

    // Directly call build_execution_queue (bypasses connectivity check
    // since A and B are both roots with no path between them).
    let order = workflow.build_execution_queue().expect("multiple roots should succeed");

    let pos_a = order.iter().position(|&id| id == a).expect("a");
    let pos_b = order.iter().position(|&id| id == b).expect("b");
    let pos_c = order.iter().position(|&id| id == c).expect("c");

    assert!(pos_a < pos_c, "a before c");
    assert!(pos_b < pos_c, "b before c");
    assert_eq!(order.len(), 3);
}

// ---------------------------------------------------------------------------
// build_execution_queue — priority ordering (x coordinate, then name)
// ---------------------------------------------------------------------------

#[test]
fn given_two_root_nodes_when_building_queue_then_lower_x_comes_first() {
    let mut workflow = Workflow::new();
    let right = workflow.add_node("run", 100.0, 0.0);
    let left = workflow.add_node("run", 10.0, 0.0);
    // No connections — both are roots, so ordering is by priority.
    // Use build_execution_queue directly (bypasses connectivity check).

    let order = workflow.build_execution_queue().expect("two roots should succeed");

    let pos_left = order.iter().position(|&id| id == left).expect("left");
    let pos_right = order.iter().position(|&id| id == right).expect("right");

    assert!(pos_left < pos_right, "node with lower x should come first: left@10 vs right@100");
}

// ---------------------------------------------------------------------------
// compare_execution_priority — edge cases
// ---------------------------------------------------------------------------

#[test]
fn given_unknown_node_ids_when_comparing_priority_then_ordering_is_equal() {
    let node_map: HashMap<NodeId, &super::super::super::Node> = HashMap::new();
    let a = NodeId::new();
    let b = NodeId::new();

    let ordering = Workflow::compare_execution_priority(&node_map, a, b);
    assert_eq!(ordering, std::cmp::Ordering::Equal);
}

#[test]
fn given_same_x_different_y_when_comparing_priority_then_lower_y_comes_first() {
    let mut workflow = Workflow::new();
    let upper = workflow.add_node("run", 50.0, 10.0);
    let lower = workflow.add_node("run", 50.0, 90.0);

    let node_map: HashMap<NodeId, &super::super::super::Node> =
        workflow.nodes.iter().map(|n| (n.id, n)).collect();

    let ordering = Workflow::compare_execution_priority(&node_map, upper, lower);
    assert_eq!(ordering, std::cmp::Ordering::Less);
}

// ---------------------------------------------------------------------------
// find_cycle — three-node cycle
// ---------------------------------------------------------------------------

#[test]
fn given_three_node_cycle_when_preparing_run_then_cycle_detected_error_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 10.0, 0.0);
    let c = workflow.add_node("run", 20.0, 0.0);

    add_connection(&mut workflow, a, b);
    add_connection(&mut workflow, b, c);
    add_connection(&mut workflow, c, a);

    let result = workflow.prepare_run();
    assert!(
        matches!(result, Err(WorkflowExecutionError::CycleDetected { .. })),
        "three-node cycle should be detected, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Larger graph stress test
// ---------------------------------------------------------------------------

#[test]
fn given_wide_dag_when_building_queue_then_topological_order_is_valid() {
    // Build a 5-layer wide DAG where each layer fans out.
    // Layer 0: 1 node
    // Layer 1: 3 nodes
    // Layer 2: 5 nodes
    // Layer 3: 3 nodes
    // Layer 4: 1 node
    let mut workflow = Workflow::new();

    let l0 = workflow.add_node("run", 0.0, 0.0);

    let l1a = workflow.add_node("run", 10.0, 0.0);
    let l1b = workflow.add_node("run", 10.0, 100.0);
    let l1c = workflow.add_node("run", 10.0, 200.0);

    let l2a = workflow.add_node("run", 20.0, 0.0);
    let l2b = workflow.add_node("run", 20.0, 50.0);
    let l2c = workflow.add_node("run", 20.0, 100.0);
    let l2d = workflow.add_node("run", 20.0, 150.0);
    let l2e = workflow.add_node("run", 20.0, 200.0);

    let l3a = workflow.add_node("run", 30.0, 50.0);
    let l3b = workflow.add_node("run", 30.0, 150.0);
    let l3c = workflow.add_node("run", 30.0, 200.0);

    let l4 = workflow.add_node("run", 40.0, 100.0);

    // Fan out from l0 to l1
    add_connection(&mut workflow, l0, l1a);
    add_connection(&mut workflow, l0, l1b);
    add_connection(&mut workflow, l0, l1c);

    // l1 to l2
    add_connection(&mut workflow, l1a, l2a);
    add_connection(&mut workflow, l1a, l2b);
    add_connection(&mut workflow, l1b, l2c);
    add_connection(&mut workflow, l1c, l2d);
    add_connection(&mut workflow, l1c, l2e);

    // l2 to l3
    add_connection(&mut workflow, l2a, l3a);
    add_connection(&mut workflow, l2b, l3a);
    add_connection(&mut workflow, l2c, l3b);
    add_connection(&mut workflow, l2d, l3b);
    add_connection(&mut workflow, l2e, l3c);

    // l3 to l4
    add_connection(&mut workflow, l3a, l4);
    add_connection(&mut workflow, l3b, l4);
    add_connection(&mut workflow, l3c, l4);

    let queue = prepare_and_get_queue(&mut workflow);
    let order = queue.expect("wide DAG should succeed");
    assert_eq!(order.len(), 13);

    // Verify all edges respect the topological order
    let positions: HashMap<NodeId, usize> =
        order.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    for conn in &workflow.connections {
        let src_pos = positions.get(&conn.source).copied().unwrap_or(usize::MAX);
        let tgt_pos = positions.get(&conn.target).copied().unwrap_or(usize::MAX);
        assert!(
            src_pos < tgt_pos,
            "edge {:?} -> {:?}: source pos {} must be < target pos {}",
            conn.source,
            conn.target,
            src_pos,
            tgt_pos
        );
    }
}
