//! Integration tests for ExecutionState machine enforcement
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::RunConfig;
use oya_frontend::graph::{ExecutionState, InvalidTransition, Node, Workflow, WorkflowNode};

// ===========================================================================
// set_node_status Valid Transition Tests
// ===========================================================================

#[test]
fn set_node_status_updates_config_on_idle_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Queued);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("queued"));
}

#[test]
fn set_node_status_updates_config_on_idle_to_skipped() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Skipped);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("skipped"));
}

#[test]
fn set_node_status_updates_config_on_queued_to_running() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First transition to Queued
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);

    // Then transition to Running
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Running);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("running"));
}

#[test]
fn set_node_status_updates_config_on_queued_to_skipped() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First transition to Queued
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);

    // Then transition to Skipped
    let result = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Skipped);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("skipped"));
}

#[test]
fn set_node_status_updates_config_on_running_to_completed() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Transition to Running
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    // Then transition to Completed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Completed);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("completed"));
}

#[test]
fn set_node_status_updates_config_on_running_to_failed() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Transition to Running
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    // Then transition to Failed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Failed);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Failed);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("failed"));
}

// ===========================================================================
// set_node_status Invalid Transition Tests
// ===========================================================================

#[test]
fn set_node_status_returns_error_on_completed_to_running() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First transition to Completed
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    // Try to transition from Completed to Running
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Completed, ExecutionState::Running));
    // State should remain Completed
    assert_eq!(node.execution_state, ExecutionState::Completed);
}

#[test]
fn set_node_status_returns_error_on_running_to_idle() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Transition to Running first
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    // Try to go backwards
    let result = Workflow::set_node_status(&mut node, ExecutionState::Idle);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Running, ExecutionState::Idle));
    // State should remain Running
    assert_eq!(node.execution_state, ExecutionState::Running);
}

#[test]
fn set_node_status_returns_error_on_idle_to_running() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running));
}

#[test]
fn set_node_status_returns_error_on_queued_to_completed() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Idle, ExecutionState::Completed));
}

#[test]
fn set_node_status_returns_error_on_failed_to_running() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Idle, ExecutionState::Running));
}

#[test]
fn set_node_status_returns_error_on_skipped_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Skipped);

    // Try to go back to Queued
    let result = Workflow::set_node_status(&mut node, ExecutionState::Queued);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Skipped, ExecutionState::Queued));
}

// ===========================================================================
// set_node_status Terminal State Tests
// ===========================================================================

#[test]
fn set_node_status_rejects_completed_to_any() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First reach Completed state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    // Try any transition from Completed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Completed);
}

#[test]
fn set_node_status_rejects_failed_to_any() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First reach Failed state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Failed);

    // Try any transition from Failed
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Failed);
}

#[test]
fn set_node_status_rejects_skipped_to_any() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First reach Skipped state
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

    // Try any transition from Skipped
    let result = Workflow::set_node_status(&mut node, ExecutionState::Running);

    assert!(result.is_err());
    assert_eq!(node.execution_state, ExecutionState::Skipped);
}

// ===========================================================================
// set_node_status Config Sync Tests
// ===========================================================================

#[test]
fn set_node_status_leaves_config_unchanged_on_failed_transition() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Running first
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    let original_status = node.config.get("status").cloned();

    // Try invalid transition
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Idle);

    // Config should be unchanged
    assert_eq!(node.config.get("status"), original_status.as_ref());
}

#[test]
fn set_node_status_leaves_state_unchanged_on_failed_transition() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Running first
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    let original_state = node.execution_state;

    // Try invalid transition
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Idle);

    // State should be unchanged
    assert_eq!(node.execution_state, original_state);
}

// ===========================================================================
// set_node_pending_status Tests
// ===========================================================================

#[test]
fn set_node_pending_status_allows_idle_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    let result = Workflow::set_node_pending_status(&mut node);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Queued);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("pending"));
}

#[test]
fn set_node_pending_status_allows_queued_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // First set to Queued
    let _ = Workflow::set_node_pending_status(&mut node);

    // Then call again (Queued -> Queued is valid for pending)
    let result = Workflow::set_node_pending_status(&mut node);

    assert!(result.is_ok());
    assert_eq!(node.execution_state, ExecutionState::Queued);
    assert_eq!(node.config.get("status").and_then(|v| v.as_str()), Some("pending"));
}

#[test]
fn set_node_pending_status_rejects_running_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Running
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    // Try to set pending
    let result = Workflow::set_node_pending_status(&mut node);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Running, ExecutionState::Queued));
}

#[test]
fn set_node_pending_status_rejects_completed_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Completed
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Completed);

    // Try to set pending
    let result = Workflow::set_node_pending_status(&mut node);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Completed, ExecutionState::Queued));
}

#[test]
fn set_node_pending_status_rejects_failed_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Failed
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Failed);

    // Try to set pending
    let result = Workflow::set_node_pending_status(&mut node);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Failed, ExecutionState::Queued));
}

#[test]
fn set_node_pending_status_rejects_skipped_to_queued() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Skipped
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Skipped);

    // Try to set pending
    let result = Workflow::set_node_pending_status(&mut node);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, InvalidTransition::new(ExecutionState::Skipped, ExecutionState::Queued));
}

#[test]
fn set_node_pending_status_leaves_config_unchanged_on_failure() {
    let mut node = Node::from_workflow_node(
        "test".to_string(),
        WorkflowNode::Run(RunConfig::default()),
        0.0,
        0.0,
    );

    // Set to Running
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Queued);
    let _ = Workflow::set_node_status(&mut node, ExecutionState::Running);

    let original_status = node.config.get("status").cloned();

    // Try invalid set_node_pending_status
    let _ = Workflow::set_node_pending_status(&mut node);

    // Config should be unchanged
    assert_eq!(node.config.get("status"), original_status.as_ref());
}
