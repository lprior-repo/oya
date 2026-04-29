#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use super::super::*;
use crate::graph::PortName;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// add_connection_checked — core validation tests
// ---------------------------------------------------------------------------

#[test]
fn given_self_connection_when_adding_checked_connection_then_self_connection_error_is_returned() {
    let mut workflow = Workflow::new();
    let node_id = workflow.add_node("http-handler", 0.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(node_id, node_id, &main, &main);

    assert_eq!(result, Err(ConnectionError::SelfConnection));
}

#[test]
fn given_duplicate_connection_when_adding_checked_connection_then_duplicate_error_is_returned() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let first = workflow.add_connection_checked(source, target, &main, &main);
    assert!(matches!(first, Ok(ConnectionResult::Created)));

    let duplicate = workflow.add_connection_checked(source, target, &main, &main);

    assert_eq!(duplicate, Err(ConnectionError::Duplicate));
}

#[test]
fn given_back_edge_when_adding_checked_connection_then_cycle_error_is_returned() {
    let mut workflow = Workflow::new();
    let first = workflow.add_node("http-handler", 0.0, 0.0);
    let second = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let created = workflow.add_connection_checked(first, second, &main, &main);
    assert!(matches!(created, Ok(ConnectionResult::Created)));

    let cycle = workflow.add_connection_checked(second, first, &main, &main);

    assert_eq!(cycle, Err(ConnectionError::WouldCreateCycle));
}

#[test]
fn given_type_mismatch_ports_when_adding_checked_connection_then_type_mismatch_error_is_returned() {
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
}

#[test]
fn given_missing_source_when_adding_checked_connection_then_source_not_found_error_is_returned() {
    let mut workflow = Workflow::new();
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let missing_source = NodeId(Uuid::new_v4());
    let result = workflow.add_connection_checked(missing_source, target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(missing_source)));
}

#[test]
fn given_missing_target_when_adding_checked_connection_then_target_not_found_error_is_returned() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let main = PortName("main".to_string());

    let missing_target = NodeId(Uuid::new_v4());
    let result = workflow.add_connection_checked(source, missing_target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingTargetNode(missing_target)));
}

#[test]
fn given_compatible_ports_when_adding_checked_connection_then_connection_is_created() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(source, target, &main, &main);

    assert_eq!(result, Ok(ConnectionResult::Created));
    assert_eq!(workflow.connections.len(), 1);
}

// ---------------------------------------------------------------------------
// Additional edge-case tests for validate_connection
// ---------------------------------------------------------------------------

#[test]
fn given_both_nodes_missing_when_adding_checked_connection_then_source_not_found_error_is_returned()
{
    let mut workflow = Workflow::new();
    let ghost_source = NodeId(Uuid::new_v4());
    let ghost_target = NodeId(Uuid::new_v4());
    let main = PortName("main".to_string());

    // Source is checked first, so MissingSourceNode should be returned
    let result = workflow.add_connection_checked(ghost_source, ghost_target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(ghost_source)));
}

#[test]
fn given_missing_source_and_existing_target_when_adding_checked_connection_then_source_not_found_error_is_returned(
) {
    let mut workflow = Workflow::new();
    let existing_target = workflow.add_node("run", 0.0, 0.0);
    let ghost_source = NodeId(Uuid::new_v4());
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(ghost_source, existing_target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(ghost_source)));
}

#[test]
fn given_existing_source_and_missing_target_when_adding_checked_connection_then_target_not_found_error_is_returned(
) {
    let mut workflow = Workflow::new();
    let existing_source = workflow.add_node("http-handler", 0.0, 0.0);
    let ghost_target = NodeId(Uuid::new_v4());
    let main = PortName("main".to_string());

    let result = workflow.add_connection_checked(existing_source, ghost_target, &main, &main);

    assert_eq!(result, Err(ConnectionError::MissingTargetNode(ghost_target)));
}

