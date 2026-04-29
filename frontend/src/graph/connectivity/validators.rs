use crate::graph::graph_ops;
use crate::graph::restate_types::{types_compatible, ParsePortTypeError, PortType};
use crate::graph::workflow_node::WorkflowNode;
use crate::graph::{Connection, Node, NodeId, PortName};

use super::{ConnectionError, SourcePortType, TargetPortType};

/// Internal state representing a validated connection ready for mutation.
#[derive(Debug, Clone)]
pub(super) struct ValidationState {
    pub(super) source: NodeId,
    pub(super) target: NodeId,
    pub(super) source_port: PortName,
    pub(super) target_port: PortName,
}

pub(super) struct ConnectionRequest<'a> {
    pub(super) source: NodeId,
    pub(super) target: NodeId,
    pub(super) source_port: &'a PortName,
    pub(super) target_port: &'a PortName,
}

/// Validates that a connection can be added without mutating state.
///
/// # Errors
///
/// Returns `ConnectionError` if validation fails:
/// - Self-connection detected
/// - Source or target node missing
/// - Cycle would be created
/// - Connection already exists
/// - Port types incompatible
pub(super) fn validate_connection(
    nodes: &[Node],
    connections: &[Connection],
    request: &ConnectionRequest<'_>,
) -> Result<ValidationState, ConnectionError> {
    let source = request.source;
    let target = request.target;
    let source_port = request.source_port;
    let target_port = request.target_port;

    validate_not_self_connection(source, target)?;
    let (source, target) = validate_nodes_exist(nodes, source, target)?;
    validate_no_cycle(connections, target, source)?;
    validate_no_duplicate(connections, source, target, source_port, target_port)?;
    check_port_type_compatibility(nodes, source, target)?;

    Ok(ValidationState {
        source,
        target,
        source_port: source_port.clone(),
        target_port: target_port.clone(),
    })
}

fn validate_not_self_connection(source: NodeId, target: NodeId) -> Result<(), ConnectionError> {
    if source == target {
        Err(ConnectionError::SelfConnection)
    } else {
        Ok(())
    }
}

fn validate_nodes_exist(
    nodes: &[Node],
    source: NodeId,
    target: NodeId,
) -> Result<(NodeId, NodeId), ConnectionError> {
    let source_found = nodes.iter().any(|node| node.id == source);
    let target_found = nodes.iter().any(|node| node.id == target);

    if !source_found {
        return Err(ConnectionError::MissingSourceNode(source));
    }
    if !target_found {
        return Err(ConnectionError::MissingTargetNode(target));
    }
    Ok((source, target))
}

fn validate_no_cycle(
    connections: &[Connection],
    target: NodeId,
    source: NodeId,
) -> Result<(), ConnectionError> {
    if graph_ops::path_exists(connections, target, source) {
        Err(ConnectionError::WouldCreateCycle)
    } else {
        Ok(())
    }
}

fn validate_no_duplicate(
    connections: &[Connection],
    source: NodeId,
    target: NodeId,
    source_port: &PortName,
    target_port: &PortName,
) -> Result<(), ConnectionError> {
    if connections.iter().any(|c| {
        c.source == source
            && c.target == target
            && c.source_port == *source_port
            && c.target_port == *target_port
    }) {
        Err(ConnectionError::Duplicate)
    } else {
        Ok(())
    }
}

/// Single-pass node lookup: finds both source and target in one iteration
/// instead of two separate O(n) scans.
pub(super) fn find_source_and_target_nodes(
    nodes: &[Node],
    source: NodeId,
    target: NodeId,
) -> Result<(&Node, &Node), ConnectionError> {
    let mut source_node = None;
    let mut target_node = None;

    for node in nodes {
        if node.id == source {
            source_node = Some(node);
        }
        if node.id == target {
            target_node = Some(node);
        }
        // Early exit once both are found
        if source_node.is_some() && target_node.is_some() {
            break;
        }
    }

    let source_ref = source_node.ok_or(ConnectionError::MissingSourceNode(source))?;
    let target_ref = target_node.ok_or(ConnectionError::MissingTargetNode(target))?;
    Ok((source_ref, target_ref))
}

pub(super) fn check_port_type_compatibility(
    nodes: &[Node],
    source: NodeId,
    target: NodeId,
) -> Result<(), ConnectionError> {
    let (source_node, target_node) = find_source_and_target_nodes(nodes, source, target)?;

    let source_type = get_node_output_port_type(source_node)?;
    let target_type = get_node_input_port_type(target_node)?;

    if !types_compatible(source_type, target_type) {
        return Err(ConnectionError::TypeMismatch {
            source_type: SourcePortType(source_type),
            target_type: TargetPortType(target_type),
        });
    }

    Ok(())
}

fn get_node_output_port_type(node: &Node) -> Result<PortType, ConnectionError> {
    node.node_type
        .parse::<WorkflowNode>()
        .map_err(|_| {
            ConnectionError::ParseError(ParsePortTypeError::UnknownVariant {
                input: node.node_type.clone(),
                expected: "any, event, state, signal, flow-control, json",
            })
        })
        .map(|workflow_node| workflow_node.output_port_type())
}

fn get_node_input_port_type(node: &Node) -> Result<PortType, ConnectionError> {
    node.node_type
        .parse::<WorkflowNode>()
        .map_err(|_| {
            ConnectionError::ParseError(ParsePortTypeError::UnknownVariant {
                input: node.node_type.clone(),
                expected: "any, event, state, signal, flow-control, json",
            })
        })
        .map(|workflow_node| workflow_node.input_port_type())
}
