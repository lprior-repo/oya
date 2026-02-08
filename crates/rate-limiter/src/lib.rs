//! # Rate Limiter
//!
//! Token bucket rate limiting algorithm implementation.
//!
//! ## Algorithm
//!
//! The token bucket algorithm works as follows:
//! - Tokens are added to the bucket at a constant rate
//! - The bucket has a maximum capacity
//! - Each request consumes one or more tokens
//! - If insufficient tokens are available, the request is rate-limited
//!
//! ## Invariants
//!
//! - Token count never exceeds capacity
//! - Token count never goes negative
//! - Refill rate is constant and deterministic
//!
//! ## Zero-Panic Guarantee
//!
//! This crate is implemented with zero panics, zero unwraps, and purely functional
//! patterns throughout. All fallible operations return `Result<T, Error>`.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use oya_core::{Error, Result};

mod token_bucket;

pub use token_bucket::{TokenBucket, AcquireResult, BucketConfig};

/// Rate limiter error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitError {
    /// Invalid configuration parameter
    InvalidConfig(String),

    /// Insufficient tokens available
    InsufficientTokens {
        /// Available tokens
        available: u64,
        /// Requested tokens
        requested: u64,
    },

    /// Bucket has been closed
    Closed,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
            Self::InsufficientTokens { available, requested } => {
                write!(
                    f,
                    "Insufficient tokens: available={}, requested={}",
                    available, requested
                )
            }
            Self::Closed => write!(f, "Bucket has been closed"),
        }
    }
}

impl std::error::Error for RateLimitError {}

/// Convert RateLimitError to oya_core::Error
impl From<RateLimitError> for Error {
    fn from(err: RateLimitError) -> Self {
        Error::invalid_record(&err.to_string())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::expect_used)]
    #![allow(clippy::panic)]

    use super::*;

    #[test]
    fn test_error_display() {
        let err = RateLimitError::InvalidConfig("capacity is zero".to_string());
        assert_eq!(err.to_string(), "Invalid configuration: capacity is zero");
    }

    #[test]
    fn test_error_insufficient_tokens() {
        let err = RateLimitError::InsufficientTokens {
            available: 5,
            requested: 10,
        };
        assert_eq!(
            err.to_string(),
            "Insufficient tokens: available=5, requested=10"
        );
    }

    #[test]
    fn test_error_closed() {
        let err = RateLimitError::Closed;
        assert_eq!(err.to_string(), "Bucket has been closed");
    }
}
