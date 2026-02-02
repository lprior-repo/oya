//! Schema definitions and type mappings for SurrealDB tables.
//!
//! This module provides Rust types that map to SurrealDB schemas, with
//! strong typing and validation to prevent invalid states.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod limits;

// Re-export commonly used types
pub use limits::{
    ConcurrencyLimit, ConcurrencyLimitConfig, RateLimitError, ResourceId, TokenBucket,
    TokenBucketConfig,
};
