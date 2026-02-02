//! Idempotency support for workflow execution.
//!
//! This module provides utilities for ensuring idempotent execution of workflows
//! using UUID v5 based namespacing and deterministic key generation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod namespace;

// Re-export main functions
pub use namespace::namespace_from_bead;
