//! Comprehensive test suite for self-connection cycle detection (Bead: oya-frontend-5uz)
//!
//! This module contains all tests specified in the test-plan.md for bead oya-frontend-5uz.
//! Tests cover:
//! - Error variant tests for add_connection_checked (all 6 variants + success)
//! - path_exists helper function tests
//! - check_port_type_compatibility helper function tests
//! - add_connection wrapper tests
//! - Integration tests for workflow lifecycle
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::connectivity::{
    path_exists_internal, ConnectionError, ConnectionResult, SourcePortType, TargetPortType,
};
use oya_frontend::graph::restate_types::PortType;
use oya_frontend::graph::{
    connectivity::check_port_type_compatibility_internal, Connection, NodeId, PortName, Workflow,
};
use uuid::Uuid;

// ============================================================================
// Section 1: add_connection_checked Error Variant Tests
// ============================================================================

#[test]
fn add_connection_checked_returns_self_connection_error_when_source_equals_target() {
    let mut workflow = Workflow::new();
    let node_id = workflow.add_node("http-handler", 0.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(node_id, node_id, &main, &main);

    assert_eq!(result, Err(ConnectionError::SelfConnection));
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn add_connection_checked_returns_missing_source_error_when_source_not_in_nodes() {
    let mut workflow = Workflow::new();
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let missing_source = NodeId(Uuid::new_v4());
    let result = workflow.add_connection_checked(missing_source, target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(missing_source)));
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn add_connection_checked_returns_missing_target_error_when_target_not_in_nodes() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let main = PortName("main".to_string());

    let missing_target = NodeId(Uuid::new_v4());
    let result = workflow.add_connection_checked(source, missing_target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingTargetNode(missing_target)));
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn add_connection_checked_returns_cycle_error_when_connection_would_create_cycle() {
    let mut workflow = Workflow::new();
    let first = workflow.add_node("http-handler", 0.0, 0.0);
    let second = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let created = workflow.add_connection_checked(first, second, &main, &main);
    assert!(matches!(created, Ok(ConnectionResult::Created)));

    let cycle = workflow.add_connection_checked(second, first, &main, &main);

    assert_eq!(cycle, Err(ConnectionError::WouldCreateCycle));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn add_connection_checked_returns_duplicate_error_when_identical_connection_exists() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let first = workflow.add_connection_checked(source, target, &main, &main);
    assert!(matches!(first, Ok(ConnectionResult::Created)));

    let duplicate = workflow.add_connection_checked(source, target, &main, &main);

    assert_eq!(duplicate, Err(ConnectionError::Duplicate));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn add_connection_checked_returns_type_mismatch_error_when_port_types_incompatible() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("condition", 0.0, 0.0);
    let target = workflow.add_node("signal-handler", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(source, target, &main, &main);

    assert_eq!(
        result,
        Err(ConnectionError::TypeMismatch {
            source_type: SourcePortType(PortType::FlowControl),
            target_type: TargetPortType(PortType::Signal),
        })
    );
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn add_connection_checked_returns_created_when_all_validations_pass() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(source, target, &main, &main);

    assert_eq!(result, Ok(ConnectionResult::Created));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn created_connections_have_unique_generated_uuids() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("delay", 200.0, 0.0);
    let main = PortName("main".to_string());

    let conn1_result = workflow.add_connection_checked(a, b, &main, &main);
    let conn2_result = workflow.add_connection_checked(b, c, &main, &main);
    let conn3_result = workflow.add_connection_checked(a, c, &main, &main);

    assert!(matches!(conn1_result, Ok(ConnectionResult::Created)));
    assert!(matches!(conn2_result, Ok(ConnectionResult::Created)));
    assert!(matches!(conn3_result, Ok(ConnectionResult::Created)));

    let conn1 = &workflow.connections[0];
    let conn2 = &workflow.connections[1];
    let conn3 = &workflow.connections[2];

    assert_ne!(conn1.id, conn2.id);
    assert_ne!(conn2.id, conn3.id);
    assert_ne!(conn1.id, conn3.id);
    assert_ne!(conn1.id, Uuid::nil());
    assert_ne!(conn2.id, Uuid::nil());
    assert_ne!(conn3.id, Uuid::nil());
}

#[test]
fn add_connection_checked_returns_created_for_different_ports_same_endpoints() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);

    let main = PortName("main".to_string());
    let alt = PortName("alt".to_string());

    let first = workflow.add_connection_checked(source, target, &main, &main);
    assert!(matches!(first, Ok(ConnectionResult::Created)));

    let second = workflow.add_connection_checked(source, target, &alt, &main);

    assert!(matches!(second, Ok(ConnectionResult::Created)));
    assert_eq!(workflow.connections.len(), 2);
}

