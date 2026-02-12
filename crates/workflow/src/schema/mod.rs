//! Schema definitions and type mappings for SurrealDB tables.
//!
//! This module provides Rust types that map to SurrealDB schemas, with
//! strong typing and validation to prevent invalid states.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod isolation;
pub mod limits;
pub mod sticky;

pub use isolation::{
    IsolationError, Schedule, ScheduleConfig, Workspace, WorkspaceConfig, WorkspacePath,
    WorkspaceStatus,
};
pub use limits::{
    ConcurrencyLimit, ConcurrencyLimitConfig, RateLimitError, ResourceId, TokenBucket,
    TokenBucketConfig,
};
pub use sticky::{
    build_count_by_worker_query, build_create_assignment_query, build_delete_by_bead_query,
    build_find_all_query, build_find_by_bead_query, build_find_by_worker_query,
    build_update_worker_query, AssignmentId, BeadIdRef, StickyAssignment, StickyAssignmentError,
    WorkerIdRef,
};
