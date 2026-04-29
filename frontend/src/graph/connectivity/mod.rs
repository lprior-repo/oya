use crate::graph::graph_ops;
use crate::graph::restate_types::{ParsePortTypeError, PortType};
use crate::graph::{Connection, NodeId, Workflow};

mod builders;
mod validators;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePortType(pub PortType);

impl std::fmt::Display for SourcePortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPortType(pub PortType);

impl std::fmt::Display for TargetPortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
    SelfConnection,
    MissingSourceNode(NodeId),
    MissingTargetNode(NodeId),
    WouldCreateCycle,
    Duplicate,
    TypeMismatch { source_type: SourcePortType, target_type: TargetPortType },
    ParseError(ParsePortTypeError),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfConnection => write!(f, "Cannot connect node to itself"),
            Self::MissingSourceNode(node_id) => {
                write!(f, "Source node not found: {node_id}")
            }
            Self::MissingTargetNode(node_id) => {
                write!(f, "Target node not found: {node_id}")
            }
            Self::WouldCreateCycle => write!(f, "Connection would create a cycle"),
            Self::Duplicate => write!(f, "Connection already exists"),
            Self::TypeMismatch { source_type, target_type } => {
                write!(f, "Type mismatch: {source_type} is not compatible with {target_type}")
            }
            Self::ParseError(err) => write!(f, "Parse error: {err}"),
        }
    }
}

impl std::error::Error for ConnectionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionResult {
    Created,
}

// ---------------------------------------------------------------------------
// Public query method
// ---------------------------------------------------------------------------

impl Workflow {
    /// Checks whether a directed path exists from `from` to `to` through the connection graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use oya_frontend::graph::{Workflow, NodeId, PortName};
    /// let mut workflow = Workflow::new();
    /// let a = workflow.add_node("http-handler", 0.0, 0.0);
    /// let b = workflow.add_node("run", 100.0, 0.0);
    /// let c = workflow.add_node("run", 200.0, 0.0);
    /// let main = PortName("main".to_string());
    ///
    /// assert!(!Workflow::path_exists(&workflow.connections, a, c));
    ///
    /// let _ = workflow.add_connection_checked(a, b, &main, &main);
    /// let _ = workflow.add_connection_checked(b, c, &main, &main);
    ///
    /// assert!(Workflow::path_exists(&workflow.connections, a, c));
    /// ```
    #[must_use]
    pub fn path_exists(connections: &[Connection], from: NodeId, to: NodeId) -> bool {
        graph_ops::path_exists(connections, from, to)
    }
}

// ---------------------------------------------------------------------------
// Test-only helpers
// ---------------------------------------------------------------------------

/// Helper function for tests to access `path_exists` directly.
///
/// This is a testing API — not part of the public interface.
#[must_use]
pub fn path_exists_internal(connections: &[Connection], from: NodeId, to: NodeId) -> bool {
    graph_ops::path_exists(connections, from, to)
}

/// Helper function for tests to access `check_port_type_compatibility` directly.
///
/// This is a testing API — not part of the public interface.
///
/// # Errors
///
/// Returns `ConnectionError::MissingSourceNode` if source not found.
/// Returns `ConnectionError::MissingTargetNode` if target not found.
/// Returns `ConnectionError::TypeMismatch` if types incompatible.
pub fn check_port_type_compatibility_internal(
    nodes: &[crate::graph::Node],
    source: NodeId,
    target: NodeId,
) -> Result<(), ConnectionError> {
    validators::check_port_type_compatibility(nodes, source, target)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    mod connection_extras;
    mod connection_validation;
}