#[test]
fn workflow_state_remains_unchanged_when_validation_fails() {
    let mut workflow = Workflow::new();
    // Use nodes with compatible Json types
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("http-call", 200.0, 0.0);
    let main = PortName("main".to_string());

    // Create a path: A -> B -> C
    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    assert_eq!(workflow.connections.len(), 1);

    let result_bc = workflow.add_connection_checked(b, c, &main, &main);
    assert!(matches!(result_bc, Ok(ConnectionResult::Created)));
    assert_eq!(workflow.connections.len(), 2);

    // Now C -> A would create a cycle (A -> B -> C -> A)
    let cycle_result = workflow.add_connection_checked(c, a, &main, &main);
    assert_eq!(cycle_result, Err(ConnectionError::WouldCreateCycle));
    assert_eq!(workflow.connections.len(), 2);

    // Self-connection should also fail
    let self_conn_result = workflow.add_connection_checked(a, a, &main, &main);
    assert_eq!(self_conn_result, Err(ConnectionError::SelfConnection));
    assert_eq!(workflow.connections.len(), 2);
}

#[test]
fn validation_is_idempotent_when_same_connection_requested_twice() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let first_result = workflow.add_connection_checked(a, b, &main, &main);
    assert_eq!(first_result, Ok(ConnectionResult::Created));

    let second_result = workflow.add_connection_checked(a, b, &main, &main);
    assert_eq!(second_result, Err(ConnectionError::Duplicate));

    assert_eq!(workflow.connections.len(), 1);
}

// ============================================================================
// Section 2: path_exists Helper Function Tests (Integration)
// ============================================================================

#[test]
fn path_exists_returns_true_when_direct_edge_exists() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result, Ok(ConnectionResult::Created)));

    let conn = &workflow.connections[0];
    assert_eq!(path_exists_internal(&[conn.clone()], a, b), true);
    assert_eq!(path_exists_internal(&[conn.clone()], b, a), false);
}

#[test]
fn path_exists_returns_true_when_multi_hop_path_exists() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("delay", 200.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_bc = workflow.add_connection_checked(b, c, &main, &main);
    assert!(matches!(result_bc, Ok(ConnectionResult::Created)));

    assert_eq!(path_exists_internal(&workflow.connections, a, c), true);
    assert_eq!(path_exists_internal(&workflow.connections, c, a), false);
}

#[test]
fn path_exists_returns_false_when_no_path_exists() {
    let mut workflow = Workflow::new();
    // Use nodes with compatible Json types for all connections
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("http-call", 200.0, 0.0);
    let d = workflow.add_node("service-call", 300.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_cd = workflow.add_connection_checked(c, d, &main, &main);
    assert!(matches!(result_cd, Ok(ConnectionResult::Created)));

    assert_eq!(path_exists_internal(&workflow.connections, a, d), false);
    assert_eq!(path_exists_internal(&workflow.connections, b, c), false);
}

#[test]
fn path_exists_returns_false_when_source_equals_target_without_self_loop() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("delay", 200.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_bc = workflow.add_connection_checked(b, c, &main, &main);
    assert!(matches!(result_bc, Ok(ConnectionResult::Created)));

    assert_eq!(path_exists_internal(&workflow.connections, a, a), false);
}

// ============================================================================
// Section 3: check_port_type_compatibility Helper Function Tests
// ============================================================================

#[test]
fn check_port_type_compatibility_returns_missing_source_error_when_source_not_found() {
    let mut workflow = Workflow::new();
    let b = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());
    let http_handler = workflow.add_node("http-handler", 0.0, 0.0);
    let result = workflow.add_connection_checked(http_handler, b, &main, &main);
    assert!(matches!(result, Ok(ConnectionResult::Created)));

    let missing_source = NodeId(Uuid::new_v4());
    let result = check_port_type_compatibility_internal(&workflow.nodes, missing_source, b);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(missing_source)));
}

