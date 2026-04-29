#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use super::super::*;
use crate::graph::PortName;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// add_connection (bool-returning wrapper)
// ---------------------------------------------------------------------------

#[test]
fn given_valid_connection_when_adding_unchecked_then_ok_is_returned() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let added = workflow.add_connection(source, target, &main, &main);

    assert_eq!(added, Ok(ConnectionResult::Created));
    assert_eq!(workflow.connections.len(), 1);
}

#[test]
fn given_self_connection_when_adding_unchecked_then_error_is_returned() {
    let mut workflow = Workflow::new();
    let node = workflow.add_node("run", 0.0, 0.0);
    let main = PortName("main".to_string());

    assert_eq!(
        workflow.add_connection(node, node, &main, &main),
        Err(ConnectionError::SelfConnection)
    );
    assert!(workflow.connections.is_empty());
}

// ---------------------------------------------------------------------------
// find_source_and_target_nodes — single-pass optimization
// ---------------------------------------------------------------------------

#[test]
fn given_single_node_list_when_finding_source_and_target_then_both_resolve_to_same_node() {
    let mut workflow = Workflow::new();
    let node_id = workflow.add_node("run", 0.0, 0.0);

    let result =
        super::super::validators::find_source_and_target_nodes(&workflow.nodes, node_id, node_id);

    assert!(result.is_ok());
    let (src, tgt) = result.expect("both refer to same node");
    assert_eq!(src.id, node_id);
    assert_eq!(tgt.id, node_id);
}

#[test]
fn given_empty_node_list_when_finding_source_and_target_then_missing_source_error_is_returned() {
    use crate::graph::Node;

    let nodes: Vec<Node> = Vec::new();
    let source = NodeId(Uuid::new_v4());
    let target = NodeId(Uuid::new_v4());

    let result = super::super::validators::find_source_and_target_nodes(&nodes, source, target);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(source)));
}

// ---------------------------------------------------------------------------
// add_connection (bool) — error path returns false without mutating
// ---------------------------------------------------------------------------

#[test]
fn given_missing_source_when_adding_unchecked_then_error_is_returned_and_no_connection_added() {
    let mut workflow = Workflow::new();
    let target = workflow.add_node("run", 0.0, 0.0);
    let ghost_source = NodeId(Uuid::new_v4());
    let main = PortName("main".to_string());

    let added = workflow.add_connection(ghost_source, target, &main, &main);

    assert!(added.is_err());
    assert!(workflow.connections.is_empty());
}

#[test]
fn given_missing_target_when_adding_unchecked_then_error_is_returned_and_no_connection_added() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let ghost_target = NodeId(Uuid::new_v4());
    let main = PortName("main".to_string());

    let added = workflow.add_connection(source, ghost_target, &main, &main);

    assert!(added.is_err());
    assert!(workflow.connections.is_empty());
}

#[test]
fn given_duplicate_connection_when_adding_unchecked_then_error_is_returned() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName("main".to_string());

    let first = workflow.add_connection(source, target, &main, &main);
    assert_eq!(first, Ok(ConnectionResult::Created));

    let duplicate = workflow.add_connection(source, target, &main, &main);
    assert_eq!(duplicate, Err(ConnectionError::Duplicate));
    assert_eq!(workflow.connections.len(), 1, "only the first connection should exist");
}

// ---------------------------------------------------------------------------
// ConnectionError Display — error message coverage
// ---------------------------------------------------------------------------

#[test]
fn given_self_connection_error_when_displaying_then_message_contains_self_connection_text() {
    let err = ConnectionError::SelfConnection;
    let msg = format!("{err}");
    assert!(msg.contains("itself"), "expected self-reference in message, got: {msg}");
}

#[test]
fn given_missing_source_error_when_displaying_then_message_contains_node_id() {
    let id = NodeId(Uuid::new_v4());
    let err = ConnectionError::MissingSourceNode(id);
    let msg = format!("{err}");
    assert!(msg.contains(&id.to_string()), "expected node id in message, got: {msg}");
}

#[test]
fn given_missing_target_error_when_displaying_then_message_contains_node_id() {
    let id = NodeId(Uuid::new_v4());
    let err = ConnectionError::MissingTargetNode(id);
    let msg = format!("{err}");
    assert!(msg.contains(&id.to_string()), "expected node id in message, got: {msg}");
}

#[test]
fn given_would_create_cycle_error_when_displaying_then_message_contains_cycle_text() {
    let err = ConnectionError::WouldCreateCycle;
    let msg = format!("{err}");
    assert!(msg.contains("cycle"), "expected 'cycle' in message, got: {msg}");
}

#[test]
fn given_duplicate_error_when_displaying_then_message_contains_duplicate_text() {
    let err = ConnectionError::Duplicate;
    let msg = format!("{err}");
    assert!(msg.contains("already exists"), "expected 'already exists' in message, got: {msg}");
}

// ---------------------------------------------------------------------------
// check_port_type_compatibility_internal — error paths via public helper
// ---------------------------------------------------------------------------

#[test]
fn given_missing_source_node_when_checking_port_compatibility_then_missing_source_error() {
    let mut workflow = Workflow::new();
    let target = workflow.add_node("run", 0.0, 0.0);
    let ghost_source = NodeId(Uuid::new_v4());

    let result = check_port_type_compatibility_internal(&workflow.nodes, ghost_source, target);

    assert_eq!(result, Err(ConnectionError::MissingSourceNode(ghost_source)));
}

#[test]
fn given_missing_target_node_when_checking_port_compatibility_then_missing_target_error() {
    let mut workflow = Workflow::new();
    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let ghost_target = NodeId(Uuid::new_v4());

    let result = check_port_type_compatibility_internal(&workflow.nodes, source, ghost_target);

    assert_eq!(result, Err(ConnectionError::MissingTargetNode(ghost_target)));
}

#[test]
fn given_invalid_node_type_when_checking_port_compatibility_then_parse_error_is_returned() {
    use crate::graph::{ExecutionState, Node, NodeCategory, RunConfig, WorkflowNode};

    let mut workflow = Workflow::new();
    let source = workflow.add_node("run", 0.0, 0.0);

    let invalid_node = Node {
        id: NodeId(Uuid::new_v4()),
        name: "invalid".to_string(),
        node: WorkflowNode::Run(RunConfig::default()),
        category: NodeCategory::Flow,
        icon: "?".to_string(),
        x: 100.0,
        y: 0.0,
        last_output: None,
        selected: false,
        executing: false,
        skipped: false,
        error: None,
        execution_state: ExecutionState::default(),
        metadata: serde_json::Value::default(),
        execution_data: serde_json::Value::default(),
        node_type: "not-a-valid-node-type".to_string(),
        description: String::new(),
        config: serde_json::Value::default(),
    };
    workflow.nodes.push(invalid_node);
    let invalid_target = workflow.nodes.last().unwrap().id;

    let result = check_port_type_compatibility_internal(&workflow.nodes, source, invalid_target);

    assert!(matches!(result, Err(ConnectionError::ParseError(_))));
}
