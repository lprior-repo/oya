//! DAG-specific error types

use super::BeadId;
use thiserror::Error;

/// DAG-specific error types
#[derive(Debug, Clone, Error)]
pub enum DagError {
    #[error("Node not found: {0}")]
    NodeNotFound(BeadId),

    #[error("Node already exists: {0}")]
    NodeAlreadyExists(BeadId),

    #[error("Edge already exists: {0} -> {1}")]
    EdgeAlreadyExists(BeadId, BeadId),

    #[error("Self-loop detected: {0}")]
    SelfLoopDetected(BeadId),

    #[error("Cycle detected involving nodes: {0:?}")]
    CycleDetected(Vec<BeadId>),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Graph is not connected")]
    NotConnected,

    #[error("Invalid node ID: {reason}")]
    InvalidNodeId { reason: String },
}

impl DagError {
    pub fn node_not_found(id: impl Into<BeadId>) -> Self {
        Self::NodeNotFound(id.into())
    }

    pub fn node_already_exists(id: impl Into<BeadId>) -> Self {
        Self::NodeAlreadyExists(id.into())
    }

    pub fn edge_already_exists(from: impl Into<BeadId>, to: impl Into<BeadId>) -> Self {
        Self::EdgeAlreadyExists(from.into(), to.into())
    }

    pub fn self_loop(id: impl Into<BeadId>) -> Self {
        Self::SelfLoopDetected(id.into())
    }

    pub fn cycle_detected(nodes: Vec<BeadId>) -> Self {
        Self::CycleDetected(nodes)
    }

    pub fn invalid_operation(reason: impl Into<String>) -> Self {
        Self::InvalidOperation(reason.into())
    }

    pub fn invalid_node_id(reason: impl Into<String>) -> Self {
        Self::InvalidNodeId {
            reason: reason.into(),
        }
    }
}

/// Result type for DAG operations
pub type DagResult<T> = std::result::Result<T, DagError>;