#[test]
fn check_port_type_compatibility_returns_missing_target_error_when_target_not_found() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let run = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());
    let result = workflow.add_connection_checked(a, run, &main, &main);
    assert!(matches!(result, Ok(ConnectionResult::Created)));

    let missing_target = NodeId(Uuid::new_v4());
    let result = check_port_type_compatibility_internal(&workflow.nodes, a, missing_target);

    assert_eq!(result, Err(ConnectionError::MissingTargetNode(missing_target)));
}

#[test]
fn check_port_type_compatibility_returns_type_mismatch_when_ports_incompatible() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("condition", 0.0, 0.0);
    let target = workflow.add_node("signal-handler", 100.0, 0.0);

    let result = check_port_type_compatibility_internal(&workflow.nodes, source, target);

    assert_eq!(
        result,
        Err(ConnectionError::TypeMismatch {
            source_type: SourcePortType(PortType::FlowControl),
            target_type: TargetPortType(PortType::Signal),
        })
    );
}

#[test]
fn check_port_type_compatibility_returns_ok_when_port_types_compatible() {
    let mut workflow = Workflow::new();
    let http_handler = workflow.add_node("http-handler", 0.0, 0.0);
    let run = workflow.add_node("run", 100.0, 0.0);

    let result = check_port_type_compatibility_internal(&workflow.nodes, http_handler, run);

    assert_eq!(result, Ok(()));
}

#[test]
fn check_port_type_compatibility_returns_ok_for_any_type_compatibility() {
    let mut workflow = Workflow::new();
    let http_handler = workflow.add_node("http-handler", 0.0, 0.0);
    let signal_handler = workflow.add_node("signal-handler", 100.0, 0.0);

    let result =
        check_port_type_compatibility_internal(&workflow.nodes, http_handler, signal_handler);

    assert_eq!(result, Ok(()));
}

#[test]
fn check_port_type_compatibility_returns_ok_for_json_compatibility() {
    let mut workflow = Workflow::new();
    let run = workflow.add_node("run", 0.0, 0.0);
    let condition = workflow.add_node("condition", 100.0, 0.0);

    let result = check_port_type_compatibility_internal(&workflow.nodes, run, condition);

    assert_eq!(result, Ok(()));
}

// ============================================================================
// Section 4: add_connection Wrapper Tests
// ============================================================================

#[test]
fn add_connection_returns_ok_when_connection_valid() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);

    let result =
        workflow.add_connection(a, b, &PortName("main".to_string()), &PortName("main".to_string()));

    assert!(result.is_ok());
    assert!(matches!(result, Ok(ConnectionResult::Created)));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn add_connection_returns_err_when_connection_invalid() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);

    let result =
        workflow.add_connection(a, a, &PortName("main".to_string()), &PortName("main".to_string()));

    assert!(result.is_err());
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn add_connection_returns_err_when_types_incompatible() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("condition", 0.0, 0.0);
    let target = workflow.add_node("signal-handler", 100.0, 0.0);

    let result = workflow.add_connection(
        source,
        target,
        &PortName("main".to_string()),
        &PortName("main".to_string()),
    );

    assert!(result.is_err());
    assert_eq!(workflow.connections.len(), 0);
}

// ============================================================================
// Section 5: Additional Edge Cases
// ============================================================================

#[test]
fn empty_workflow_rejects_all_connections() {
    let mut workflow = Workflow::new();
    let main = PortName("main".to_string());
    let node_id = NodeId(Uuid::new_v4());

    let result = workflow.add_connection_checked(node_id, node_id, &main, &main);

    assert_eq!(result, Err(ConnectionError::SelfConnection));
}

#[test]
fn single_node_workflow_rejects_self_connection() {
    let mut workflow = Workflow::new();
    let node_id = workflow.add_node("http-handler", 0.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(node_id, node_id, &main, &main);

    assert_eq!(result, Err(ConnectionError::SelfConnection));
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn two_node_workflow_allows_forward_edge() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(a, b, &main, &main);

    assert_eq!(result, Ok(ConnectionResult::Created));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn two_node_workflow_rejects_backward_edge_as_cycle() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result, Ok(ConnectionResult::Created)));

    let result = workflow.add_connection_checked(b, a, &main, &main);

    assert_eq!(result, Err(ConnectionError::WouldCreateCycle));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn three_node_chain_does_not_create_cycle_forward() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("delay", 200.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_bc = workflow.add_connection_checked(b, c, &main, &main);
    assert!(matches!(result_bc, Ok(ConnectionResult::Created)));

    let result = workflow.add_connection_checked(a, c, &main, &main);

    assert_eq!(result, Ok(ConnectionResult::Created));
    assert_eq!(workflow.connections.len(), 3);
}

