//! Connection validation errors for Restate workflows.
//!
//! Implements Scott Wlaschin DDD principles:
//! - Parse, don't validate
//! - Make illegal states unrepresentable
//! - Types act as documentation

use crate::graph::{
    core_types::Node,
    port_types::types_compatible,
    restate_types::PortType,
    service_kinds::{ContextType, ServiceKind},
    workflow_node::WorkflowNode,
    NodeId,
};

// ===========================================================================
// Connection Error
// ===========================================================================

/// Errors that can occur when validating connections between nodes.
///
/// This enum captures all possible reasons why a connection between two workflow
/// nodes might be invalid, providing detailed error messages for debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
    /// Port types are incompatible
    PortTypeMismatch { source: PortType, target: PortType },

    /// Service kinds are incompatible for the operation
    ServiceKindIncompatible {
        source_kind: ServiceKind,
        target_kind: ServiceKind,
        reason: &'static str,
    },

    /// Context types are incompatible
    ContextTypeMismatch { source_context: ContextType, target_context: ContextType },

    /// Node reference not found
    NodeNotFound { node_id: NodeId },
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PortTypeMismatch { source, target } => {
                write!(f, "Port type mismatch: {source} cannot connect to {target}")
            }
            Self::ServiceKindIncompatible { source_kind, target_kind, reason } => write!(
                f,
                "Service kind incompatible: {source_kind} cannot connect to {target_kind}: {reason}"
            ),
            Self::ContextTypeMismatch { source_context, target_context } => write!(
                f,
                "Context type mismatch: {source_context} cannot connect to {target_context}"
            ),
            Self::NodeNotFound { node_id } => {
                write!(f, "Node not found: {node_id}")
            }
        }
    }
}

impl std::error::Error for ConnectionError {}

// ===========================================================================
// Helper Functions
// ===========================================================================

/// Lookup a node by ID from a node slice.
///
/// Returns `Ok(&node)` if found, `Err(ConnectionError::NodeNotFound)` if not.
///
/// # Errors
///
/// Returns `ConnectionError::NodeNotFound` if no node with the given ID exists.
///
/// # Examples
///
/// ```
/// use crate::oya_frontend::graph::{NodeId, Node, workflow_node::WorkflowNode};
/// use crate::oya_frontend::graph::connection_errors::get_node_by_id;
///
/// let nodes = vec![
///     Node::from_workflow_node("test".to_string(), WorkflowNode::default(), 0.0, 0.0),
/// ];
/// let node_id = nodes[0].id;
/// let result = get_node_by_id(node_id, &nodes);
/// assert!(result.is_ok());
/// ```
pub fn get_node_by_id(id: NodeId, nodes: &[Node]) -> Result<&Node, ConnectionError> {
    nodes.iter().find(|n| n.id == id).ok_or(ConnectionError::NodeNotFound { node_id: id })
}

// ===========================================================================
// Connection Validation
// ===========================================================================

/// Checks if a connection between two nodes is valid, considering both
/// port types and service kinds.
///
/// Returns `Ok` if the connection is valid, `Err` with specific error otherwise.
///
/// # Errors
///
/// Returns `ConnectionError::PortTypeMismatch` if port types are incompatible.
/// Returns `ConnectionError::ContextTypeMismatch` if context types are incompatible
/// (e.g., asynchronous workflow nodes connecting to synchronous nodes).
///
/// # Examples
///
/// ```
/// use crate::oya_frontend::graph::{workflow_node::WorkflowNode, connection_errors::check_connection};
///
/// let source = WorkflowNode::HttpHandler(Default::default());
/// let target = WorkflowNode::HttpCall(Default::default());
/// let result = check_connection(&source, &target);
/// assert!(result.is_ok());
/// ```
pub fn check_connection(
    source_node: &WorkflowNode,
    target_node: &WorkflowNode,
) -> Result<(), ConnectionError> {
    // Check port type compatibility
    let source_port_type = source_node.output_port_type();
    let target_port_type = target_node.input_port_type();

    if !types_compatible(source_port_type, target_port_type) {
        return Err(ConnectionError::PortTypeMismatch {
            source: source_port_type,
            target: target_port_type,
        });
    }

    // Check service kind compatibility
    // Handler nodes cannot call Actor nodes without state context
    let source_kind = source_node.service_kind();
    let target_kind = target_node.service_kind();

    if source_kind == ServiceKind::Handler && target_kind == ServiceKind::Actor {
        return Err(ConnectionError::ServiceKindIncompatible {
            source_kind,
            target_kind,
            reason: "Handler cannot call Actor without state context",
        });
    }

    // Context type compatibility check for workflow nodes
    let source_context = source_node.context_type();
    let target_context = target_node.context_type();

    // Workflow nodes (with promises) can only connect to Workflow nodes
    if source_context == ContextType::Asynchronous && target_context != ContextType::Asynchronous {
        return Err(ConnectionError::ContextTypeMismatch { source_context, target_context });
    }

    Ok(())
}
