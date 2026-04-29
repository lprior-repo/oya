//! Tests for connection_errors.rs: verifying error types are returned correctly.
//!
//! Covers: PortTypeMismatch, ServiceKindIncompatible, ContextTypeMismatch,
//! NodeNotFound, and valid connection acceptance.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use oya_frontend::graph::connection_errors::{check_connection, get_node_by_id, ConnectionError};
use oya_frontend::graph::restate_types::PortType;
use oya_frontend::graph::service_kinds::{ContextType, ServiceKind};
use oya_frontend::graph::workflow_node::WorkflowNode;
use oya_frontend::graph::{Node, NodeId, PortName, Workflow};

// ===========================================================================
// 1. PortTypeMismatch
// ===========================================================================

#[test]
fn given_incompatible_port_types_when_checking_connection_then_port_type_mismatch() {
    // condition outputs FlowControl, signal-handler expects Signal
    let source = WorkflowNode::Condition(Default::default());
    let target = WorkflowNode::SignalHandler(Default::default());

    let result = check_connection(&source, &target);
    assert_eq!(
        result,
        Err(ConnectionError::PortTypeMismatch {
            source: PortType::FlowControl,
            target: PortType::Signal,
        })
    );
}

#[test]
fn given_event_to_signal_when_checking_connection_then_port_type_mismatch() {
    let source = WorkflowNode::CronTrigger(Default::default());
    let target = WorkflowNode::SignalHandler(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::PortTypeMismatch { .. })));
}

#[test]
fn given_flow_control_to_event_when_checking_connection_then_port_type_mismatch() {
    let source = WorkflowNode::Run(Default::default());
    let target = WorkflowNode::CronTrigger(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::PortTypeMismatch { .. })));
}

#[test]
fn given_signal_to_event_when_checking_connection_then_port_type_mismatch() {
    let source = WorkflowNode::SignalHandler(Default::default());
    let target = WorkflowNode::CronTrigger(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::PortTypeMismatch { .. })));
}

// ===========================================================================
// 2. ServiceKindIncompatible (Handler → Actor)
// ===========================================================================

#[test]
fn given_handler_to_actor_when_checking_connection_then_service_kind_incompatible() {
    // Handler (e.g., run) → Actor (e.g., get-state)
    let source = WorkflowNode::Run(Default::default());
    let target = WorkflowNode::GetState(Default::default());

    let result = check_connection(&source, &target);
    assert_eq!(
        result,
        Err(ConnectionError::ServiceKindIncompatible {
            source_kind: ServiceKind::Handler,
            target_kind: ServiceKind::Actor,
            reason: "Handler cannot call Actor without state context",
        })
    );
}

#[test]
fn given_handler_to_save_to_memory_when_checking_connection_then_service_kind_incompatible() {
    let source = WorkflowNode::HttpHandler(Default::default());
    let target = WorkflowNode::SaveToMemory(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::ServiceKindIncompatible { .. })));
}

#[test]
fn given_handler_to_set_state_when_checking_connection_then_service_kind_incompatible() {
    let source = WorkflowNode::Run(Default::default());
    let target = WorkflowNode::SetState(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::ServiceKindIncompatible { .. })));
}

// ===========================================================================
// 3. ContextTypeMismatch (Asynchronous → Synchronous)
// ===========================================================================

#[test]
fn given_workflow_to_handler_when_checking_connection_then_context_type_mismatch() {
    // Workflow (e.g., workflow-call) → Handler (e.g., run) — async → sync
    let source = WorkflowNode::WorkflowCall(Default::default());
    let target = WorkflowNode::Run(Default::default());

    let result = check_connection(&source, &target);
    assert_eq!(
        result,
        Err(ConnectionError::ContextTypeMismatch {
            source_context: ContextType::Asynchronous,
            target_context: ContextType::Synchronous,
        })
    );
}

#[test]
fn given_workflow_submit_to_handler_when_checking_connection_then_context_type_mismatch() {
    let source = WorkflowNode::WorkflowSubmit(Default::default());
    let target = WorkflowNode::HttpHandler(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::ContextTypeMismatch { .. })));
}

#[test]
fn given_awakeable_to_actor_when_checking_connection_then_context_type_mismatch() {
    let source = WorkflowNode::Awakeable(Default::default());
    let target = WorkflowNode::GetState(Default::default());

    let result = check_connection(&source, &target);
    assert!(matches!(result, Err(ConnectionError::ContextTypeMismatch { .. })));
}

// ===========================================================================
// 4. NodeNotFound (via get_node_by_id)
// ===========================================================================

#[test]
fn given_empty_node_list_when_looking_up_node_then_node_not_found() {
    let nodes: Vec<Node> = vec![];
    let ghost = NodeId::new();

    let result = get_node_by_id(ghost, &nodes);
    assert_eq!(result, Err(ConnectionError::NodeNotFound { node_id: ghost }));
}

#[test]
fn given_node_list_without_target_when_looking_up_node_then_node_not_found() {
    let mut wf = Workflow::new();
    let _existing = wf.add_node("run", 0.0, 0.0);
    let ghost = NodeId::new();

    let result = get_node_by_id(ghost, &wf.nodes);
    assert_eq!(result, Err(ConnectionError::NodeNotFound { node_id: ghost }));
}

