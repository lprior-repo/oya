//! Error types for the events crate.

use std::fmt;
use thiserror::Error;

/// Result type alias for event operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Connection error for SurrealDB setup.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ConnectionError {
    #[error("invalid connection URL: {0}")]
    InvalidUrl(String),

    #[error("connection pool exhausted: max {max_connections} reached")]
    PoolExhausted { max_connections: usize },

    #[error("connection timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("database initialization failed: {reason}")]
    InitializationFailed { reason: String },

    #[error("backend not supported: {backend}")]
    UnsupportedBackend { backend: String },
}

/// Event error types.
#[derive(Debug, Clone)]
pub enum Error {
    /// Event store operation failed.
    StoreFailed { operation: String, reason: String },
    /// Event not found.
    EventNotFound { event_id: String },
    /// Invalid event data.
    InvalidEvent { reason: String },
    /// Subscription failed.
    SubscriptionFailed { reason: String },
    /// Channel closed.
    ChannelClosed,
    /// Serialization error.
    Serialization { reason: String },
    /// Projection failed.
    ProjectionFailed { projection: String, reason: String },
    /// Bead not found.
    BeadNotFound { bead_id: String },
    /// Invalid state transition.
    InvalidTransition { from: String, to: String },
    /// Internal error.
    Internal(String),
    /// Connection error.
    Connection(#[from] ConnectionError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoreFailed { operation, reason } => {
                write!(f, "event store operation '{operation}' failed: {reason}")
            }
            Self::EventNotFound { event_id } => {
                write!(f, "event '{event_id}' not found")
            }
            Self::InvalidEvent { reason } => {
                write!(f, "invalid event: {reason}")
            }
            Self::SubscriptionFailed { reason } => {
                write!(f, "subscription failed: {reason}")
            }
            Self::ChannelClosed => {
                write!(f, "event channel closed")
            }
            Self::Serialization { reason } => {
                write!(f, "serialization error: {reason}")
            }
            Self::ProjectionFailed { projection, reason } => {
                write!(f, "projection '{projection}' failed: {reason}")
            }
            Self::BeadNotFound { bead_id } => {
                write!(f, "bead '{bead_id}' not found")
            }
            Self::InvalidTransition { from, to } => {
                write!(f, "invalid state transition from '{from}' to '{to}'")
            }
            Self::Internal(msg) => {
                write!(f, "internal error: {msg}")
            }
            Self::Connection(err) => {
                write!(f, "connection error: {err}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Create a store failed error.
    pub fn store_failed(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::StoreFailed {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create an event not found error.
    pub fn event_not_found(event_id: impl Into<String>) -> Self {
        Self::EventNotFound {
            event_id: event_id.into(),
        }
    }

    /// Create an invalid event error.
    pub fn invalid_event(reason: impl Into<String>) -> Self {
        Self::InvalidEvent {
            reason: reason.into(),
        }
    }

    /// Create a subscription failed error.
    pub fn subscription_failed(reason: impl Into<String>) -> Self {
        Self::SubscriptionFailed {
            reason: reason.into(),
        }
    }

    /// Create a serialization error.
    pub fn serialization(reason: impl Into<String>) -> Self {
        Self::Serialization {
            reason: reason.into(),
        }
    }

    /// Create a projection failed error.
    pub fn projection_failed(projection: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ProjectionFailed {
            projection: projection.into(),
            reason: reason.into(),
        }
    }

    /// Create a bead not found error.
    pub fn bead_not_found(bead_id: impl Into<String>) -> Self {
        Self::BeadNotFound {
            bead_id: bead_id.into(),
        }
    }

    /// Create an invalid transition error.
    pub fn invalid_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidTransition {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Create a connection error.
    pub fn connection(err: ConnectionError) -> Self {
        Self::Connection(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::store_failed("append", "disk full");
        assert!(err.to_string().contains("append"));
        assert!(err.to_string().contains("disk full"));
    }

    #[test]
    fn test_connection_error_display() {
        let err = ConnectionError::InvalidUrl("bad://url".to_string());
        assert!(err.to_string().contains("bad://url"));

        let err = ConnectionError::PoolExhausted {
            max_connections: 10,
        };
        assert!(err.to_string().contains("10"));

        let err = ConnectionError::Timeout { timeout_ms: 5000 };
        assert!(err.to_string().contains("5000"));
    }

    #[test]
    fn test_error_connection_conversion() {
        let conn_err = ConnectionError::InitializationFailed {
            reason: "rocksdb locked".to_string(),
        };
        let err = Error::connection(conn_err);
        assert!(err.to_string().contains("rocksdb locked"));
    }
}