#[test]
fn given_self_loop_on_different_port_names_when_adding_checked_connection_then_self_connection_error_is_returned(
) {
    let mut workflow = Workflow::new();
    let node = workflow.add_node("run", 0.0, 0.0);
    let port_a = PortName("output_a".to_string());
    let port_b = PortName("input_b".to_string());

    let result = workflow.add_connection_checked(node, node, &port_a, &port_b);

    assert_eq!(result, Err(ConnectionError::SelfConnection));
}

#[test]
fn given_same_endpoints_different_ports_when_adding_checked_connection_then_both_connections_are_created(
) {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("condition", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main_port = PortName("main".to_string());
    let alt_port = PortName("alt".to_string());

    // First connection with "main" ports
    let first = workflow.add_connection_checked(source, target, &main_port, &main_port);
    assert_eq!(first, Ok(ConnectionResult::Created));

    // The condition node outputs FlowControl and run accepts Plain, so this
    // will fail with TypeMismatch — use same type nodes instead.
    let mut wf2 = Workflow::new();
    let s = wf2.add_node("run", 0.0, 0.0);
    let t = wf2.add_node("run", 100.0, 0.0);

    let c1 = wf2.add_connection_checked(s, t, &main_port, &main_port);
    assert_eq!(c1, Ok(ConnectionResult::Created));

    // Different port names are treated as a different connection,
    // but type check still applies to the same port types
    let c2 = wf2.add_connection_checked(s, t, &alt_port, &main_port);
    assert_eq!(c2, Ok(ConnectionResult::Created));
    assert_eq!(wf2.connections.len(), 2);
}

#[test]
fn given_indirect_cycle_when_adding_checked_connection_then_cycle_error_is_returned() {
    // A -> B -> C, then try C -> A
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("run", 200.0, 0.0);
    let main = PortName("main".to_string());

    let _ = workflow.add_connection_checked(a, b, &main, &main);
    let _ = workflow.add_connection_checked(b, c, &main, &main);

    // Trying C -> A should detect the indirect cycle
    let result = workflow.add_connection_checked(c, a, &main, &main);
    assert_eq!(result, Err(ConnectionError::WouldCreateCycle));
}

// ---------------------------------------------------------------------------
// path_exists
// ---------------------------------------------------------------------------

#[test]
fn given_disconnected_nodes_when_checking_path_exists_then_false_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("run", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    // No connections

    assert!(!Workflow::path_exists(&workflow.connections, a, b));
}

#[test]
fn given_direct_edge_when_checking_path_exists_then_true_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let _ = workflow.add_connection_checked(a, b, &main, &main);

    assert!(Workflow::path_exists(&workflow.connections, a, b));
}

#[test]
fn given_transitive_path_when_checking_path_exists_then_true_is_returned() {
    let mut workflow = Workflow::new();
    let a = workflow.add_node("http-handler", 0.0, 0.0);
    let b = workflow.add_node("run", 100.0, 0.0);
    let c = workflow.add_node("run", 200.0, 0.0);
    let main = PortName("main".to_string());

    let _ = workflow.add_connection_checked(a, b, &main, &main);
    let _ = workflow.add_connection_checked(b, c, &main, &main);

    assert!(Workflow::path_exists(&workflow.connections, a, c));
}

#[test]
fn given_same_node_no_self_loop_when_checking_path_exists_then_false_is_returned() {
    let connections: Vec<Connection> = Vec::new();
    let node = NodeId(Uuid::new_v4());
    assert!(!Workflow::path_exists(&connections, node, node));
}

#[test]
fn given_self_loop_connection_when_checking_path_exists_from_node_to_itself_then_true_is_returned()
{
    let node = NodeId(Uuid::new_v4());
    let connections = vec![Connection {
        id: Uuid::new_v4(),
        source: node,
        target: node,
        source_port: PortName("main".to_string()),
        target_port: PortName("main".to_string()),
    }];

    assert!(Workflow::path_exists(&connections, node, node));
}

#[test]
fn given_empty_connections_when_checking_path_exists_then_false_is_returned() {
    let connections: Vec<Connection> = Vec::new();
    let a = NodeId(Uuid::new_v4());
    let b = NodeId(Uuid::new_v4());

    assert!(!Workflow::path_exists(&connections, a, b));
}
