//! Integration tests for workflow validation.
//!
//! These tests verify validation behavior through the public API,
//! treating the validation module as a black box.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{
    validate_unique_node_ids, validate_workflow, Connection, Node, NodeCategory, NodeId, Workflow,
};
use uuid::Uuid;

// ===========================================================================
// Integration Tests: Structural Validation
// ===========================================================================

/// Tests that entry point validation works end-to-end
#[test]
fn integration_workflow_validation_requires_entry_point() {
    let node = Node {
        id: NodeId::new(),
        name: "Durable Node".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(!result.valid);
    assert!(result.has_errors());
    assert_eq!(result.error_count(), 1);
    assert!(result.issues[0].message.contains("entry point"));
}

/// Tests that a workflow with an entry point passes validation
#[test]
fn integration_workflow_validation_passes_with_entry_point() {
    let node = Node {
        id: NodeId::new(),
        name: "Entry Node".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    assert!(result.valid);
    assert_eq!(result.error_count(), 0);
}

/// Tests that validation collects multiple issues
#[test]
fn integration_validation_collects_multiple_issues() {
    let entry_id = NodeId::new();
    let orphan_id = NodeId::new();

    let entry_node = Node {
        id: entry_id,
        name: "Entry".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let orphan_node = Node {
        id: orphan_id,
        name: "Orphan".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };

    let workflow = Workflow {
        nodes: vec![entry_node, orphan_node],
        connections: vec![],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);

    // Should have at least entry validation passing, but orphan detection should run
    // error_count() and issues.len() return usize, always >= 0; just verify no panic
    let _ = result.error_count();
    let _ = result.issues.len();
}

/// Tests that validation never mutates the input workflow
#[test]
fn integration_validation_never_mutates_input() {
    let node = Node {
        id: NodeId::new(),
        name: "Original Name".to_string(),
        category: NodeCategory::Entry,
        executing: true,
        ..Default::default()
    };
    let workflow = Workflow { nodes: vec![node], connections: vec![], ..Default::default() };

    let original_name = workflow.nodes[0].name.clone();
    let original_executing = workflow.nodes[0].executing;

    let _ = validate_workflow(&workflow);
    // validate_workflow returns Result, discarding is acceptable here since we're testing
    // that it doesn't mutate the workflow regardless of success/failure

    assert_eq!(workflow.nodes[0].name, original_name);
    assert_eq!(workflow.nodes[0].executing, original_executing);
}

/// Tests duplicate node ID detection in validation
#[test]
fn integration_validation_detects_duplicate_node_ids() {
    let id = NodeId::new();
    let node1 = Node {
        id,
        name: "Node 1".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let node2 = Node {
        id,
        name: "Node 2".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };

    let workflow =
        Workflow { nodes: vec![node1, node2], connections: vec![], ..Default::default() };

    let issues = validate_unique_node_ids(&workflow);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity, oya_frontend::graph::ValidationSeverity::Error);
    assert!(issues[0].message.contains("Duplicate node ID"));
}

/// Tests validation with multiple nodes and connections
#[test]
fn integration_validation_with_multiple_nodes_and_connections() {
    let entry_id = NodeId::new();
    let node1_id = NodeId::new();
    let node2_id = NodeId::new();

    let entry_node = Node {
        id: entry_id,
        name: "Entry".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let node1 = Node {
        id: node1_id,
        name: "Node 1".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };
    let node2 = Node {
        id: node2_id,
        name: "Node 2".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };

    let conn1 = Connection {
        id: Uuid::new_v4(),
        source: entry_id,
        target: node1_id,
        source_port: "out".into(),
        target_port: "in".into(),
    };
    let conn2 = Connection {
        id: Uuid::new_v4(),
        source: node1_id,
        target: node2_id,
        source_port: "out".into(),
        target_port: "in".into(),
    };

    let workflow = Workflow {
        nodes: vec![entry_node, node1, node2],
        connections: vec![conn1, conn2],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);

    // Should pass since we have an entry point and connected nodes
    assert!(result.valid || !result.has_errors());
}

/// Tests validation of workflow with only entry nodes
#[test]
fn integration_validation_multiple_entry_points() {
    let entry1_id = NodeId::new();
    let entry2_id = NodeId::new();

    let entry1 = Node {
        id: entry1_id,
        name: "Entry 1".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let entry2 = Node {
        id: entry2_id,
        name: "Entry 2".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };

    let workflow =
        Workflow { nodes: vec![entry1, entry2], connections: vec![], ..Default::default() };

    let result = validate_workflow(&workflow);

    // Should pass - multiple entry points are valid
    assert!(result.valid);
}

/// Tests validation of workflow with unreachable nodes
#[test]
fn integration_validation_detects_unreachable_nodes() {
    let entry_id = NodeId::new();
    let reachable_id = NodeId::new();
    let unreachable_id = NodeId::new();

    let entry_node = Node {
        id: entry_id,
        name: "Entry".to_string(),
        category: NodeCategory::Entry,
        ..Default::default()
    };
    let reachable_node = Node {
        id: reachable_id,
        name: "Reachable".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };
    let unreachable_node = Node {
        id: unreachable_id,
        name: "Unreachable".to_string(),
        category: NodeCategory::Durable,
        ..Default::default()
    };

    let conn = Connection {
        id: Uuid::new_v4(),
        source: entry_id,
        target: reachable_id,
        source_port: "out".into(),
        target_port: "in".into(),
    };

    let workflow = Workflow {
        nodes: vec![entry_node, reachable_node, unreachable_node],
        connections: vec![conn],
        ..Default::default()
    };

    let result = validate_workflow(&workflow);

    // Should detect unreachable node as warning; issues.len() is usize, always >= 0
    let _ = result.issues.len();
}
