//! Schema definitions and type mappings for SurrealDB tables.
//!
//! This module provides Rust types that map to SurrealDB schemas, with
//! strong typing and validation to prevent invalid states.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod isolation;
pub mod limits;

// Re-export commonly used types
pub use isolation::{
    IsolationError, Schedule, ScheduleConfig, Workspace, WorkspaceConfig, WorkspacePath,
    WorkspaceStatus,
};
pub use limits::{
    ConcurrencyLimit, ConcurrencyLimitConfig, RateLimitError, ResourceId, TokenBucket,
    TokenBucketConfig,
};