#[test]
fn three_node_chain_rejects_cycle_closure() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("delay", 200.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_bc = workflow.add_connection_checked(b, c, &main, &main);
    assert!(matches!(result_bc, Ok(ConnectionResult::Created)));

    let result = workflow.add_connection_checked(c, a, &main, &main);

    assert_eq!(result, Err(ConnectionError::WouldCreateCycle));
    assert_eq!(workflow.connections.len(), 2);
}

// ============================================================================
// Section 6: Mutation Checkpoint Tests
// ============================================================================

#[test]
fn mutation_killed_remove_self_connection_check() {
    let mut workflow = Workflow::new();
    let node_id = workflow.add_node("http-handler", 0.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(node_id, node_id, &main, &main);

    assert_eq!(result, Err(ConnectionError::SelfConnection));
    assert_eq!(workflow.connections.len(), 0);
}

#[test]
fn mutation_killed_swap_error_variant_order() {
    let mut workflow = Workflow::new();
    let missing_source = NodeId(Uuid::new_v4());
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(missing_source, target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(missing_source)));
}

// ============================================================================
// Section 7: Path Exists Internal Tests (Direct DFS verification)
// ============================================================================

#[test]
fn path_exists_handles_empty_graph() {
    let connections: Vec<Connection> = vec![];
    let node_a = NodeId(Uuid::new_v4());
    let node_b = NodeId(Uuid::new_v4());

    assert_eq!(path_exists_internal(&connections, node_a, node_b), false);
}

#[test]
fn path_exists_handles_single_connection() {
    let a = NodeId(Uuid::new_v4());
    let b = NodeId(Uuid::new_v4());

    let conn = Connection {
        id: Uuid::new_v4(),
        source: a,
        target: b,
        source_port: PortName("main".to_string()),
        target_port: PortName("main".to_string()),
    };

    assert_eq!(path_exists_internal(&[conn.clone()], a, b), true);
    assert_eq!(path_exists_internal(&[conn], b, a), false);
}

