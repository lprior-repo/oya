//! Integration tests for the undo/redo system.
//!
//! These tests verify the undo/redo snapshot machinery operates correctly
//! across add-node, move-node, connect, and edge-case scenarios.
//!
//! The undo/redo logic lives in `use_workflow_state.rs` as standalone functions:
//!   - `push_undo_snapshot(stack, snapshot, cap)`  -- push with eviction
//!   - `apply_undo(workflow, undo_stack, redo_stack)` -- pop undo, push redo
//!   - `apply_redo(workflow, undo_stack, redo_stack)` -- pop redo, push undo
//!
//! All tests operate directly on `Workflow` and the stack functions,
//! without requiring a Dioxus runtime.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::{NodeId, PortName, Workflow};

// ---------------------------------------------------------------------------
// Helpers -- replicate the standalone functions from use_workflow_state.rs
// ---------------------------------------------------------------------------

fn push_undo_snapshot(undo_stack: &mut Vec<Workflow>, snapshot: Workflow, cap: usize) {
    undo_stack.push(snapshot);
    if undo_stack.len() > cap {
        undo_stack.remove(0);
    }
}

fn apply_undo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
) -> bool {
    match undo_stack.pop() {
        Some(snapshot) => {
            let current = workflow.clone();
            redo_stack.push(current);
            *workflow = snapshot;
            true
        }
        None => false,
    }
}

fn apply_redo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
) -> bool {
    match redo_stack.pop() {
        Some(snapshot) => {
            let current = workflow.clone();
            undo_stack.push(current);
            *workflow = snapshot;
            true
        }
        None => false,
    }
}

const STACK_CAP: usize = 30;

/// Simulate the "add node then save undo point" transaction pattern.
fn add_node_with_undo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
    node_type: &str,
    x: f32,
    y: f32,
) -> NodeId {
    let snapshot = workflow.clone();
    let id = workflow.add_node(node_type, x, y);
    push_undo_snapshot(undo_stack, snapshot, STACK_CAP);
    redo_stack.clear();
    id
}

/// Simulate the "update position then save undo point" transaction pattern.
fn move_node_with_undo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
    node_id: NodeId,
    dx: f32,
    dy: f32,
) {
    let snapshot = workflow.clone();
    workflow.update_node_position(node_id, dx, dy);
    push_undo_snapshot(undo_stack, snapshot, STACK_CAP);
    redo_stack.clear();
}

/// Simulate the "add connection then save undo point" transaction pattern.
fn add_connection_with_undo(
    workflow: &mut Workflow,
    undo_stack: &mut Vec<Workflow>,
    redo_stack: &mut Vec<Workflow>,
    source: NodeId,
    target: NodeId,
    source_port: &PortName,
    target_port: &PortName,
) -> bool {
    let snapshot = workflow.clone();
    let ok = workflow.add_connection(source, target, source_port, target_port);
    if ok.is_ok() {
        push_undo_snapshot(undo_stack, snapshot, STACK_CAP);
        redo_stack.clear();
    }
    ok.is_ok()
}

fn node_position(workflow: &Workflow, id: NodeId) -> Option<(f32, f32)> {
    workflow.nodes.iter().find(|n| n.id == id).map(|n| (n.x, n.y))
}

// ===========================================================================
// 1. Basic undo/redo cycle
// ===========================================================================

#[test]
fn given_node_added_when_undo_then_node_is_removed_and_redo_restores_it() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    // Add a node with undo tracking
    let node_id =
        add_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, "run", 10.0, 20.0);

    assert_eq!(workflow.nodes.len(), 1, "workflow should have 1 node after add");
    assert!(undo_stack.len() == 1, "undo stack should have 1 snapshot");

    // Undo: node should be gone
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo, "undo should succeed");
    assert!(workflow.nodes.is_empty(), "workflow should have 0 nodes after undo");
    assert!(undo_stack.is_empty(), "undo stack should be empty after undo");
    assert!(redo_stack.len() == 1, "redo stack should have 1 snapshot");

    // Redo: node should be back
    let did_redo = apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_redo, "redo should succeed");
    assert_eq!(workflow.nodes.len(), 1, "workflow should have 1 node after redo");
    assert!(
        workflow.nodes.iter().any(|n| n.id == node_id),
        "restored node should match original id"
    );
}

#[test]
fn given_node_moved_when_undo_then_position_is_restored() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    // Add a node (no undo snapshot needed for the initial add in this test)
    let node_id = workflow.add_node("run", 50.0, 60.0);
    let original_pos = node_position(&workflow, node_id);
    assert_eq!(original_pos, Some((50.0, 60.0)));

    // Move with undo tracking
    move_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, node_id, 100.0, 200.0);
    let moved_pos = node_position(&workflow, node_id);
    assert_eq!(moved_pos, Some((150.0, 260.0)), "node position should reflect the move");

    // Undo: position should be restored
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo, "undo should succeed");
    let restored_pos = node_position(&workflow, node_id);
    assert_eq!(
        restored_pos, original_pos,
        "node position should be restored to original after undo"
    );
}

