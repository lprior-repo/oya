//! # Merge Queue
//!
//! Parallel task merging and conflict resolution for OYA.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::panic)]
#![deny(clippy::expect_used)]

pub use oya_core::{Error, Result};

/// Queue management module
pub mod queue {}

/// Conflict resolution module
pub mod conflict {}
