//! Error types for the workflow crate.

use std::fmt;

/// Result type alias for workflow operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Workflow error types.
#[derive(Debug, Clone)]
pub enum Error {
    /// Phase execution failed.
    PhaseFailed {
        phase_name: String,
        reason: String,
    },
    /// Phase timeout exceeded.
    PhaseTimeout {
        phase_name: String,
        timeout_secs: u64,
    },
    /// Rollback failed.
    RollbackFailed {
        phase_name: String,
        reason: String,
    },
    /// Checkpoint creation failed.
    CheckpointFailed {
        reason: String,
    },
    /// Checkpoint not found.
    CheckpointNotFound {
        phase_id: String,
    },
    /// Rewind failed.
    RewindFailed {
        reason: String,
    },
    /// Journal replay failed.
    ReplayFailed {
        reason: String,
    },
    /// Invalid state transition.
    InvalidTransition {
        from: String,
        to: String,
    },
    /// Storage operation failed.
    StorageFailed {
        operation: String,
        reason: String,
    },
    /// Workflow not found.
    WorkflowNotFound {
        workflow_id: String,
    },
    /// Phase not found.
    PhaseNotFound {
        phase_name: String,
    },
    /// Serialization error.
    Serialization {
        reason: String,
    },
    /// Handler not registered.
    HandlerNotFound {
        phase_name: String,
    },
    /// Maximum retries exceeded.
    MaxRetriesExceeded {
        phase_name: String,
        attempts: u32,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhaseFailed { phase_name, reason } => {
                write!(f, "phase '{phase_name}' failed: {reason}")
            }
            Self::PhaseTimeout {
                phase_name,
                timeout_secs,
            } => {
                write!(f, "phase '{phase_name}' timed out after {timeout_secs}s")
            }
            Self::RollbackFailed { phase_name, reason } => {
                write!(f, "rollback of phase '{phase_name}' failed: {reason}")
            }
            Self::CheckpointFailed { reason } => {
                write!(f, "checkpoint creation failed: {reason}")
            }
            Self::CheckpointNotFound { phase_id } => {
                write!(f, "checkpoint not found for phase '{phase_id}'")
            }
            Self::RewindFailed { reason } => {
                write!(f, "rewind failed: {reason}")
            }
            Self::ReplayFailed { reason } => {
                write!(f, "journal replay failed: {reason}")
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid state transition from '{from}' to '{to}'")
            }
            Self::StorageFailed { operation, reason } => {
                write!(f, "storage operation '{operation}' failed: {reason}")
            }
            Self::WorkflowNotFound { workflow_id } => {
                write!(f, "workflow '{workflow_id}' not found")
            }
            Self::PhaseNotFound { phase_name } => {
                write!(f, "phase '{phase_name}' not found")
            }
            Self::Serialization { reason } => {
                write!(f, "serialization error: {reason}")
            }
            Self::HandlerNotFound { phase_name } => {
                write!(f, "handler not found for phase '{phase_name}'")
            }
            Self::MaxRetriesExceeded {
                phase_name,
                attempts,
            } => {
                write!(
                    f,
                    "phase '{phase_name}' exceeded max retries ({attempts} attempts)"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Create a phase failed error.
    pub fn phase_failed(phase_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::PhaseFailed {
            phase_name: phase_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a phase timeout error.
    pub fn phase_timeout(phase_name: impl Into<String>, timeout_secs: u64) -> Self {
        Self::PhaseTimeout {
            phase_name: phase_name.into(),
            timeout_secs,
        }
    }

    /// Create a rollback failed error.
    pub fn rollback_failed(phase_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::RollbackFailed {
            phase_name: phase_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a checkpoint failed error.
    pub fn checkpoint_failed(reason: impl Into<String>) -> Self {
        Self::CheckpointFailed {
            reason: reason.into(),
        }
    }

    /// Create a checkpoint not found error.
    pub fn checkpoint_not_found(phase_id: impl Into<String>) -> Self {
        Self::CheckpointNotFound {
            phase_id: phase_id.into(),
        }
    }

    /// Create a rewind failed error.
    pub fn rewind_failed(reason: impl Into<String>) -> Self {
        Self::RewindFailed {
            reason: reason.into(),
        }
    }

    /// Create a replay failed error.
    pub fn replay_failed(reason: impl Into<String>) -> Self {
        Self::ReplayFailed {
            reason: reason.into(),
        }
    }

    /// Create an invalid transition error.
    pub fn invalid_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidTransition {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Create a storage failed error.
    pub fn storage_failed(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::StorageFailed {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a workflow not found error.
    pub fn workflow_not_found(workflow_id: impl Into<String>) -> Self {
        Self::WorkflowNotFound {
            workflow_id: workflow_id.into(),
        }
    }

    /// Create a phase not found error.
    pub fn phase_not_found(phase_name: impl Into<String>) -> Self {
        Self::PhaseNotFound {
            phase_name: phase_name.into(),
        }
    }

    /// Create a serialization error.
    pub fn serialization(reason: impl Into<String>) -> Self {
        Self::Serialization {
            reason: reason.into(),
        }
    }

    /// Create a handler not found error.
    pub fn handler_not_found(phase_name: impl Into<String>) -> Self {
        Self::HandlerNotFound {
            phase_name: phase_name.into(),
        }
    }

    /// Create a max retries exceeded error.
    pub fn max_retries_exceeded(phase_name: impl Into<String>, attempts: u32) -> Self {
        Self::MaxRetriesExceeded {
            phase_name: phase_name.into(),
            attempts,
        }
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::PhaseFailed { .. }
                | Self::PhaseTimeout { .. }
                | Self::StorageFailed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::phase_failed("build", "compilation error");
        assert!(err.to_string().contains("build"));
        assert!(err.to_string().contains("compilation error"));
    }

    #[test]
    fn test_is_retryable() {
        assert!(Error::phase_failed("test", "transient").is_retryable());
        assert!(Error::phase_timeout("test", 30).is_retryable());
        assert!(!Error::handler_not_found("test").is_retryable());
    }
}