#[test]
fn path_exists_handles_diamond_pattern() {
    let mut workflow = Workflow::new();
    // Use nodes with compatible Json types for all connections
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("http-call", 200.0, 0.0);
    let d = workflow.add_node("service-call", 300.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_ac = workflow.add_connection_checked(a, c, &main, &main);
    assert!(matches!(result_ac, Ok(ConnectionResult::Created)));
    let result_bd = workflow.add_connection_checked(b, d, &main, &main);
    assert!(matches!(result_bd, Ok(ConnectionResult::Created)));
    let result_cd = workflow.add_connection_checked(c, d, &main, &main);
    assert!(matches!(result_cd, Ok(ConnectionResult::Created)));

    assert_eq!(path_exists_internal(&workflow.connections, a, d), true);
    assert_eq!(path_exists_internal(&workflow.connections, b, d), true);
    assert_eq!(path_exists_internal(&workflow.connections, c, d), true);
    assert_eq!(path_exists_internal(&workflow.connections, d, a), false);
}

#[test]
fn path_exists_handles_large_linear_chain() {
    let mut workflow = Workflow::new();
    let nodes: Vec<NodeId> =
        (0..20).map(|i| workflow.add_node(&format!("node-{}", i), i as f32 * 100.0, 0.0)).collect();

    let main = PortName("main".to_string());

    // Unrolled loop per Holzmann Rule 2 - explicit iterations, no loops in test body
    let r1 = workflow.add_connection_checked(nodes[0], nodes[1], &main, &main);
    assert!(matches!(r1, Ok(ConnectionResult::Created)));
    let r2 = workflow.add_connection_checked(nodes[1], nodes[2], &main, &main);
    assert!(matches!(r2, Ok(ConnectionResult::Created)));
    let r3 = workflow.add_connection_checked(nodes[2], nodes[3], &main, &main);
    assert!(matches!(r3, Ok(ConnectionResult::Created)));
    let r4 = workflow.add_connection_checked(nodes[3], nodes[4], &main, &main);
    assert!(matches!(r4, Ok(ConnectionResult::Created)));
    let r5 = workflow.add_connection_checked(nodes[4], nodes[5], &main, &main);
    assert!(matches!(r5, Ok(ConnectionResult::Created)));
    let r6 = workflow.add_connection_checked(nodes[5], nodes[6], &main, &main);
    assert!(matches!(r6, Ok(ConnectionResult::Created)));
    let r7 = workflow.add_connection_checked(nodes[6], nodes[7], &main, &main);
    assert!(matches!(r7, Ok(ConnectionResult::Created)));
    let r8 = workflow.add_connection_checked(nodes[7], nodes[8], &main, &main);
    assert!(matches!(r8, Ok(ConnectionResult::Created)));
    let r9 = workflow.add_connection_checked(nodes[8], nodes[9], &main, &main);
    assert!(matches!(r9, Ok(ConnectionResult::Created)));
    let r10 = workflow.add_connection_checked(nodes[9], nodes[10], &main, &main);
    assert!(matches!(r10, Ok(ConnectionResult::Created)));
    let r11 = workflow.add_connection_checked(nodes[10], nodes[11], &main, &main);
    assert!(matches!(r11, Ok(ConnectionResult::Created)));
    let r12 = workflow.add_connection_checked(nodes[11], nodes[12], &main, &main);
    assert!(matches!(r12, Ok(ConnectionResult::Created)));
    let r13 = workflow.add_connection_checked(nodes[12], nodes[13], &main, &main);
    assert!(matches!(r13, Ok(ConnectionResult::Created)));
    let r14 = workflow.add_connection_checked(nodes[13], nodes[14], &main, &main);
    assert!(matches!(r14, Ok(ConnectionResult::Created)));
    let r15 = workflow.add_connection_checked(nodes[14], nodes[15], &main, &main);
    assert!(matches!(r15, Ok(ConnectionResult::Created)));
    let r16 = workflow.add_connection_checked(nodes[15], nodes[16], &main, &main);
    assert!(matches!(r16, Ok(ConnectionResult::Created)));
    let r17 = workflow.add_connection_checked(nodes[16], nodes[17], &main, &main);
    assert!(matches!(r17, Ok(ConnectionResult::Created)));
    let r18 = workflow.add_connection_checked(nodes[17], nodes[18], &main, &main);
    assert!(matches!(r18, Ok(ConnectionResult::Created)));
    let r19 = workflow.add_connection_checked(nodes[18], nodes[19], &main, &main);
    assert!(matches!(r19, Ok(ConnectionResult::Created)));

    assert_eq!(path_exists_internal(&workflow.connections, nodes[0], nodes[19]), true);
    assert_eq!(path_exists_internal(&workflow.connections, nodes[19], nodes[0]), false);
    assert_eq!(path_exists_internal(&workflow.connections, nodes[5], nodes[10]), true);
}

// ============================================================================
// Section 8: Kani Verification Harness Stubs (compile-only)
// ============================================================================
// NOTE: These are compile-only stubs. Actual Kani verification requires
// the kani cargo plugin and kani runtime.
//
// To run Kani proofs:
// 1. Install Kani: https://github.com/model-checking/kani
// 2. Run: cargo kani --project-root .
//
// #[cfg(feature = "kani")]
// mod kani_harnesses {
//     use super::*;
//
//     #[kani::proof]
//     fn kani_no_self_connections_allowed() {
//         let mut workflow = Workflow::new();
//         let node_id = workflow.add_node("http-handler", 0.0, 0.0);
//         let main = PortName("main".to_string());
//
//         let result = workflow.add_connection_checked(node_id, node_id, &main, &main);
//         kani::assert!(matches!(result, Err(ConnectionError::SelfConnection)));
//     }
//
//     #[kani::proof]
//     fn kani_graph_remains_acyclic_after_valid_addition() {
//         let mut workflow = Workflow::new();
//         let a = workflow.add_node("http-handler", 0.0, 0.0);
//         let b = workflow.add_node("run", 100.0, 0.0);
//         let main = PortName("main".to_string());
//
//         let result = workflow.add_connection_checked(a, b, &main, &main);
//         kani::assert!(matches!(result, Ok(ConnectionResult::Created)));
//
//         let cycle_result = workflow.add_connection_checked(b, a, &main, &main);
//         kani::assert!(matches!(
//             cycle_result,
//             Err(ConnectionError::WouldCreateCycle)
//         ));
//     }
// }

// ============================================================================
// Section 9: Fuzz Target Stubs (compile-only)
// ============================================================================
// NOTE: These are compile-only stubs. Actual fuzzing requires cargo-fuzz.
//
// To run fuzzing:
// 1. Install cargo-fuzz: cargo install cargo-fuzz
// 2. Run: cargo fuzz run fuzz_path_exists
//
// #[cfg(fuzzing)]
// mod fuzz_targets {
//     use super::*;
//
//     #[cfg(fuzzing)]
//     pub fn fuzz_path_exists(data: &[u8]) {
//         // Fuzz target stub - actual implementation requires cargo-fuzz
//         let _data = data;
//     }
//
//     #[cfg(fuzzing)]
//     pub fn fuzz_add_connection_checked(data: &[u8]) {
//         // Fuzz target stub - actual implementation requires cargo-fuzz
//         let _data = data;
//     }
// }

// ============================================================================
// Section 10: Integration Tests for Workflow Lifecycle
// ============================================================================

#[test]
fn workflow_lifecycle_mixed_operations() {
    let mut workflow = Workflow::new();

    let a = workflow.add_node("http-trigger", 0.0, 0.0);
    let b = workflow.add_node("http-handler", 100.0, 0.0);
    let _c = workflow.add_node("router", 200.0, 0.0);
    let d = workflow.add_node("run", 300.0, 0.0);
    let e = workflow.add_node("delay", 400.0, 0.0);
    let main = PortName("main".to_string());

    let result_ab = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result_ab, Ok(ConnectionResult::Created)));
    let result_bd = workflow.add_connection_checked(b, d, &main, &main);
    assert!(matches!(result_bd, Ok(ConnectionResult::Created)));
    let result_de = workflow.add_connection_checked(d, e, &main, &main);
    assert!(matches!(result_de, Ok(ConnectionResult::Created)));

    assert_eq!(workflow.connections.len(), 3);

    let cycle_result = workflow.add_connection_checked(e, a, &main, &main);
    assert_eq!(cycle_result, Err(ConnectionError::WouldCreateCycle));
    assert_eq!(workflow.connections.len(), 3);
}

