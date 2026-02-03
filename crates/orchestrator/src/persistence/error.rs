//! Persistence error types for the orchestrator.
//!
//! All errors are explicit, typed, and recoverable - no panics allowed.

use std::fmt;

use thiserror::Error;

/// Errors that can occur during persistence operations.
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// Failed to connect to the database
    #[error("connection failed: {reason}")]
    ConnectionFailed { reason: String },

    /// Query execution failed
    #[error("query failed: {reason}")]
    QueryFailed { reason: String },

    /// Record not found
    #[error("record not found: {entity_type} with id '{id}'")]
    NotFound { entity_type: String, id: String },

    /// Record already exists
    #[error("record already exists: {entity_type} with id '{id}'")]
    AlreadyExists { entity_type: String, id: String },

    /// Serialization/deserialization error
    #[error("serialization error: {reason}")]
    SerializationError { reason: String },

    /// Transaction failed
    #[error("transaction failed: {reason}")]
    TransactionFailed { reason: String },

    /// Connection pool exhausted
    #[error("connection pool exhausted")]
    PoolExhausted,

    /// Timeout waiting for operation
    #[error("operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Invalid state for operation
    #[error("invalid state: {reason}")]
    InvalidState { reason: String },

    /// Schema error
    #[error("schema error: {reason}")]
    SchemaError { reason: String },
}

impl PersistenceError {
    /// Create a connection failed error.
    pub fn connection_failed(reason: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            reason: reason.into(),
        }
    }

    /// Create a query failed error.
    pub fn query_failed(reason: impl Into<String>) -> Self {
        Self::QueryFailed {
            reason: reason.into(),
        }
    }

    /// Create a not found error.
    pub fn not_found(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type: entity_type.into(),
            id: id.into(),
        }
    }

    /// Create an already exists error.
    pub fn already_exists(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::AlreadyExists {
            entity_type: entity_type.into(),
            id: id.into(),
        }
    }

    /// Create a serialization error.
    pub fn serialization_error(reason: impl Into<String>) -> Self {
        Self::SerializationError {
            reason: reason.into(),
        }
    }

    /// Create a transaction failed error.
    pub fn transaction_failed(reason: impl Into<String>) -> Self {
        Self::TransactionFailed {
            reason: reason.into(),
        }
    }

    /// Create a timeout error.
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Create an invalid state error.
    pub fn invalid_state(reason: impl Into<String>) -> Self {
        Self::InvalidState {
            reason: reason.into(),
        }
    }

    /// Create a schema error.
    pub fn schema_error(reason: impl Into<String>) -> Self {
        Self::SchemaError {
            reason: reason.into(),
        }
    }

    /// Check if error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionFailed { .. }
                | Self::PoolExhausted
                | Self::Timeout { .. }
                | Self::TransactionFailed { .. }
        )
    }
}

/// Result type for persistence operations.
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Helper to convert SurrealDB errors to PersistenceError.
pub fn from_surrealdb_error(err: impl fmt::Display) -> PersistenceError {
    let msg = err.to_string();

    // Categorize based on error message patterns
    if msg.contains("timeout") || msg.contains("Timeout") {
        PersistenceError::timeout(0)
    } else if msg.contains("connection") || msg.contains("Connection") || msg.contains("connect") {
        PersistenceError::connection_failed(msg)
    } else if msg.contains("already exists") || msg.contains("duplicate") {
        PersistenceError::already_exists("unknown", msg)
    } else if msg.contains("not found") || msg.contains("does not exist") {
        PersistenceError::not_found("unknown", msg)
    } else {
        PersistenceError::query_failed(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_failed_error() {
        let err = PersistenceError::connection_failed("host unreachable");
        assert!(matches!(err, PersistenceError::ConnectionFailed { .. }));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_not_found_error() {
        let err = PersistenceError::not_found("workflow", "wf-123");
        assert!(matches!(err, PersistenceError::NotFound { .. }));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_timeout_error() {
        let err = PersistenceError::timeout(5000);
        assert!(matches!(
            err,
            PersistenceError::Timeout { duration_ms: 5000 }
        ));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = PersistenceError::not_found("workflow", "wf-123");
        assert_eq!(
            err.to_string(),
            "record not found: workflow with id 'wf-123'"
        );
    }

    #[test]
    fn test_from_surrealdb_error_timeout() {
        let err = from_surrealdb_error("operation timeout after 30s");
        assert!(matches!(err, PersistenceError::Timeout { .. }));
    }

    #[test]
    fn test_from_surrealdb_error_connection() {
        let err = from_surrealdb_error("connection refused");
        assert!(matches!(err, PersistenceError::ConnectionFailed { .. }));
    }

    #[test]
    fn test_from_surrealdb_error_generic() {
        let err = from_surrealdb_error("some random error");
        assert!(matches!(err, PersistenceError::QueryFailed { .. }));
    }
}
