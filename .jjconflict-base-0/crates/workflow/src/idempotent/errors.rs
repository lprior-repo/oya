//! Error types for idempotent execution.
//!
//! This module provides error types for the IdempotentExecutor, focusing on
//! duplicate execution detection and proper error propagation.

use std::fmt;
use uuid::Uuid;

/// Result type alias for idempotent operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Idempotent executor error types.
#[derive(Debug, Clone)]
pub enum Error {
    /// Duplicate execution detected - returning cached result.
    DuplicateExecution {
        /// The idempotency key that was already executed.
        key: Uuid,
        /// Optional description of the cached result.
        cached_result_info: Option<String>,
    },
    /// Cache lookup failed.
    CacheLookupFailed {
        /// The key that failed to lookup.
        key: Uuid,
        /// The reason for the failure.
        reason: String,
    },
    /// Database lookup failed.
    DbLookupFailed {
        /// The key that failed to lookup.
        key: Uuid,
        /// The reason for the failure.
        reason: String,
    },
    /// Failed to store result in cache.
    CacheStoreFailed {
        /// The key that failed to store.
        key: Uuid,
        /// The reason for the failure.
        reason: String,
    },
    /// Failed to store result in database.
    DbStoreFailed {
        /// The key that failed to store.
        key: Uuid,
        /// The reason for the failure.
        reason: String,
    },
    /// Execution function failed.
    ExecutionFailed {
        /// The key for the failed execution.
        key: Uuid,
        /// The reason for the failure.
        reason: String,
    },
    /// Serialization error.
    SerializationFailed {
        /// The reason for the failure.
        reason: String,
    },
    /// Deserialization error.
    DeserializationFailed {
        /// The reason for the failure.
        reason: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateExecution {
                key,
                cached_result_info,
            } => {
                if let Some(info) = cached_result_info {
                    write!(
                        f,
                        "duplicate execution detected for key {key}: returning cached result ({info})"
                    )
                } else {
                    write!(
                        f,
                        "duplicate execution detected for key {key}: returning cached result"
                    )
                }
            }
            Self::CacheLookupFailed { key, reason } => {
                write!(f, "cache lookup failed for key {key}: {reason}")
            }
            Self::DbLookupFailed { key, reason } => {
                write!(f, "database lookup failed for key {key}: {reason}")
            }
            Self::CacheStoreFailed { key, reason } => {
                write!(f, "failed to store result in cache for key {key}: {reason}")
            }
            Self::DbStoreFailed { key, reason } => {
                write!(
                    f,
                    "failed to store result in database for key {key}: {reason}"
                )
            }
            Self::ExecutionFailed { key, reason } => {
                write!(f, "execution failed for key {key}: {reason}")
            }
            Self::SerializationFailed { reason } => {
                write!(f, "serialization failed: {reason}")
            }
            Self::DeserializationFailed { reason } => {
                write!(f, "deserialization failed: {reason}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Create a duplicate execution error.
    pub fn duplicate_execution(key: Uuid, cached_result_info: Option<String>) -> Self {
        Self::DuplicateExecution {
            key,
            cached_result_info,
        }
    }

    /// Create a cache lookup failed error.
    pub fn cache_lookup_failed(key: Uuid, reason: impl Into<String>) -> Self {
        Self::CacheLookupFailed {
            key,
            reason: reason.into(),
        }
    }

    /// Create a database lookup failed error.
    pub fn db_lookup_failed(key: Uuid, reason: impl Into<String>) -> Self {
        Self::DbLookupFailed {
            key,
            reason: reason.into(),
        }
    }

    /// Create a cache store failed error.
    pub fn cache_store_failed(key: Uuid, reason: impl Into<String>) -> Self {
        Self::CacheStoreFailed {
            key,
            reason: reason.into(),
        }
    }

    /// Create a database store failed error.
    pub fn db_store_failed(key: Uuid, reason: impl Into<String>) -> Self {
        Self::DbStoreFailed {
            key,
            reason: reason.into(),
        }
    }

    /// Create an execution failed error.
    pub fn execution_failed(key: Uuid, reason: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            key,
            reason: reason.into(),
        }
    }

    /// Create a serialization failed error.
    pub fn serialization_failed(reason: impl Into<String>) -> Self {
        Self::SerializationFailed {
            reason: reason.into(),
        }
    }

    /// Create a deserialization failed error.
    pub fn deserialization_failed(reason: impl Into<String>) -> Self {
        Self::DeserializationFailed {
            reason: reason.into(),
        }
    }

    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::CacheLookupFailed { .. }
                | Self::DbLookupFailed { .. }
                | Self::CacheStoreFailed { .. }
                | Self::DbStoreFailed { .. }
        )
    }

