//! Error types for swarm operations.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type for swarm operations.
pub type SwarmResult<T> = Result<T, SwarmError>;

/// Work state for a bead in the swarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeadWorkState {
    /// Bead is pending assignment.
    Pending,

    /// Bead is claimed by an agent.
    Claimed,

    /// Contract is ready.
    ContractReady,

    /// Bead is being implemented.
    Implementing,

    /// Implementation is complete.
    ImplementationComplete,

    /// Bead is being reviewed.
    Reviewing,

    /// Bead has landed successfully.
    Landed,

    /// Bead failed and needs retry.
    Failed,
}

impl std::fmt::Display for BeadWorkState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Claimed => write!(f, "Claimed"),
            Self::ContractReady => write!(f, "ContractReady"),
            Self::Implementing => write!(f, "Implementing"),
            Self::ImplementationComplete => write!(f, "ImplementationComplete"),
            Self::Reviewing => write!(f, "Reviewing"),
            Self::Landed => write!(f, "Landed"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Error types for swarm operations.
#[derive(Debug, Error)]
pub enum SwarmError {
    /// Invalid state transition.
    #[error("Invalid state transition for bead {bead_id}: from {from} to {to}")]
    InvalidStateTransition {
        bead_id: String,
        from: BeadWorkState,
        to: BeadWorkState,
    },

    /// Bead not found.
    #[error("Bead not found: {bead_id}")]
    BeadNotFound { bead_id: String },

    /// IO error.
    #[error("IO error: {operation} failed: {reason}")]
    IoError { operation: String, reason: String },

    /// Handoff operation failed.
    #[error("Handoff failed: {file_path} - {operation} failed: {reason}")]
    HandoffFailed {
        file_path: String,
        operation: String,
        reason: String,
    },
}
