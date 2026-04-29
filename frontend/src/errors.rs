use thiserror::Error;

use crate::graph::NodeId;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    #[error("Node {0} not found")]
    NodeNotFound(NodeId),

    #[error("Connection would create a cycle")]
    CycleDetected,

    #[error("Connection already exists")]
    DuplicateConnection,

    #[error("Invalid connection: {0}")]
    InvalidConnection(String),

    #[error("Cannot connect node to itself")]
    SelfConnection,
}

pub type WorkflowResult<T> = Result<T, WorkflowError>;