#[test]
fn given_node_list_with_target_when_looking_up_node_then_found() {
    let mut wf = Workflow::new();
    let id = wf.add_node("run", 0.0, 0.0);

    let result = get_node_by_id(id, &wf.nodes);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().id, id);
}

#[test]
fn given_node_not_found_error_when_displayed_then_contains_node_id() {
    let id = NodeId::new();
    let err = ConnectionError::NodeNotFound { node_id: id };
    let msg = format!("{err}");
    assert!(msg.contains(&id.to_string()), "Display should contain the node ID");
}

// ===========================================================================
// 5. Valid Connections (no error)
// ===========================================================================

#[test]
fn given_http_handler_to_http_call_when_checking_connection_then_ok() {
    let source = WorkflowNode::HttpHandler(Default::default());
    let target = WorkflowNode::HttpCall(Default::default());

    let result = check_connection(&source, &target);
    assert!(result.is_ok());
}

#[test]
fn given_run_to_run_when_checking_connection_then_ok() {
    let source = WorkflowNode::Run(Default::default());
    let target = WorkflowNode::Run(Default::default());

    let result = check_connection(&source, &target);
    assert!(result.is_ok());
}

#[test]
fn given_http_handler_to_run_when_checking_connection_then_ok() {
    // Json output → FlowControl input — Json is compatible with everything
    let source = WorkflowNode::HttpHandler(Default::default());
    let target = WorkflowNode::Run(Default::default());

    let result = check_connection(&source, &target);
    assert!(result.is_ok());
}

#[test]
fn given_condition_to_run_when_checking_connection_then_ok() {
    let source = WorkflowNode::Condition(Default::default());
    let target = WorkflowNode::Run(Default::default());

    let result = check_connection(&source, &target);
    assert!(result.is_ok());
}

#[test]
fn given_get_state_to_run_when_checking_connection_then_ok() {
    // Actor → Handler — Actor calling Handler is allowed (only Handler→Actor blocked)
    let source = WorkflowNode::GetState(Default::default());
    let target = WorkflowNode::Run(Default::default());

    let result = check_connection(&source, &target);
    assert!(result.is_ok());
}

#[test]
fn given_workflow_call_to_wait_for_webhook_when_checking_connection_then_ok() {
    // Both Workflow context, same context type
    let source = WorkflowNode::WorkflowCall(Default::default());
    let target = WorkflowNode::WaitForWebhook(Default::default());

    let result = check_connection(&source, &target);
    assert!(result.is_ok());
}

// ===========================================================================
// 6. Error Display Messages
// ===========================================================================

#[test]
fn given_port_type_mismatch_when_displayed_then_message_describes_types() {
    let err = ConnectionError::PortTypeMismatch {
        source: PortType::FlowControl,
        target: PortType::Signal,
    };
    let msg = format!("{err}");
    assert!(msg.contains("mismatch"), "Should mention mismatch: {msg}");
}

#[test]
fn given_service_kind_incompatible_when_displayed_then_message_describes_kinds() {
    let err = ConnectionError::ServiceKindIncompatible {
        source_kind: ServiceKind::Handler,
        target_kind: ServiceKind::Actor,
        reason: "test reason",
    };
    let msg = format!("{err}");
    assert!(msg.contains("handler"), "Should mention source kind: {msg}");
    assert!(msg.contains("actor"), "Should mention target kind: {msg}");
    assert!(msg.contains("test reason"), "Should include reason: {msg}");
}

#[test]
fn given_context_type_mismatch_when_displayed_then_message_describes_contexts() {
    let err = ConnectionError::ContextTypeMismatch {
        source_context: ContextType::Asynchronous,
        target_context: ContextType::Synchronous,
    };
    let msg = format!("{err}");
    assert!(msg.contains("mismatch"), "Should mention mismatch: {msg}");
}

// ===========================================================================
// 7. Integration: via Workflow add_connection_checked (graph-level errors)
// ===========================================================================

#[test]
fn given_self_connection_via_workflow_then_error_returned() {
    let mut wf = Workflow::new();
    let node = wf.add_node("run", 0.0, 0.0);
    let port = PortName("main".to_string());

    // Workflow's add_connection_checked uses the connectivity validator,
    // not connection_errors::check_connection. But self-connection is still blocked.
    let result = wf.add_connection(node, node, &port, &port);
    assert!(result.is_err(), "Self-connection should be rejected");
}

#[test]
fn given_cycle_via_workflow_then_error_returned() {
    let mut wf = Workflow::new();
    let a = wf.add_node("http-handler", 0.0, 0.0);
    let b = wf.add_node("run", 100.0, 0.0);
    let port = PortName("main".to_string());

    let _ = wf.add_connection(a, b, &port, &port);
    let result = wf.add_connection(b, a, &port, &port);
    assert!(result.is_err(), "Cycle should be detected");
}

#[test]
fn given_valid_connection_via_workflow_then_created() {
    let mut wf = Workflow::new();
    let a = wf.add_node("http-handler", 0.0, 0.0);
    let b = wf.add_node("run", 100.0, 0.0);
    let port = PortName("main".to_string());

    let result = wf.add_connection(a, b, &port, &port);
    assert!(result.is_ok());
}
