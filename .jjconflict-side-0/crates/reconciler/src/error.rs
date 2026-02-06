//! Error types for the reconciler crate.

use std::fmt;

/// Result type alias for reconciler operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Reconciler error types.
#[derive(Debug, Clone)]
pub enum Error {
    /// Reconciliation failed.
    ReconcileFailed { reason: String },
    /// Action execution failed.
    ActionFailed { action: String, reason: String },
    /// State computation failed.
    StateFailed { reason: String },
    /// Bead not found.
    BeadNotFound { bead_id: String },
    /// Concurrency limit reached.
    ConcurrencyLimit { current: usize, max: usize },
    /// Event bus error.
    EventError { reason: String },
    /// Loop was stopped.
    LoopStopped,
    /// Invalid configuration.
    InvalidConfig { reason: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReconcileFailed { reason } => {
                write!(f, "reconciliation failed: {reason}")
            }
            Self::ActionFailed { action, reason } => {
                write!(f, "action '{action}' failed: {reason}")
            }
            Self::StateFailed { reason } => {
                write!(f, "state computation failed: {reason}")
            }
            Self::BeadNotFound { bead_id } => {
                write!(f, "bead '{bead_id}' not found")
            }
            Self::ConcurrencyLimit { current, max } => {
                write!(f, "concurrency limit reached ({current}/{max})")
            }
            Self::EventError { reason } => {
                write!(f, "event error: {reason}")
            }
            Self::LoopStopped => {
                write!(f, "reconciliation loop stopped")
            }
            Self::InvalidConfig { reason } => {
                write!(f, "invalid configuration: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Create a reconcile failed error.
    pub fn reconcile_failed(reason: impl Into<String>) -> Self {
        Self::ReconcileFailed {
            reason: reason.into(),
        }
    }

    /// Create an action failed error.
    pub fn action_failed(action: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ActionFailed {
            action: action.into(),
            reason: reason.into(),
        }
    }

    /// Create a state failed error.
    pub fn state_failed(reason: impl Into<String>) -> Self {
        Self::StateFailed {
            reason: reason.into(),
        }
    }

    /// Create a bead not found error.
    pub fn bead_not_found(bead_id: impl Into<String>) -> Self {
        Self::BeadNotFound {
            bead_id: bead_id.into(),
        }
    }

    /// Create a concurrency limit error.
    pub fn concurrency_limit(current: usize, max: usize) -> Self {
        Self::ConcurrencyLimit { current, max }
    }

    /// Create an event error.
    pub fn event_error(reason: impl Into<String>) -> Self {
        Self::EventError {
            reason: reason.into(),
        }
    }

    /// Create an invalid config error.
    pub fn invalid_config(reason: impl Into<String>) -> Self {
        Self::InvalidConfig {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::reconcile_failed("something went wrong");
        assert!(err.to_string().contains("something went wrong"));
    }

    #[test]
    fn test_action_failed() {
        let err = Error::action_failed("start_bead", "timeout");
        assert!(err.to_string().contains("start_bead"));
        assert!(err.to_string().contains("timeout"));
    }
}