    /// Check if this error indicates duplicate execution.
    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::DuplicateExecution { .. })
    }

    /// Get the idempotency key associated with this error.
    pub fn key(&self) -> Option<Uuid> {
        match self {
            Self::DuplicateExecution { key, .. }
            | Self::CacheLookupFailed { key, .. }
            | Self::DbLookupFailed { key, .. }
            | Self::CacheStoreFailed { key, .. }
            | Self::DbStoreFailed { key, .. }
            | Self::ExecutionFailed { key, .. } => Some(*key),
            Self::SerializationFailed { .. } | Self::DeserializationFailed { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duplicate_execution_error_display() {
        let key = Uuid::nil();
        let err = Error::duplicate_execution(key, Some("test result".to_string()));
        let display = err.to_string();
        assert!(display.contains("duplicate execution"));
        assert!(display.contains("test result"));
        assert!(display.contains(&key.to_string()));
    }

    #[test]
    fn test_duplicate_execution_error_display_no_info() {
        let key = Uuid::nil();
        let err = Error::duplicate_execution(key, None);
        let display = err.to_string();
        assert!(display.contains("duplicate execution"));
        assert!(display.contains(&key.to_string()));
    }

    #[test]
    fn test_is_duplicate() {
        let key = Uuid::nil();
        let err = Error::duplicate_execution(key, None);
        assert!(err.is_duplicate());

        let err = Error::execution_failed(key, "test");
        assert!(!err.is_duplicate());
    }

    #[test]
    fn test_is_retryable() {
        let key = Uuid::nil();

        assert!(Error::cache_lookup_failed(key, "test").is_retryable());
        assert!(Error::db_lookup_failed(key, "test").is_retryable());
        assert!(Error::cache_store_failed(key, "test").is_retryable());
        assert!(Error::db_store_failed(key, "test").is_retryable());

        assert!(!Error::duplicate_execution(key, None).is_retryable());
        assert!(!Error::execution_failed(key, "test").is_retryable());
        assert!(!Error::serialization_failed("test").is_retryable());
    }

    #[test]
    fn test_key_extraction() {
        let key = Uuid::nil();

        assert_eq!(Error::duplicate_execution(key, None).key(), Some(key));
        assert_eq!(Error::execution_failed(key, "test").key(), Some(key));
        assert_eq!(Error::serialization_failed("test").key(), None);
    }

    #[test]
    fn test_error_constructors() {
        let key = Uuid::nil();

        let err = Error::cache_lookup_failed(key, "cache miss");
        assert!(err.to_string().contains("cache lookup failed"));

        let err = Error::db_lookup_failed(key, "connection error");
        assert!(err.to_string().contains("database lookup failed"));

        let err = Error::cache_store_failed(key, "write lock");
        assert!(err.to_string().contains("failed to store result in cache"));

        let err = Error::db_store_failed(key, "transaction failed");
        assert!(err
            .to_string()
            .contains("failed to store result in database"));

        let err = Error::serialization_failed("bincode error");
        assert!(err.to_string().contains("serialization failed"));

        let err = Error::deserialization_failed("invalid format");
        assert!(err.to_string().contains("deserialization failed"));
    }
}