#[test]
fn given_two_nodes_connected_when_undo_then_connection_is_gone_and_redo_restores_it() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    let source = workflow.add_node("http-handler", 0.0, 0.0);
    let target = workflow.add_node("run", 100.0, 0.0);
    let main = PortName::from("main");

    // Connect with undo tracking
    let ok = add_connection_with_undo(
        &mut workflow,
        &mut undo_stack,
        &mut redo_stack,
        source,
        target,
        &main,
        &main,
    );
    assert!(ok, "connection should be created");
    assert_eq!(workflow.connections.len(), 1, "workflow should have 1 connection");

    // Undo: connection should be gone
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo, "undo should succeed");
    assert!(workflow.connections.is_empty(), "connection should be gone after undo");
    assert_eq!(workflow.nodes.len(), 2, "nodes should still be present after undo");

    // Redo: connection should be back
    let did_redo = apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_redo, "redo should succeed");
    assert_eq!(workflow.connections.len(), 1, "connection should be restored after redo");
}

// ===========================================================================
// 2. Undo stack limits
// ===========================================================================

#[test]
fn given_35_snapshots_when_cap_is_30_then_oldest_5_are_evicted() {
    let mut undo_stack: Vec<Workflow> = Vec::new();

    // Push 35 snapshots with cap 30
    (0..35).for_each(|i| {
        let mut wf = Workflow::new();
        let _ = wf.add_node("run", i as f32, 0.0);
        push_undo_snapshot(&mut undo_stack, wf, STACK_CAP);
    });

    // Stack should be capped at 30
    assert_eq!(undo_stack.len(), STACK_CAP, "stack should be capped at {STACK_CAP}");

    // The first 5 snapshots (i=0..4) should have been evicted.
    // The remaining stack should contain snapshots with 1 node each,
    // corresponding to i=5..34. Verify by checking node x positions.
    let first_x = undo_stack.first().and_then(|wf| wf.nodes.first()).map(|n| n.x);
    assert_eq!(first_x, Some(5.0), "oldest surviving snapshot should have x=5.0");

    let last_x = undo_stack.last().and_then(|wf| wf.nodes.first()).map(|n| n.x);
    assert_eq!(last_x, Some(34.0), "newest snapshot should have x=34.0");
}

#[test]
fn given_stack_eviction_when_undo_then_workflow_restores_correctly() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    // Push 35 snapshots, each adding a node. Cap at 30.
    (0..35).for_each(|i| {
        let snapshot = workflow.clone();
        let _ = workflow.add_node("run", i as f32 * 10.0, 0.0);
        push_undo_snapshot(&mut undo_stack, snapshot, STACK_CAP);
        redo_stack.clear();
    });

    assert_eq!(undo_stack.len(), STACK_CAP);

    // Undo should work -- restores to the snapshot that was pushed just before
    // the last add_node call
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo, "undo should succeed even after eviction");

    // After one undo, the workflow should have 34 nodes (the state before the 35th add)
    assert_eq!(
        workflow.nodes.len(),
        34,
        "workflow should have 34 nodes after undoing the 35th add"
    );
}

// ===========================================================================
// 3. Undo/redo with selection
// ===========================================================================

#[test]
fn given_selected_node_when_undo_add_then_selection_flag_is_also_restored() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    // Start with one selected node
    let first_id = workflow.add_node("run", 0.0, 0.0);
    if let Some(node) = workflow.nodes.iter_mut().find(|n| n.id == first_id) {
        node.set_selected(true);
    }
    assert!(workflow.nodes.iter().any(|n| n.selected));

    // Save snapshot of "selected node" state, then add a second node
    let _second_id = add_node_with_undo(
        &mut workflow,
        &mut undo_stack,
        &mut redo_stack,
        "http-handler",
        100.0,
        0.0,
    );

    // After adding, still 2 nodes, first should still be selected
    assert_eq!(workflow.nodes.len(), 2);
    let first_selected = workflow.nodes.iter().find(|n| n.id == first_id).map(|n| n.selected);
    assert_eq!(first_selected, Some(true));

    // Undo: go back to the 1-node state where the node was selected
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo, "undo should succeed");
    assert_eq!(workflow.nodes.len(), 1, "should be back to 1 node");
    assert!(
        workflow.nodes.iter().any(|n| n.selected),
        "the restored snapshot should still have the node selected"
    );
}

// ===========================================================================
// 4. Edge cases
// ===========================================================================