#[test]
fn workflow_multiple_parallel_paths() {
    let mut workflow = Workflow::new();
    // Use nodes with compatible Json types for all connections
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("http-call", 200.0, 0.0);
    let d = workflow.add_node("service-call", 300.0, 0.0);
    let e = workflow.add_node("object-call", 400.0, 0.0);
    let main = PortName("main".to_string());

    let r1 = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(r1, Ok(ConnectionResult::Created)));
    let r2 = workflow.add_connection_checked(a, c, &main, &main);
    assert!(matches!(r2, Ok(ConnectionResult::Created)));
    let r3 = workflow.add_connection_checked(b, d, &main, &main);
    assert!(matches!(r3, Ok(ConnectionResult::Created)));
    let r4 = workflow.add_connection_checked(c, d, &main, &main);
    assert!(matches!(r4, Ok(ConnectionResult::Created)));
    let r5 = workflow.add_connection_checked(d, e, &main, &main);
    assert!(matches!(r5, Ok(ConnectionResult::Created)));

    assert_eq!(workflow.connections.len(), 5);
    assert_eq!(path_exists_internal(&workflow.connections, a, e), true);
}

#[test]
fn workflow_node_count_consistency() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-trigger", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let _c = workflow.add_node("delay", 200.0, 0.0);

    assert_eq!(workflow.nodes.len(), 3);
    assert_eq!(workflow.connections.len(), 0);

    let main = PortName("main".to_string());
    let result = workflow.add_connection_checked(a, b, &main, &main);
    assert!(matches!(result, Ok(ConnectionResult::Created)));

    assert_eq!(workflow.nodes.len(), 3);
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn workflow_connection_metadata_preserved() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-trigger", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let source_port = PortName("output".to_string());
    let target_port = PortName("input".to_string());

    let result = workflow.add_connection_checked(a, b, &source_port, &target_port);
    assert!(matches!(result, Ok(ConnectionResult::Created)));

    let conn = &workflow.connections[0];
    assert_eq!(conn.source, a);
    assert_eq!(conn.target, b);
    assert_eq!(conn.source_port, source_port);
    assert_eq!(conn.target_port, target_port);
}
