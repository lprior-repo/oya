#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

//! Error types for BeadStore.
//!
//! Uses thiserror for semantic domain errors that can be handled
//! by callers.

use crate::types::BeadId;
use std::io;
use thiserror::Error;

/// Errors that can occur in BeadStore operations.
#[derive(Debug, Error)]
pub enum StoreError {
    /// Bead not found in store.
    #[error("bead not found: {0}")]
    NotFound(BeadId),

    /// I/O error during read/write operations.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Lock was poisoned (concurrent access failed).
    #[error("lock poisoned")]
    LockPoisoned,

    /// Invalid bead data (validation failed).
    #[error("invalid bead data: {0}")]
    InvalidData(String),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_error_display() {
        let err = StoreError::NotFound(BeadId::new("test-123"));
        assert_eq!(err.to_string(), "bead not found: test-123");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let store_err = StoreError::from(io_err);
        assert!(store_err.to_string().contains("I/O error"));
        assert!(store_err.to_string().contains("file not found"));
    }
}
