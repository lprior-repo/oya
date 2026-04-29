//! E2E tests for ExecutionState machine enforcement
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::RunConfig;
use oya_frontend::graph::{ExecutionState, Node, Workflow, WorkflowNode};

// ===========================================================================
// Full Workflow Lifecycle Tests
// ===========================================================================

#[test]
fn e2e_workflow_lifecycle_completes_all_states() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Start in Idle (default)
    assert_eq!(node.execution_state, ExecutionState::Idle);

    // Idle -> Queued
    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Queued);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("queued"));

    // Queued -> Running
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Running);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("running"));

    // Running -> Completed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Completed);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("completed"));
}

#[test]
fn e2e_workflow_lifecycle_skips_to_completed() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Idle -> Skipped
    let result = Workflow::set_node_status(&mut node, ExecutionState::Skipped);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Skipped);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("skipped"));
}

#[test]
fn e2e_workflow_lifecycle_runs_to_failed() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Idle -> Queued -> Running -> Failed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    assert!(result.is_ok());

    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_ok());

    let result = Workflow::set_node_status(&mut node, ExecutionState::Failed);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Failed);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("failed"));
}

// ===========================================================================
// Terminal State Immutability Tests
// ===========================================================================

#[test]
fn e2e_terminal_state_irreversibility() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Reach Completed state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    // Try to transition from Completed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Completed);
}

#[test]
fn e2e_terminal_state_irreversibility_failed() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Reach Failed state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Failed);

    // Try to transition from Failed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Failed);
}

#[test]
fn e2e_terminal_state_irreversibility_skipped() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Reach Skipped state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

    // Try to transition from Skipped
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Skipped);
}

// ===========================================================================
// Config Sync Across All Transitions Tests
// ===========================================================================

#[test]
fn e2e_config_sync_across_all_transitions() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Initial state: Idle, no config status
    assert_eq!(node.execution_state, ExecutionState::Idle);
    assert!(
        !node.config.is_object()
            || !node.config.as_object().expect("is_object checked above").contains_key("status")
    );

    // Idle -> Queued
    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Queued);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("queued"));

    // Queued -> Running
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Running);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("running"));

    // Running -> Completed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);
    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Completed);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("completed"));

    // Try invalid transition (Running -> Failed from Completed)
    let result = Workflow::set_node_status(&mut node, ExecutionState::Failed);
    assert!(result.is_err());

    // Config should remain "completed"
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("completed"));
}

// ===========================================================================
// State Machine Enforcement Tests
// ===========================================================================

#[test]
fn e2e_state_machine_enforcement_no_direct_mutation() {
    // This test verifies that the state machine prevents invalid state sequences
    // by testing that we cannot reach invalid states through valid transitions
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Try to skip Queued state (Idle -> Running)
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());

    // Try to skip Queued and Running (Idle -> Completed)
    let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);
    assert!(result.is_err());

    // Try to go backwards (Running -> Idle)
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);
    assert!(result.is_err());
}

#[test]
fn e2e_no_backwards_transitions() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Reach Running state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    // Try to go backwards to Idle
    let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);
    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Running);

    // Try to go backwards to Queued
    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Running);
}

#[test]
fn e2e_no_skipping_intermediate_states() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Try Idle -> Running (skipping Queued)
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());

    // Try Queued -> Completed (skipping Running)
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);
    assert!(result.is_err());
}

#[test]
fn e2e_queued_gateway_enforcement() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Verify that Running can only be reached via Queued
    // This test documents the invariant that Idle -> Running is invalid

    // Direct transition should fail
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());

    // Valid path: Idle -> Queued -> Running
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_ok());
}

// ===========================================================================
// Mutation Survivability Tests
// ===========================================================================

#[test]
fn e2e_mutation_survivability_terminal_check() {
    // This test verifies that terminal state checks are in place
    // If terminal state checks were removed, this test would fail

    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Reach terminal state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    // Verify terminal state check is working
    // If this passes, terminal state check is in place
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());
}

#[test]
fn e2e_mutation_survivability_self_transition_rejection() {
    // This test verifies that self-transitions are rejected
    // If self-transition checks were removed, this test would fail

    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Try self-transition at Idle
    let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);
    assert!(result.is_err());

    // Try self-transition at Running
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);
    assert!(result.is_err());
}

// ===========================================================================
// Empty Config Tests
// ===========================================================================

#[test]
fn e2e_empty_config_handling() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Node starts with empty config
    assert!(node.config.is_object() || node.config.is_null());

    // Set status should work with empty config
    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    assert!(result.is_ok());
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("queued"));
}