#[test]
fn given_empty_undo_stack_when_undo_then_workflow_is_unchanged() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();
    let _ = workflow.add_node("run", 0.0, 0.0);
    let snapshot = workflow.clone();

    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);

    assert!(!did_undo, "undo should return false with empty stack");
    assert_eq!(workflow, snapshot, "workflow should be unchanged after no-op undo");
    assert!(redo_stack.is_empty(), "redo stack should remain empty after no-op undo");
}

#[test]
fn given_empty_redo_stack_when_redo_then_workflow_is_unchanged() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();
    let _ = workflow.add_node("run", 0.0, 0.0);
    let snapshot = workflow.clone();

    let did_redo = apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack);

    assert!(!did_redo, "redo should return false with empty stack");
    assert_eq!(workflow, snapshot, "workflow should be unchanged after no-op redo");
    assert!(undo_stack.is_empty(), "undo stack should remain empty after no-op redo");
}

#[test]
fn given_undo_performed_when_new_operation_then_redo_stack_is_cleared() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    // Operation 1: add node A
    let _node_a =
        add_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, "run", 0.0, 0.0);

    // Operation 2: add node B
    let _node_b = add_node_with_undo(
        &mut workflow,
        &mut undo_stack,
        &mut redo_stack,
        "http-handler",
        100.0,
        0.0,
    );

    assert_eq!(workflow.nodes.len(), 2);
    assert_eq!(undo_stack.len(), 2);

    // Undo once: go back to 1-node state
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo);
    assert_eq!(workflow.nodes.len(), 1);
    assert_eq!(redo_stack.len(), 1, "redo stack should have 1 entry");

    // New operation after undo: should clear redo stack
    let _node_c =
        add_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, "run", 200.0, 0.0);

    assert!(redo_stack.is_empty(), "redo stack should be cleared after a new operation");
    assert_eq!(
        undo_stack.len(),
        2,
        "undo stack should have 2 snapshots (original + new operation)"
    );

    // Redo should now be a no-op since the redo stack was cleared
    let did_redo = apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(!did_redo, "redo should be no-op after redo stack is cleared");
}

// ===========================================================================
// Additional coverage: multi-step undo chains
// ===========================================================================

#[test]
fn given_three_operations_when_undo_twice_then_workflow_is_at_first_state() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    let _ = add_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, "run", 0.0, 0.0);
    assert_eq!(workflow.nodes.len(), 1);

    let _ = add_node_with_undo(
        &mut workflow,
        &mut undo_stack,
        &mut redo_stack,
        "http-handler",
        100.0,
        0.0,
    );
    assert_eq!(workflow.nodes.len(), 2);

    let _ = add_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, "run", 200.0, 0.0);
    assert_eq!(workflow.nodes.len(), 3);

    // Undo twice: should be back to 1 node
    let _ = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert_eq!(workflow.nodes.len(), 2);

    let _ = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert_eq!(workflow.nodes.len(), 1, "should be back to 1 node after 2 undos");

    // Redo twice: should be back to 3 nodes
    let _ = apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert_eq!(workflow.nodes.len(), 2);

    let _ = apply_redo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert_eq!(workflow.nodes.len(), 3, "should be back to 3 nodes after 2 redos");
}

#[test]
fn given_node_removed_with_undo_when_undo_then_node_is_restored() {
    let mut workflow = Workflow::new();
    let mut undo_stack: Vec<Workflow> = Vec::new();
    let mut redo_stack: Vec<Workflow> = Vec::new();

    // Add two nodes
    let node_a =
        add_node_with_undo(&mut workflow, &mut undo_stack, &mut redo_stack, "run", 0.0, 0.0);
    let node_b = add_node_with_undo(
        &mut workflow,
        &mut undo_stack,
        &mut redo_stack,
        "http-handler",
        100.0,
        0.0,
    );
    assert_eq!(workflow.nodes.len(), 2);

    // Remove node_b with undo tracking
    let snapshot = workflow.clone();
    workflow.remove_node(node_b);
    push_undo_snapshot(&mut undo_stack, snapshot, STACK_CAP);
    redo_stack.clear();

    assert_eq!(workflow.nodes.len(), 1);
    assert!(workflow.nodes.iter().all(|n| n.id != node_b));

    // Undo the removal
    let did_undo = apply_undo(&mut workflow, &mut undo_stack, &mut redo_stack);
    assert!(did_undo, "undo should succeed");
    assert_eq!(workflow.nodes.len(), 2, "both nodes should be back after undo");
    assert!(workflow.nodes.iter().any(|n| n.id == node_a), "node_a should be present");
    assert!(workflow.nodes.iter().any(|n| n.id == node_b), "node_b should be restored after undo");
}
