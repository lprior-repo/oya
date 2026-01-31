//! Error types for Juggernaut SDLC Factory.
//!
//! All errors are structured for AI consumption with error codes and context.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Core error type with Railway-Oriented design.
#[derive(Error, Debug, Serialize, Deserialize, Clone)]
pub enum JuggernautError {
    #[error("workflow error: {message}")]
    Workflow {
        code: WorkflowErrorCode,
        message: String,
        context: Option<String>,
    },

    #[error("event error: {message}")]
    Event {
        code: EventErrorCode,
        message: String,
        context: Option<String>,
    },

    #[error("storage error: {message}")]
    Storage {
        code: StorageErrorCode,
        message: String,
        context: Option<String>,
    },

    #[error("intent error: {message}")]
    Intent {
        code: IntentErrorCode,
        message: String,
        context: Option<String>,
    },

    #[error("reconcile error: {message}")]
    Reconcile {
        code: ReconcileErrorCode,
        message: String,
        context: Option<String>,
    },

    #[error("external error: {message}")]
    External {
        code: ExternalErrorCode,
        message: String,
        context: Option<String>,
    },

    #[error("state transition error: cannot transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: crate::state::BeadState,
        to: crate::state::BeadState,
    },
}

/// Workflow error codes for AI parsing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowErrorCode {
    PhaseTimeout,
    PhaseFailure,
    CheckpointCorrupted,
    JournalReplayFailed,
    RewindFailed,
    InvalidPhaseTransition,
}

/// Event error codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventErrorCode {
    PublishFailed,
    SubscribeFailed,
    StoreAppendFailed,
    ProjectionFailed,
    EventNotFound,
}

/// Storage error codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageErrorCode {
    ConnectionFailed,
    QueryFailed,
    WriteFailed,
    SerializationFailed,
    DeserializationFailed,
    NotFound,
}

/// Intent error codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentErrorCode {
    ParseFailed,
    DecompositionFailed,
    EarsValidationFailed,
    KirkAnalysisFailed,
}

/// Reconcile error codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReconcileErrorCode {
    StateMismatch,
    ActionFailed,
    ConcurrencyLimitExceeded,
    DependencyUnresolved,
}

/// External error codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalErrorCode {
    OpencodeConnectionFailed,
    OpencodeExecutionFailed,
    LlmTimeout,
    NetworkError,
}

/// Result alias for Juggernaut operations.
pub type Result<T> = std::result::Result<T, JuggernautError>;

impl JuggernautError {
    /// Create a workflow error.
    pub fn workflow(code: WorkflowErrorCode, message: impl Into<String>) -> Self {
        Self::Workflow {
            code,
            message: message.into(),
            context: None,
        }
    }

    /// Create a workflow error with context.
    pub fn workflow_with_context(
        code: WorkflowErrorCode,
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::Workflow {
            code,
            message: message.into(),
            context: Some(context.into()),
        }
    }

    /// Create a storage error.
    pub fn storage(code: StorageErrorCode, message: impl Into<String>) -> Self {
        Self::Storage {
            code,
            message: message.into(),
            context: None,
        }
    }

    /// Create an event error.
    pub fn event(code: EventErrorCode, message: impl Into<String>) -> Self {
        Self::Event {
            code,
            message: message.into(),
            context: None,
        }
    }

    /// Create an invalid state transition error.
    pub fn invalid_transition(
        from: crate::state::BeadState,
        to: crate::state::BeadState,
    ) -> Self {
        Self::InvalidStateTransition { from, to }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::BeadState;

    #[test]
    fn test_workflow_error_creation() {
        let err = JuggernautError::workflow(WorkflowErrorCode::PhaseTimeout, "phase timed out");
        assert!(matches!(err, JuggernautError::Workflow { code: WorkflowErrorCode::PhaseTimeout, .. }));
    }

    #[test]
    fn test_workflow_error_with_context() {
        let err = JuggernautError::workflow_with_context(
            WorkflowErrorCode::PhaseFailure,
            "phase failed",
            "during RED phase",
        );
        if let JuggernautError::Workflow { context, .. } = err {
            assert_eq!(context, Some("during RED phase".to_string()));
        } else {
            std::process::abort(); // Can't panic, but this should never happen
        }
    }

    #[test]
    fn test_invalid_state_transition_error() {
        let err = JuggernautError::invalid_transition(BeadState::Pending, BeadState::Completed);
        let msg = format!("{}", err);
        assert!(msg.contains("Pending"));
        assert!(msg.contains("Completed"));
    }

    #[test]
    fn test_error_serialization() {
        let err = JuggernautError::storage(StorageErrorCode::NotFound, "bead not found");
        let json = serde_json::to_string(&err);
        assert!(json.is_ok());
    }
}
