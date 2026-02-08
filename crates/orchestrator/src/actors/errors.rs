//! Actor-specific error types.
//!
//! These are business logic errors returned in RPC replies.
//! They are NOT actor crashes - they're normal error responses.

use std::time::Duration;
use thiserror::Error;

/// Business logic errors returned in RPC replies.
///
/// These errors represent expected failure modes that don't crash the actor.
/// The actor continues running after returning these errors to callers.
#[derive(Debug, Clone, Error)]
pub enum ActorError {
    /// The requested workflow was not found.
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    /// The requested bead was not found.
    #[error("Bead not found: {0}")]
    BeadNotFound(String),

    /// A workflow with this ID already exists.
    #[error("Workflow already exists: {0}")]
    WorkflowAlreadyExists(String),

    /// An error occurred in DAG operations.
    #[error("DAG error: {0}")]
    DagError(String),

    /// RPC call timed out.
    #[error("RPC timeout after {0:?}")]
    RpcTimeout(Duration),

    /// The actor is not available (stopped or not started).
    #[error("Actor not available")]
    ActorUnavailable,

    /// Invalid state transition.
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// The bead is already claimed by another worker.
    #[error("Bead already claimed: {bead_id} by {worker_id}")]
    BeadAlreadyClaimed { bead_id: String, worker_id: String },

    /// Channel communication error.
    #[error("Channel error: {0}")]
    ChannelError(String),

    /// Failed to spawn an actor.
    #[error("Spawn failed: {0}")]
    SpawnFailed(String),

    /// Internal actor error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ActorError {
    /// Create a workflow not found error.
    pub fn workflow_not_found(id: impl Into<String>) -> Self {
        Self::WorkflowNotFound(id.into())
    }

    /// Create a bead not found error.
    pub fn bead_not_found(id: impl Into<String>) -> Self {
        Self::BeadNotFound(id.into())
    }

    /// Create a generic not found error (uses BeadNotFound variant).
    pub fn not_found(resource: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::BeadNotFound(format!("{}: {}", resource.into(), reason.into()))
    }

    /// Create a workflow already exists error.
    pub fn workflow_already_exists(id: impl Into<String>) -> Self {
        Self::WorkflowAlreadyExists(id.into())
    }

    /// Create a DAG error.
    pub fn dag_error(msg: impl Into<String>) -> Self {
        Self::DagError(msg.into())
    }

    /// Create an RPC timeout error.
    pub fn rpc_timeout(duration: Duration) -> Self {
        Self::RpcTimeout(duration)
    }

    /// Create an actor unavailable error.
    pub fn actor_unavailable() -> Self {
        Self::ActorUnavailable
    }

    /// Create an invalid state transition error.
    pub fn invalid_state_transition(msg: impl Into<String>) -> Self {
        Self::InvalidStateTransition(msg.into())
    }

    /// Create a bead already claimed error.
    pub fn bead_already_claimed(bead_id: impl Into<String>, worker_id: impl Into<String>) -> Self {
        Self::BeadAlreadyClaimed {
            bead_id: bead_id.into(),
            worker_id: worker_id.into(),
        }
    }

    /// Create a channel error.
    pub fn channel_error(msg: impl Into<String>) -> Self {
        Self::ChannelError(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl From<crate::dag::DagError> for ActorError {
    fn from(e: crate::dag::DagError) -> Self {
        Self::DagError(e.to_string())
    }
}

impl From<crate::Error> for ActorError {
    fn from(e: crate::Error) -> Self {
        Self::DagError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_workflow_not_found_error() {
        let err = ActorError::workflow_not_found("wf-123");
        assert!(matches!(err, ActorError::WorkflowNotFound(_)));
        assert!(err.to_string().contains("wf-123"));
    }

    #[test]
    fn should_create_bead_not_found_error() {
        let err = ActorError::bead_not_found("bead-456");
        assert!(matches!(err, ActorError::BeadNotFound(_)));
        assert!(err.to_string().contains("bead-456"));
    }

    #[test]
    fn should_create_dag_error_from_dag_error() {
        let dag_err = crate::dag::DagError::node_not_found("node-789");
        let actor_err: ActorError = dag_err.into();
        assert!(matches!(actor_err, ActorError::DagError(_)));
    }

    #[test]
    fn should_create_rpc_timeout_error() {
        let err = ActorError::rpc_timeout(Duration::from_secs(5));
        assert!(matches!(err, ActorError::RpcTimeout(_)));
        assert!(err.to_string().contains("5s"));
    }

    #[test]
    fn should_create_bead_already_claimed_error() {
        let err = ActorError::bead_already_claimed("bead-1", "worker-2");
        assert!(err.to_string().contains("bead-1"));
        assert!(err.to_string().contains("worker-2"));
    }
}
